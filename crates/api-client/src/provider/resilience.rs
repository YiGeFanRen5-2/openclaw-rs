//! # Resilience traits
//!
//! Retry logic, rate limiting, and circuit breaker for providers.

use crate::provider::trait_def::{Provider, ProviderError, ProviderStream};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter using token bucket algorithm.
///
/// Controls request rate to avoid hitting provider limits.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Minimum interval between requests.
    interval: Duration,
    /// Timestamp of last request.
    last_request: Arc<Mutex<Option<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with a minimum interval between requests.
    pub fn new(requests_per_second: f64) -> Self {
        let interval = Duration::from_secs_f64(1.0 / requests_per_second.max(0.1));
        Self {
            interval,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a rate limiter for a specific provider.
    ///
    /// Default rate limits (can be overridden by config):
    /// - OpenAI: 60 req/min for many models
    /// - Anthropic: 50 req/min for claude-3-5-sonnet
    pub fn for_provider(provider: &str) -> Self {
        match provider {
            "openai" => Self::new(1.0),    // ~1 req/s
            "anthropic" => Self::new(0.8), // ~50 req/min
            _ => Self::new(10.0),          // relaxed default
        }
    }

    /// Wait until the next request is allowed.
    pub async fn acquire(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            if elapsed < self.interval {
                tokio::time::sleep(self.interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }
}

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial delay on first retry.
    pub initial_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
    /// HTTP status codes that should trigger a retry.
    pub retry_on_status: &'static [u16],
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retry_on_status: &[429, 500, 502, 503, 504],
        }
    }
}

/// Calculate the delay for a given retry attempt.
pub fn retry_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let delay = config
        .initial_delay
        .mul_f64(config.backoff_multiplier.powi(attempt as i32));
    delay.min(config.max_delay)
}

/// Check if a status code should trigger a retry.
pub fn is_retryable(status: u16, config: &RetryConfig) -> bool {
    config.retry_on_status.contains(&status)
}

/// Wrapper provider that adds retry logic.
///
/// Wraps any `Provider` and automatically retries failed requests
/// with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryProvider<P> {
    inner: P,
    config: RetryConfig,
}

impl<P: Provider> RetryProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            config: RetryConfig::default(),
        }
    }

    pub fn with_config(inner: P, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    fn should_retry(&self, err: &ProviderError) -> bool {
        match err {
            ProviderError::Network(_) => true,
            ProviderError::Provider(msg) => {
                // Heuristic: rate limit or server error
                msg.contains("429")
                    || msg.contains("500")
                    || msg.contains("502")
                    || msg.contains("503")
                    || msg.contains("rate limit")
                    || msg.contains("too many requests")
            }
            ProviderError::Config(_) => false,
            ProviderError::StreamingNotSupported => false,
            ProviderError::ContextLengthExceeded(_, _) => false,
        }
    }
}

#[async_trait::async_trait]
impl<P: Provider> Provider for RetryProvider<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn capabilities(&self) -> crate::provider::trait_def::ProviderCapabilities {
        self.inner.capabilities()
    }

    fn validate_context(&self, prompt: &str) -> Result<(), ProviderError> {
        self.inner.validate_context(prompt)
    }

    async fn generate(&self, prompt: &str) -> Result<String, ProviderError> {
        let mut last_err = None;

        for attempt in 0..self.config.max_attempts {
            match self.inner.generate(prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if self.should_retry(&e) && attempt < self.config.max_attempts - 1 {
                        last_err = Some(e);
                        let delay = retry_delay(attempt, &self.config);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ProviderError::Provider("max retries exceeded".into())))
    }

    async fn stream(&self, prompt: &str) -> Result<ProviderStream, ProviderError> {
        self.inner.stream(prompt).await
    }
}

/// Resilient provider wrapper: rate limiting + retry around any provider.
pub struct ResilientProvider {
    inner: Box<dyn Provider + Send + Sync>,
    limiter: Arc<RateLimiter>,
    retry_config: RetryConfig,
}

impl ResilientProvider {
    pub fn new(inner: Box<dyn Provider + Send + Sync>) -> Self {
        let name = inner.name().to_string();
        Self {
            inner,
            limiter: Arc::new(RateLimiter::for_provider(&name)),
            retry_config: RetryConfig::default(),
        }
    }

    fn should_retry(&self, err: &ProviderError) -> bool {
        match err {
            ProviderError::Network(_) => true,
            ProviderError::Provider(msg) => {
                msg.contains("429")
                    || msg.contains("500")
                    || msg.contains("502")
                    || msg.contains("503")
                    || msg.contains("rate limit")
                    || msg.contains("too many requests")
            }
            ProviderError::Config(_) => false,
            ProviderError::StreamingNotSupported => false,
            ProviderError::ContextLengthExceeded(_, _) => false,
        }
    }
}

#[async_trait::async_trait]
impl Provider for ResilientProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn capabilities(&self) -> crate::provider::trait_def::ProviderCapabilities {
        self.inner.capabilities()
    }

    fn validate_context(&self, prompt: &str) -> Result<(), ProviderError> {
        self.inner.validate_context(prompt)
    }

    async fn generate(&self, prompt: &str) -> Result<String, ProviderError> {
        self.limiter.acquire().await;
        let mut last_err = None;

        for attempt in 0..self.retry_config.max_attempts {
            match self.inner.generate(prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if self.should_retry(&e) && attempt < self.retry_config.max_attempts - 1 {
                        last_err = Some(e);
                        let delay = retry_delay(attempt, &self.retry_config);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| ProviderError::Provider("max retries exceeded".into())))
    }

    async fn stream(&self, prompt: &str) -> Result<ProviderStream, ProviderError> {
        self.limiter.acquire().await;
        self.inner.stream(prompt).await
    }
}

/// Wrapping helper: retry + rate-limit around any provider box.
pub async fn create_resilient(
    inner: Box<dyn Provider + Send + Sync>,
) -> Result<Box<dyn Provider + Send + Sync>, ProviderError> {
    Ok(Box::new(ResilientProvider::new(inner)))
}
