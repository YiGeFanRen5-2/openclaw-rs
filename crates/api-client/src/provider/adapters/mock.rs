//! Mock provider for testing and development.

use crate::provider::trait_def::{Provider, ProviderCapabilities, ProviderError};
use async_trait::async_trait;

/// Mock provider for testing and development.
///
/// Responses are deterministic based on the prompt.
#[derive(Debug, Clone)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
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
        Ok(format!("[MOCK] Response to: {}", prompt.trim_end()))
    }

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>,
        ProviderError,
    > {
        use futures_util::stream;
        let words_vec: Vec<Result<String, ProviderError>> =
            format!("[MOCK] Streamed response to: {}", prompt.trim_end())
                .split_whitespace()
                .map(|s| Ok(s.to_string() + " "))
                .collect();
        Ok(Box::pin(stream::iter(words_vec)))
    }
}
