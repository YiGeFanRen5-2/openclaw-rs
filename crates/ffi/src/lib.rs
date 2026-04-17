//! # OpenClaw FFI Crate (Phase 2 - Updated)
//! 使用 napi-rs 提供 Rust 运行时接口给 Node.js

use napi_derive::napi;

use runtime::{Runtime, RuntimeConfig, SessionId};
use tools::ToolCall;

/// OpenClaw 运行时
#[napi]
pub struct OpenClawRuntime {
    inner: Runtime,
}

#[napi]
impl OpenClawRuntime {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self { inner: runtime })
    }

    #[napi]
    pub fn create_session(&mut self, id: Option<String>) -> napi::Result<SessionHandle> {
        let session_id = id.unwrap_or_else(|| format!("session-{}", uuid::Uuid::new_v4()));
        let session_id = self
            .inner
            .create_session(session_id)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(SessionHandle::new(session_id))
    }

    #[napi]
    pub fn list_sessions(&self) -> napi::Result<Vec<String>> {
        let ids = self.inner.list_sessions();
        Ok(ids.into_iter().map(|sid| sid.0).collect())
    }

    #[napi]
    pub fn register_tool(&mut self, _name: String, _schema: String) -> napi::Result<()> {
        // Phase 3 实现
        Ok(())
    }

    /// 执行工具，arguments 需为 JSON 字符串
    #[napi]
    pub fn execute_tool(
        &mut self,
        session_id: String,
        name: String,
        arguments: String,
    ) -> napi::Result<String> {
        let sid = SessionId(session_id);
        let call = ToolCall { name, arguments };
        let result = self
            .inner
            .execute_tool(&sid, call)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(result.content)
    }
}

/// 会话句柄（只持有 ID，具体操作通过 Runtime）
#[napi]
pub struct SessionHandle {
    inner: SessionId,
}

#[napi]
impl SessionHandle {
    pub fn new(inner: SessionId) -> Self {
        Self { inner }
    }

    #[napi]
    pub fn id(&self) -> String {
        self.inner.0.clone()
    }

    #[napi]
    pub fn token_count(&self) -> i64 {
        0
    }
}
