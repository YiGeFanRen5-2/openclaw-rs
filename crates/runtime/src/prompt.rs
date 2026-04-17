use std::sync::Arc;

use openclaw_plugin::RuntimePlugin;
use openclaw_tools::ToolOutput;
use serde::{Deserialize, Serialize};

use crate::{orchestration::{OrchestrationPlan, OrchestrationResult}, session::Session};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBundle {
    pub system: String,
    pub user: String,
    pub context: Vec<String>,
}

impl PromptBundle {
    pub fn new(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            user: user.into(),
            context: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptContext<'a> {
    pub session: &'a Session,
    pub plan: &'a OrchestrationPlan,
    pub last_tool_output: Option<&'a ToolOutput>,
    pub last_model_output: Option<&'a str>,
}

#[derive(Default)]
pub struct PromptRenderer {
    plugins: Vec<Arc<dyn RuntimePlugin>>,
}

impl PromptRenderer {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn with_plugins(plugins: Vec<Arc<dyn RuntimePlugin>>) -> Self {
        Self { plugins }
    }

    pub async fn render(&self, template: &str, context: PromptContext<'_>) -> OrchestrationResult<String> {
        let mut working_template = template.to_string();
        for plugin in &self.plugins {
            working_template = plugin.transform_prompt_template(working_template).await;
        }

        let last_tool_json = match context.last_tool_output {
            Some(output) => serde_json::to_string_pretty(&output.content)?,
            None => "null".to_string(),
        };

        let notes = if context.plan.notes.is_empty() {
            "".to_string()
        } else {
            context.plan.notes.join("\n")
        };

        let mut rendered = working_template
            .replace("{{last_tool_output}}", &last_tool_json)
            .replace("{{last_model_output}}", context.last_model_output.unwrap_or("null"))
            .replace("{{session.id}}", &context.session.id)
            .replace("{{notes}}", &notes);

        for plugin in &self.plugins {
            rendered = plugin
                .transform_rendered_prompt(working_template.clone(), rendered)
                .await;
        }

        Ok(rendered)
    }
}
