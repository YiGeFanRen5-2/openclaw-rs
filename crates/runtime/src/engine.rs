use std::sync::Arc;

use openclaw_api_client::{ChatRequest, ChatResponse, ChatMessage, Result as ApiResult};
use openclaw_plugin::RuntimePlugin;
use openclaw_tools::{ExecutionContext, ToolCall, ToolHandler, ToolOutput, ToolSpec};

use crate::orchestration::{OrchestrationPlan, Orchestrator, OrchestrationResult, StepResult};
use crate::provider::Provider as NewProvider;
use crate::session::Session;

pub struct RuntimeEngine {
    provider: Arc<dyn NewProvider + Send + Sync>,
    orchestrator: Orchestrator,
}

impl RuntimeEngine {
    pub fn new(provider: Arc<dyn NewProvider + Send + Sync>) -> Self {
        Self {
            provider,
            orchestrator: Orchestrator::new(),
        }
    }

    pub fn with_plugins(provider: Arc<dyn NewProvider + Send + Sync>, plugins: Vec<Arc<dyn RuntimePlugin>>) -> Self {
        Self {
            provider,
            orchestrator: Orchestrator::with_plugins(plugins),
        }
    }

    /// Compatibility wrapper: convert old ChatRequest to new provider::generate
    pub async fn run_chat(&self, request: ChatRequest) -> ApiResult<ChatResponse> {
        // Convert messages to a single prompt string
        let prompt = request.messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let content = self.provider.generate(&prompt).await
            .map_err(|e| openclaw_api_client::Error::Provider(e.to_string()))?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
            },
            model: request.model,
            usage: None,
        })
    }

    pub async fn run_tool<H: ToolHandler>(
        &self,
        context: &ExecutionContext,
        spec: &ToolSpec,
        handler: &H,
        call: ToolCall,
    ) -> OrchestrationResult<ToolOutput> {
        self.orchestrator
            .execute_tool_step(context, spec, handler, call)
            .await
    }

    pub async fn run_plan<H: ToolHandler>(
        &self,
        session: &Session,
        context: &ExecutionContext,
        handler: &H,
        plan: &OrchestrationPlan,
    ) -> OrchestrationResult<Vec<StepResult>> {
        self.orchestrator
            .execute_plan(&self.provider, session, context, handler, plan)
            .await
    }

    pub fn orchestrator(&self) -> &Orchestrator {
        &self.orchestrator
    }
}
