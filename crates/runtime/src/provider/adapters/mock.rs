use crate::provider::trait_def::{Provider, ProviderCapabilities, ProviderError};
use crate::provider::config::ProviderConfig;
use async_trait::async_trait;
// use std::pin::Pin; // Removed as unused

/// Mock provider for testing and development.
///
/// Responses are deterministic based on the prompt.
#[derive(Debug, Clone)]
pub struct MockProvider {
    pub delay_ms: Option<u64>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self { delay_ms: None }
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: false,
            max_context_length: 8192,
        }
    }

    async fn generate(&self, prompt: &str) -> Result<String, ProviderError> {
        if let Some(ms) = self.delay_ms {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }
        Ok(format!("[MOCK] Response to: {}", prompt.trim_end()))
    }

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>, ProviderError>
    {
        use futures_util::stream;
        let words_vec: Vec<Result<String, ProviderError>> = format!("[MOCK] Streamed response to: {}", prompt.trim_end())
            .split_whitespace()
            .map(|s| Ok(s.to_string() + " "))
            .collect();
        Ok(Box::pin(stream::iter(words_vec)))
    }
}

/// Create a provider from config.
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn Provider>, ProviderError> {
    match config.kind.as_str() {
        "mock" => Ok(Box::new(MockProvider::new())),
        "openai" => {
            if config.api_key.is_none() {
                return Err(ProviderError::Config("OpenAI requires api_key".into()));
            }
            openai::create_provider(config)
        }
        _ => Err(ProviderError::Config(format!("unknown provider kind: {}", config.kind))),
    }
}
