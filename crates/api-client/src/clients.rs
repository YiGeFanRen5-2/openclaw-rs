use async_trait::async_trait;
use crate::{provider::{Provider, ProviderConfig}, models::*, error::{Error, Result}};
use reqwest::{Client, header};

pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap();

        Self { config, client }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key)).map_err(|e| Error::Configuration(e.to_string()))?
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json")
        );

        let payload = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "stream": false,
        });

        let response = self.client
            .post(&format!("{}/v1/chat/completions", self.config.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!("HTTP {}: {}", status, text)));
        }

        let resp_json: serde_json::Value = response.json().await
            .map_err(|e| Error::Http(e.to_string()))?;

        let message = ChatMessage {
            role: resp_json["choices"][0]["message"]["role"].as_str().unwrap_or("assistant").to_string(),
            content: resp_json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string(),
        };

        let usage = resp_json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            message,
            usage,
            model: request.model,
        })
    }

    async fn stream(&self, request: ChatRequest) -> Result<tokio::sync::mpsc::Receiver<Result<StreamChunk>>> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key)).map_err(|e| Error::Configuration(e.to_string()))?
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json")
        );

        let payload = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "stream": true,
        });

        let response = self.client
            .post(&format!("{}/v1/chat/completions", self.config.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!("HTTP {}: {}", status, text)));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let bytes = response.bytes().await
            .map_err(|e| Error::Http(e.to_string()))?;

        // Simple SSE parsing (simplified for MVP)
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    let delta = json["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                    let chunk = StreamChunk {
                        delta: ChatMessage { role: "assistant".to_string(), content: delta },
                        finish_reason: json["choices"][0]["finish_reason"].as_str().map(|s| s.to_string()),
                        usage: None,
                    };
                    if tx.send(Ok(chunk)).await.is_err() {
                        break;
                    }
                }
            }
        }

        Ok(rx)
    }
}
