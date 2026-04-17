//! Tool execution engine (MVP).

use async_trait::async_trait;
use serde_json::json;

use crate::permissions::{PermissionResult, PermissionSet};
use crate::tool::{ToolCall, ToolOutput, ToolSpec};

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub permissions: PermissionSet,
}

impl ExecutionContext {
    pub fn new(permissions: PermissionSet) -> Self {
        Self { permissions }
    }
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn invoke(&self, call: ToolCall) -> PermissionResult<ToolOutput>;
}

#[derive(Debug, Default)]
pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute<H: ToolHandler>(
        &self,
        context: &ExecutionContext,
        spec: &ToolSpec,
        handler: &H,
        call: ToolCall,
    ) -> PermissionResult<ToolOutput> {
        for permission in &spec.required_permissions {
            context.permissions.check(permission)?;
        }

        handler.invoke(call).await
    }
}

pub struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn invoke(&self, call: ToolCall) -> PermissionResult<ToolOutput> {
        Ok(ToolOutput::json(json!({
            "ok": true,
            "echo": call.arguments,
        })))
    }
}
