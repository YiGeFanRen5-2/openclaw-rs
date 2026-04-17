/// Provider configuration used by adapter factory.
///
/// This is a simplified config that can be extended for specific providers.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider kind (e.g., "mock", "openai")
    pub kind: String,

    /// API key (if required)
    pub api_key: Option<String>,

    /// Base URL (if non-standard)
    pub base_url: Option<String>,

    /// Model identifier
    pub model: Option<String>,

    /// Additional parameters (temperature, top_p, etc.)
    pub extra: std::collections::HashMap<String, String>,
}

impl ProviderConfig {
    /// Create a new provider config with required fields.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            api_key: None,
            base_url: None,
            model: None,
            extra: Default::default(),
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

    /// Set the model name.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add an extra parameter.
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::new("mock")
    }
}
