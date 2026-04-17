//! Minimal API definitions for node-bridge
//! 这些类型将作为 openclaw-rs 的一部分导出

use serde::{Deserialize, Serialize};

/// Session ID（字符串包装器）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: String, // ISO 8601
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_tokens: Option<usize>,
    pub compaction_threshold: f32,
    pub sandbox_enabled: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_tokens: Some(100_000),
            compaction_threshold: 0.8,
            sandbox_enabled: true,
        }
    }
}

/// Session error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionError {
    pub message: String,
}

impl From<SessionError> for String {
    fn from(e: SessionError) -> Self {
        e.message
    }
}

impl SessionError {
    pub fn new(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
}

/// Runtime error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeError {
    SessionNotFound(String),
    SessionError(SessionError),
    General(String),
}

impl RuntimeError {
    pub fn session_not_found(id: &str) -> Self {
        Self::SessionNotFound(id.to_string())
    }

    pub fn session_error(msg: &str) -> Self {
        Self::SessionError(SessionError::new(msg))
    }

    pub fn general(msg: &str) -> Self {
        Self::General(msg.to_string())
    }
}

// 为了方便 FFI 传递，定义错误 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDTO {
    pub message: String,
}