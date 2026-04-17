use async_trait::async_trait;
use std::pin::Pin;

/// Provider capabilities flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub max_context_length: usize,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            supports_streaming: false,
            supports_functions: false,
            max_context_length: 4096,
        }
    }
}

/// Provider generation error
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("provider returned error: {0}")]
    Provider(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("streaming not supported")]
    StreamingNotSupported,

    #[error("context length exceeded: {0} > {1}")]
    ContextLengthExceeded(usize, usize),
}

/// Trait defining AI provider interface.
///
/// Implementors must be thread-safe (Send + Sync) and can be used
/// as a trait object (Box<dyn Provider>).
#[async_trait]
pub trait Provider: Send + Sync {
    /// Generate a complete response for the given prompt.
    async fn generate(&self, prompt: &str) -> Result<String, ProviderError>;

    /// Stream a response token by token.
    ///
    /// Default implementation returns error if streaming not supported.
    async fn stream(
        &self,
        _prompt: &str,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>, ProviderError> {
        Err(ProviderError::StreamingNotSupported)
    }

    /// Human readable provider name.
    fn name(&self) -> &str;

    /// Capabilities of this provider.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Check if prompt fits within context window.
    fn validate_context(&self, prompt: &str) -> Result<(), ProviderError> {
        let len = prompt.chars().count(); // rough estimate
        let max = self.capabilities().max_context_length;
        if len > max {
            Err(ProviderError::ContextLengthExceeded(len, max))
        } else {
            Ok(())
        }
    }
}

/// Stream helper type
pub type ProviderStream = Pin<Box<dyn futures_util::Stream<Item = Result<String, ProviderError>> + Send>>;
