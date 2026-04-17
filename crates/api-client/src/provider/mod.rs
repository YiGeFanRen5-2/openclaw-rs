//! # Provider module
//! Provider trait and adapters

pub mod adapters;
pub mod config;
pub mod resilience;
pub mod trait_def;

pub use config::ProviderConfig;
pub use resilience::{RateLimiter, RetryConfig, RetryProvider};
pub use trait_def::{Provider, ProviderCapabilities, ProviderError, ProviderStream};

use self::trait_def::ProviderError as TraitError;

/// Create a provider instance from configuration (synchronous).
///
/// Supported kinds: "openai", "anthropic", "mock"
pub fn create_provider(
    config: &ProviderConfig,
) -> Result<Box<dyn Provider + Send + Sync>, ProviderError> {
    let kind = config.kind.as_deref().unwrap_or("mock");

    match kind {
        "openai" => {
            let handle = tokio::runtime::Handle::current();
            let p = handle
                .block_on(adapters::OpenAIProvider::new(config))
                .map_err(|e| TraitError::Config(e.to_string()))?;
            Ok(Box::new(p))
        }
        "mock" => Ok(Box::new(adapters::MockProvider::new())),
        "anthropic" => {
            let handle = tokio::runtime::Handle::current();
            let p = handle
                .block_on(adapters::AnthropicProvider::new(config))
                .map_err(|e| TraitError::Config(e.to_string()))?;
            Ok(Box::new(p))
        }
        _ => Err(TraitError::Config(format!(
            "unknown provider kind: {}",
            kind
        ))),
    }
}
