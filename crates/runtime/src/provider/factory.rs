use crate::provider::config::ProviderConfig;
use crate::provider::trait_def::{Provider, ProviderError};

/// Factory for creating provider instances based on configuration.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider instance from configuration.
    ///
    /// Supported providers: "openai", "anthropic", "mock"
    pub async fn create(config: &ProviderConfig) -> Result<Box<dyn Provider + Send + Sync>, ProviderError> {
        match config.provider.as_str() {
            "openai" => {
                let provider = super::adapters::openai::OpenAIProvider::new(config).await?;
                Ok(Box::new(provider))
            }
            "anthropic" => {
                let provider = super::adapters::anthropic::AnthropicProvider::new(config).await?;
                Ok(Box::new(provider))
            }
            "mock" => {
                let provider = super::adapters::mock::create_provider(config).await?;
                Ok(Box::new(provider))
            }
            provider => Err(ProviderError::Config(format!(
                "Unknown provider: {}. Supported: openai, anthropic, mock",
                provider
            ))),
        }
    }
}

/// Provider configuration extension with provider type field.
#[derive(Debug, Clone)]
pub struct ExtendedProviderConfig {
    pub base: ProviderConfig,
    /// Provider type: "openai", "anthropic", "mock"
    pub provider: String,
}

impl ExtendedProviderConfig {
    pub fn new(provider: String, base: ProviderConfig) -> Self {
        Self { base, provider }
    }
}