use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookStage {
    BeforeTool,
    AfterTool,
    BeforeModel,
    AfterModel,
    BeforePromptRender,
    AfterPromptRender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub stage: HookStage,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHookPayload {
    pub template: String,
    pub rendered: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHookPayload {
    pub tool_name: String,
    pub call: Value,
    pub output: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHookPayload {
    pub model: String,
    pub prompt: String,
    pub output: Option<String>,
}

#[async_trait]
pub trait PromptHook: Send + Sync {
    async fn before_render(&self, payload: PromptHookPayload) -> PromptHookPayload {
        payload
    }

    async fn after_render(&self, payload: PromptHookPayload) -> PromptHookPayload {
        payload
    }
}

#[async_trait]
pub trait ToolHook: Send + Sync {
    async fn before_tool(&self, payload: ToolHookPayload) -> ToolHookPayload {
        payload
    }

    async fn after_tool(&self, payload: ToolHookPayload) -> ToolHookPayload {
        payload
    }
}

#[async_trait]
pub trait ModelHook: Send + Sync {
    async fn before_model(&self, payload: ModelHookPayload) -> ModelHookPayload {
        payload
    }

    async fn after_model(&self, payload: ModelHookPayload) -> ModelHookPayload {
        payload
    }
}
