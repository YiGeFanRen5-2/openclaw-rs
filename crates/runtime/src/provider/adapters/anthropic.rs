use crate::provider::trait_def::{Provider, ProviderCapabilities, ProviderError};
use crate::provider::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::{Client, header};
use std::time::Duration;

/// Anthropic Claude API provider.
///
/// Supports Messages API (non-legacy). Uses Claude 3 models by default.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub async fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config.api_key.clone().ok_or_else(|| {
            ProviderError::Config("Anthropic requires api_key".into())
        })?;

        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".into());

        let model = config.model.clone()
            .unwrap_or_else(|| "claude-3-opus-20240229".into());

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
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
            supports_functions: false, // tool/function calling not yet implemented
            max_context_length: 200000, // Claude 3 context window
        }
    }

    async fn generate(&self, prompt: &str) -> Result<String, ProviderError> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(&self.api_key)
                .map_err(|e| ProviderError::Config(e.to_string()))?
        );
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01")
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json")
        );

        #[derive(serde::Serialize)]
        struct MessageRequest {
            model: String,
            max_tokens: u32,
            messages: Vec<serde_json::Value>,
            stream: bool,
        }

        let body = MessageRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
            stream: false,
        };

        let url = format!("{}/messages", self.base_url);
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!("HTTP {}: {}", status, text)));
        }

        #[derive(serde::Deserialize)]
        struct MessageResponse {
            content: Vec<serde_json::Value>,
        }

        let resp_json: MessageResponse = response.json().await
            .map_err(|e| ProviderError::Provider(format!("JSON decode error: {}", e)))?;

        let content = resp_json.content
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
    ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>, ProviderError> {
        use futures_util::stream;
        use tokio_stream::wrappers::ReceiverStream;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(&self.api_key)
                .map_err(|e| ProviderError::Config(e.to_string()))?
        );
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01")
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json")
        );

        #[derive(serde::Serialize)]
        struct MessageRequest {
            model: String,
            max_tokens: u32,
            messages: Vec<serde_json::Value>,
            stream: bool,
        }

        let body = MessageRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
            stream: true,
        };

        let url = format!("{}/messages", self.base_url);
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!("HTTP {}: {}", status, text)));
        }

        let bytes = response.bytes().await
            .map_err(|e| ProviderError::Network(e))?;

        let text = String::from_utf8_lossy(&bytes);
        let mut chunks = Vec::new();

        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    // Anthropic stream format: {"type":"content_block_delta","delta":{"text":"..."}}
                    if let (Some(type_val), Some(delta)) = (json.get("type"), json.get("delta")) {
                        if type_val.as_str() == Some("content_block_delta") {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                if !text.is_empty() {
                                    chunks.push(Ok(text.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Box::pin(stream::iter(chunks)))
    }
}
