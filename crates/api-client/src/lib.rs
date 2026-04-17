//! # API Client Crate
//! 模型提供商抽象层，支持 Anthropic、OpenAI、xAI 等
//!
//! 当前状态：最小化 stub，Phase 1 PoC 完成后逐步填充

pub mod models;
pub mod provider;

pub use models::{ChatMessage, ChatResponse};
pub use provider::{Provider, ProviderCapabilities, ProviderError};
