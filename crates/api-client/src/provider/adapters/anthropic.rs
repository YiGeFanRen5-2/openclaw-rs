//! Anthropic API provider (Claude).
//!
//! Supports claude-3-5-sonnet, claude-3-opus, etc.
//! API: https://docs.anthropic.com/en/api/reference

use crate::provider::config::ProviderConfig;
use crate::provider::trait_def::{Provider, ProviderCapabilities, ProviderError};
use async_trait::async_trait;
use reqwest::{header, Client};
use std::time::Duration;

/// Anthropic API provider.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub async fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| ProviderError::Config("Anthropic requires api_key".into()))?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".into());

        let model = config
            .model
            .clone()
            .unwrap_or_else(|| "claude-3-5-sonnet-20241022".into());

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::Config(e.to_string()))?;

        Ok(Self {
            api_key,
            base_url,
            model,
            client,
        })
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: true,
            max_context_length: 200_000,
        }
    }

    async fn generate(&self, prompt: &str) -> Result<String, ProviderError> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| ProviderError::Config(e.to_string()))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(&self.api_key)
                .map_err(|e| ProviderError::Config(e.to_string()))?,
        );
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01"),
        );

        #[derive(serde::Serialize)]
        struct MessagesRequest<'a> {
            model: &'a str,
            max_tokens: u32,
            messages: Vec<serde_json::Value>,
        }

        let body = MessagesRequest {
            model: &self.model,
            max_tokens: 1024,
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
        };

        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        #[derive(serde::Deserialize)]
        struct MessagesResponse {
            content: Vec<serde_json::Value>,
        }

        let resp_json: MessagesResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("JSON decode error: {}", e)))?;

        let content = resp_json
            .content
            .first()
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ProviderError::Provider("unexpected response format".into()))?;

        Ok(content)
    }

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>,
        ProviderError,
    > {
        use futures_util::stream;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| ProviderError::Config(e.to_string()))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(&self.api_key)
                .map_err(|e| ProviderError::Config(e.to_string()))?,
        );
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01"),
        );

        #[derive(serde::Serialize)]
        struct MessagesRequest<'a> {
            model: &'a str,
            max_tokens: u32,
            messages: Vec<serde_json::Value>,
            stream: bool,
        }

        let body = MessagesRequest {
            model: &self.model,
            max_tokens: 1024,
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
            stream: true,
        };

        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let bytes = response.bytes().await.map_err(ProviderError::Network)?;

        let text = String::from_utf8_lossy(&bytes);
        let mut chunks = Vec::new();

        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(delta) = json["content_block_delta"]["text"].as_str() {
                        chunks.push(Ok(delta.to_string()));
                    }
                }
            }
        }

        Ok(Box::pin(stream::iter(chunks)))
    }
}
