use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::hook::{
    ModelHook, ModelHookPayload, PromptHook, PromptHookPayload, ToolHook, ToolHookPayload,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
}

#[async_trait]
pub trait RuntimePlugin: Plugin + PromptHook + ToolHook + ModelHook {
    async fn transform_prompt_template(&self, template: String) -> String {
        self.before_render(PromptHookPayload {
            template,
            rendered: None,
        })
        .await
        .template
    }

    async fn transform_rendered_prompt(&self, template: String, rendered: String) -> String {
        self.after_render(PromptHookPayload {
            template,
            rendered: Some(rendered),
        })
        .await
        .rendered
        .unwrap_or_default()
    }

    async fn transform_tool_call(&self, tool_name: String, call: serde_json::Value) -> ToolHookPayload {
        self.before_tool(ToolHookPayload {
            tool_name,
            call,
            output: None,
        })
        .await
    }

    async fn transform_tool_result(
        &self,
        tool_name: String,
        call: serde_json::Value,
        output: serde_json::Value,
    ) -> ToolHookPayload {
        self.after_tool(ToolHookPayload {
            tool_name,
            call,
            output: Some(output),
        })
        .await
    }

    async fn transform_model_request(&self, model: String, prompt: String) -> ModelHookPayload {
        self.before_model(ModelHookPayload {
            model,
            prompt,
            output: None,
        })
        .await
    }

    async fn transform_model_result(&self, model: String, prompt: String, output: String) -> ModelHookPayload {
        self.after_model(ModelHookPayload {
            model,
            prompt,
            output: Some(output),
        })
        .await
    }
}
