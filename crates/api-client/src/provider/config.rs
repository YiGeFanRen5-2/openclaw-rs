//! Provider configuration

use serde::{Deserialize, Serialize};

/// Provider configuration for creating provider instances.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Base URL of the provider API.
    #[serde(default)]
    pub base_url: Option<String>,

    /// API key for authentication.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Default model to use.
    #[serde(default)]
    pub model: Option<String>,

    /// Provider type: "openai", "anthropic", "mock", etc.
    #[serde(default)]
    pub kind: Option<String>,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Organization ID (for OpenAI).
    #[serde(default)]
    pub organization: Option<String>,

    /// Custom headers to include in requests.
    #[serde(default)]
    pub extra_headers: std::collections::HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30
}

impl ProviderConfig {
    /// Create a new config with the given provider kind.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: Some(kind.into()),
            ..Default::default()
        }
    }

    /// Set the API key.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            api_key: None,
            model: None,
            kind: None,
            timeout_seconds: default_timeout(),
            organization: None,
            extra_headers: std::collections::HashMap::new(),
        }
    }
}
