use std::{cell::RefCell, collections::VecDeque, fs, io, path::Path, sync::Arc};

use anyhow::anyhow;
use openclaw_api_client::Error as ApiError;
use openclaw_plugin::RuntimePlugin;
use openclaw_tools::{
    ExecutionContext, Executor, PermissionError, ToolCall, ToolHandler, ToolOutput, ToolSpec,
};
use serde::{Deserialize, Serialize};
use std::io::BufRead;

use crate::{
    prompt::{PromptContext, PromptRenderer},
    session::Session,
    provider::Provider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestrationStep {
    Tool {
        spec: ToolSpec,
        call: ToolCall,
    },
    Model {
        model: String,
        prompt: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestrationPlan {
    pub steps: Vec<OrchestrationStep>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepResult {
    Tool(ToolOutput),
    Model { model: String, output: String },
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("tool permission error: {0}")]
    ToolPermission(#[from] PermissionError),
    #[error("provider error: {0}")]
    Provider(#[from] ApiError),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type OrchestrationResult<T> = Result<T, OrchestrationError>;

#[derive(Default)]
pub struct Orchestrator {
    executor: Executor,
    prompt_renderer: PromptRenderer,
    plugins: Vec<Arc<dyn RuntimePlugin>>,
    last_tool_output: RefCell<Option<ToolOutput>>,
    last_model_output: RefCell<Option<String>>,
    window_capacity: RefCell<usize>,
    history: RefCell<VecDeque<(usize, ToolOutput, String)>>, // (turn, tool_output, model_output)
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
            prompt_renderer: PromptRenderer::new(),
            plugins: Vec::new(),
            last_tool_output: RefCell::new(None),
            last_model_output: RefCell::new(None),
            window_capacity: RefCell::new(10), // default capacity
            history: RefCell::new(VecDeque::new()),
        }
    }

    pub fn with_plugins(plugins: Vec<Arc<dyn RuntimePlugin>>) -> Self {
        Self {
            executor: Executor::new(),
            prompt_renderer: PromptRenderer::with_plugins(plugins.clone()),
            plugins,
            last_tool_output: RefCell::new(None),
            last_model_output: RefCell::new(None),
            window_capacity: RefCell::new(10),
            history: RefCell::new(VecDeque::new()),
        }
    }

    pub fn set_window_capacity(&self, capacity: usize) {
        *self.window_capacity.borrow_mut() = capacity;
        // Trim history if needed
        let mut hist = self.history.borrow_mut();
        while capacity > 0 && hist.len() > capacity {
            hist.pop_front();
        }
    }

    pub fn window_capacity(&self) -> usize {
        *self.window_capacity.borrow()
    }

    /// Get a copy of the current history (for testing/inspection)
    pub fn history(&self) -> Vec<(usize, ToolOutput, String)> {
        self.history.borrow().iter().cloned().collect()
    }

    /// Push a completed turn into history and enforce capacity
    pub fn push_turn(&self, turn: usize, tool_output: ToolOutput, model_output: String) {
        // Clone for last_* before moving into history
        let tool_output_clone = tool_output.clone();
        let model_output_clone = model_output.clone();

        self.history.borrow_mut().push_back((turn, tool_output, model_output));
        let cap = *self.window_capacity.borrow();
        while cap > 0 && self.history.borrow().len() > cap {
            self.history.borrow_mut().pop_front();
        }

        // Update last_* for single-turn prompt compatibility
        *self.last_tool_output.borrow_mut() = Some(tool_output_clone);
        *self.last_model_output.borrow_mut() = Some(model_output_clone);
    }

    pub async fn execute_tool_step<H: ToolHandler>(
        &self,
        context: &ExecutionContext,
        spec: &ToolSpec,
        handler: &H,
        call: ToolCall,
    ) -> OrchestrationResult<ToolOutput> {
        Ok(self.executor.execute(context, spec, handler, call).await?)
    }

    pub async fn execute_plan<H: ToolHandler>(
        &self,
        provider: &Arc<dyn Provider + Send + Sync>,
        session: &Session,
        context: &ExecutionContext,
        handler: &H,
        plan: &OrchestrationPlan,
    ) -> OrchestrationResult<Vec<StepResult>> {
        let mut results = Vec::new();

        for step in &plan.steps {
            match step {
                OrchestrationStep::Tool { spec, call } => {
                    let mut effective_call = call.clone();
                    for plugin in &self.plugins {
                        let payload = plugin
                            .transform_tool_call(spec.name.clone(), effective_call.arguments.clone())
                            .await;
                        effective_call.arguments = payload.call;
                    }

                    let output = self
                        .executor
                        .execute(context, spec, handler, effective_call.clone())
                        .await?;

                    let mut effective_output = output.clone();
                    for plugin in &self.plugins {
                        let payload = plugin
                            .transform_tool_result(
                                spec.name.clone(),
                                effective_call.arguments.clone(),
                                effective_output.content.clone(),
                            )
                            .await;
                        if let Some(value) = payload.output {
                            effective_output.content = value;
                        }
                    }

                    // Persist for next model step
                    *self.last_tool_output.borrow_mut() = Some(effective_output.clone());
                    results.push(StepResult::Tool(effective_output));
                }
                OrchestrationStep::Model { model, prompt } => {
                    let rendered_prompt = self
                        .prompt_renderer
                        .render(
                            prompt,
                            PromptContext {
                                session,
                                plan,
                                last_tool_output: self.last_tool_output.borrow().as_ref(),
                                last_model_output: self.last_model_output.borrow().as_deref(),
                            },
                        )
                        .await?;

                    let mut effective_model = model.clone();
                    let mut effective_prompt = rendered_prompt;
                    for plugin in &self.plugins {
                        let payload = plugin
                            .transform_model_request(effective_model.clone(), effective_prompt.clone())
                            .await;
                        effective_model = payload.model;
                        effective_prompt = payload.prompt;
                    }

                    // Call new provider abstraction
                    let output = provider
                        .generate(&effective_prompt)
                        .await
                        .map_err(|e| ApiError::Provider(e.to_string()))?;

                    // Apply plugins if needed
                    let mut final_model = effective_model.clone();
                    let mut final_output = output;
                    for plugin in &self.plugins {
                        let payload = plugin
                            .transform_model_result(
                                final_model.clone(),
                                effective_prompt.clone(),
                                final_output.clone(),
                            )
                            .await;
                        final_model = payload.model;
                        if let Some(output) = payload.output {
                            final_output = output;
                        }
                    }

                    // Persist for next model step
                    *self.last_model_output.borrow_mut() = Some(final_output.clone());
                    results.push(StepResult::Model {
                        model: final_model,
                        output: final_output,
                    });
                }
            }
        }

        Ok(results)
    }

    /// 从 REPL 日志文件恢复最近的状态（用于会话恢复）
    /// 日志格式：每轮以 "## turn N" 开头，其后有 "output:" 行包含 JSON
    /// 取最后一轮的输出，解析出 last_tool_output 和 last_model_output
    pub fn restore_from_log(&self, log_path: &Path) -> anyhow::Result<()> {
        let file = fs::File::open(log_path).map_err(|e| anyhow!("open log failed: {}", e))?;
        let reader = io::BufReader::new(file);

        let mut last_tool: Option<ToolOutput> = None;
        let mut last_model: Option<String> = None;
        let mut lines = reader.lines().peekable();

        while let Some(line) = lines.next() {
            let line: io::Result<String> = line;
            let line = line?;
            if line.trim().starts_with("## turn") {
                // 开始一轮：收集 output 块（output: 之后直到下一个空行或下一轮）
                let mut in_output = false;
                let mut output_content = String::new();
                for next in &mut lines {
                    let next: io::Result<String> = next;
                    let next = next?;
                    if next.trim().starts_with("## turn") {
                        // 下一轮开始，回退该行供外层处理
                        let _ = next;
                        break;
                    }
                    if in_output {
                        if next.trim().is_empty() {
                            break; // output 块结束
                        }
                        output_content.push_str(&next);
                        output_content.push('\n');
                    } else if next.trim().starts_with("output:") {
                        in_output = true;
                        // output: 可能有剩余内容在本行
                        let rest = next["output:".len()..].trim();
                        if !rest.is_empty() {
                            output_content.push_str(rest);
                            output_content.push('\n');
                        }
                    }
                }
                if in_output && !output_content.is_empty() {
                    // 尝试解析为 StepResult 数组
                    if let Ok(results) = serde_json::from_str::<Vec<StepResult>>(&output_content) {
                        for step in results {
                            match step {
                                StepResult::Tool(tool_output) => {
                                    last_tool = Some(tool_output);
                                }
                                StepResult::Model { model: _, output } => {
                                    last_model = Some(output);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(last_model) = last_model {
            *self.last_model_output.borrow_mut() = Some(last_model);
        }
        if let Some(last_tool) = last_tool {
            *self.last_tool_output.borrow_mut() = Some(last_tool);
        }

        Ok(())
    }
}
