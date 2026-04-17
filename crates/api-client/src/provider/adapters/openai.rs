use crate::provider::config::ProviderConfig;
use crate::provider::trait_def::{Provider, ProviderCapabilities, ProviderError};
use async_trait::async_trait;
use reqwest::{header, Client};
use std::time::Duration;

/// OpenAI API provider with real HTTP calls.
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: Client,
}

impl OpenAIProvider {
    pub async fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| ProviderError::Config("OpenAI requires api_key".into()))?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".into());

        let model = config.model.clone().unwrap_or_else(|| "gpt-4o".into());

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
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: true,
            max_context_length: 128000,
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

        #[derive(serde::Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<serde_json::Value>,
            temperature: f32,
            max_tokens: Option<u32>,
            stream: bool,
        }

        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
            temperature: 0.7,
            max_tokens: Some(1024),
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);
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
        struct ChatResponse {
            choices: Vec<serde_json::Value>,
        }

        let resp_json: ChatResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("JSON decode error: {}", e)))?;

        let content = resp_json
            .choices
            .first()
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
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

        #[derive(serde::Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<serde_json::Value>,
            temperature: f32,
            max_tokens: Option<u32>,
            stream: bool,
        }

        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![serde_json::json!({
                "role": "user",
                "content": prompt,
            })],
            temperature: 0.7,
            max_tokens: Some(1024),
            stream: true,
        };

        let url = format!("{}/chat/completions", self.base_url);
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
                    let delta = json["choices"][0]["delta"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    if !delta.is_empty() {
                        chunks.push(Ok(delta));
                    }
                }
            }
        }

        Ok(Box::pin(stream::iter(chunks)))
    }
}
