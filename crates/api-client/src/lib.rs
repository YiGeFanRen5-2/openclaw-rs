//! # API Client Crate
//! 模型提供商抽象层，支持 Anthropic、OpenAI、xAI 等
//!
//! 当前状态：最小化 stub，Phase 1 PoC 完成后逐步填充
//!
//! # Stability
//!
//! This crate is marked as **stable**. Public API follows semantic versioning.
//! See [`docs/api/SEMVER.md`](https://github.com/openclaw/openclaw-rs/blob/master/docs/api/SEMVER.md)
//! for details on stability guarantees.

pub mod models;
pub mod provider;

pub use models::{ChatMessage, ChatResponse};
pub use provider::{Provider, ProviderCapabilities, ProviderError};

/// API version string (semver compatible)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the API version as a string.
///
/// # Example
/// ```
/// use openclaw_api_client::api_version;
/// println!("API version: {}", api_version());
/// ```
pub fn api_version() -> &'static str {
    VERSION
}

/// API version components (major, minor, patch)
///
/// # Example
/// ```
/// use openclaw_api_client::api_version_info;
/// let (major, minor, patch) = api_version_info();
/// println!("v{}.{}.{}", major, minor, patch);
/// ```
pub fn api_version_info() -> (u8, u8, u8) {
    let version = env!("CARGO_PKG_VERSION");
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.split('-').next().and_then(|s| s.parse().ok())).unwrap_or(0);
    (major, minor, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_format() {
        let v = VERSION;
        assert!(v.contains('.'), "version should be semver format");
    }

    #[test]
    fn test_api_version() {
        assert_eq!(api_version(), VERSION);
    }

    #[test]
    fn test_api_version_info() {
        let (major, minor, patch) = api_version_info();
        assert!(major >= 0);
        assert!(minor >= 0);
        assert!(patch >= 0);
    }
}
