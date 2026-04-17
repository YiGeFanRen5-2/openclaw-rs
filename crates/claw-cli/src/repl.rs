use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufRead, Write},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use runtime::{
    provider::{self, Provider, ProviderConfig},
    OrchestrationPlan, OrchestrationStep, RuntimeEngine, Session, StepResult,
};
use openclaw_plugin::{
    ModelHook, ModelHookPayload, Plugin, PluginMetadata, PromptHook, RuntimePlugin, ToolHook,
    ToolHookPayload,
};
use tools::{
    EchoTool, ExecutionContext, Permission, PermissionSet, ToolCall, ToolSpec, ToolOutput,
};
use serde::Serialize;
use serde_json::json;

use crate::cli::{DemoArgs, ProviderMode, ReplArgs, SharedProviderArgs, StatusArgs};

pub struct Repl;

impl Repl {
    pub fn new() -> Self {
        Self
    }

    pub async fn run_status(&self, args: StatusArgs) -> anyhow::Result<String> {
        let status = provider_status(&args.provider);
        Ok(serde_json::to_string_pretty(&status)?)
    }

    pub async fn run_demo(&self, args: DemoArgs) -> anyhow::Result<String> {
        let provider = Self::build_provider(&args.provider).await?;
        self.run_demo_with_provider(provider, args).await
    }

    pub async fn run_interactive(&self, args: ReplArgs) -> anyhow::Result<String> {
        let provider = Self::build_provider(&args.provider).await?;
        self.run_interactive_with_provider(provider, args).await
    }

    async fn build_provider(
        args: &SharedProviderArgs,
    ) -> anyhow::Result<Arc<dyn Provider + Send + Sync>> {
        match args.provider {
            ProviderMode::Mock => Ok(Arc::new(runtime::provider::MockProvider::new())),
            ProviderMode::Openai => {
                // Build provider config from args
                let mut cfg = provider::ProviderConfig::new("openai");
                if let Some(api_key) = args.api_key.clone()
                    .or_else(|| env::var("OPENCLAW_API_KEY").ok())
                    .or_else(|| env::var("OPENAI_API_KEY").ok())
                {
                    cfg = cfg.api_key(api_key);
                }
                if let Some(base_url) = args.base_url.clone()
                    .or_else(|| env::var("OPENCLAW_BASE_URL").ok())
                    .or_else(|| env::var("OPENAI_BASE_URL").ok())
                {
                    cfg = cfg.base_url(base_url);
                }
                // For now, use stub OpenAIProvider (not live unless feature enabled)
                Ok(Arc::new(runtime::provider::OpenAIProvider::new(&cfg).await?))
            }
            ProviderMode::Anthropic => {
                let mut cfg = provider::ProviderConfig::new("anthropic");
                if let Some(api_key) = args.api_key.clone()
                    .or_else(|| env::var("OPENCLAW_API_KEY").ok())
                    .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
                {
                    cfg = cfg.api_key(api_key);
                }
                if let Some(base_url) = args.base_url.clone()
                    .or_else(|| env::var("OPENCLAW_BASE_URL").ok())
                {
                    cfg = cfg.base_url(base_url);
                }
                if let Some(model) = env::var("ANTHROPIC_MODEL").ok() {
                    cfg = cfg.model(model);
                }
                Ok(Arc::new(runtime::provider::AnthropicProvider::new(&cfg).await?))
            }
            ProviderMode::Gemini => {
                let mut cfg = provider::ProviderConfig::new("gemini");
                if let Some(api_key) = args.api_key.clone()
                    .or_else(|| env::var("OPENCLAW_API_KEY").ok())
                    .or_else(|| env::var("GEMINI_API_KEY").ok())
                {
                    cfg = cfg.api_key(api_key);
                }
                if let Some(base_url) = args.base_url.clone()
                    .or_else(|| env::var("OPENCLAW_BASE_URL").ok())
                {
                    cfg = cfg.base_url(base_url);
                }
                if let Some(model) = env::var("GEMINI_MODEL").ok() {
                    cfg = cfg.model(model);
                }
                Ok(Arc::new(runtime::provider::GeminiProvider::new(&cfg).await?))
            }
        }
    }

    fn build_plugins(plugin_names: &[String]) -> Vec<Arc<dyn RuntimePlugin>> {
        let mut plugins = Vec::new();
        for name in plugin_names {
            match name.as_str() {
                "demo" => plugins.push(Arc::new(DemoPlugin::new()) as Arc<dyn RuntimePlugin>),
                // Future: load from config, dynamic libs, etc.
                _ => {
                    eprintln!("unknown plugin: {}, skipping", name);
                }
            }
        }
        plugins
    }

    async fn run_interactive_with_provider(
        &self,
        provider: Arc<dyn Provider + Send + Sync>,
        args: ReplArgs,
    ) -> anyhow::Result<String> {
        let engine = if args.provider.no_plugin {
            RuntimeEngine::new(provider)
        } else {
            let plugins = Self::build_plugins(&args.provider.plugins);
            RuntimeEngine::with_plugins(provider, plugins)
        };
        // Apply window capacity configuration
        engine.orchestrator().set_window_capacity(args.provider.window_capacity);

        let mut permissions = PermissionSet::new();
        permissions.allow(Permission::ReadFile);
        let context = ExecutionContext::new(permissions);
        let spec = ToolSpec::new("echo", "Echo demo tool")
            .require_permission(Permission::ReadFile);

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        let (session_id, log_path, mut turn) = if let Some(resume_id) = &args.provider.resume {
            // 恢复会话：使用现有 session_id 和日志
            let log_path = repl_log_path(resume_id)?;
            // 尝试从日志恢复状态
            if let Err(e) = engine.orchestrator().restore_from_log(&log_path) {
                eprintln!("警告：无法从日志恢复：{}", e);
            }
            // 从日志统计已有轮次
            let existing_turns = count_existing_turns(&log_path)?;
            (resume_id.clone(), log_path, existing_turns)
        } else {
            // 新建会话
            let session_id = format!("repl-{}", Utc::now().format("%Y%m%dT%H%M%SZ"));
            let log_path = repl_log_path(&session_id)?;
            writeln!(stdout, "交互式 REPL 已就绪。输入消息，或输入 'exit' 退出。")?;
            writeln!(stdout, "会话 ID：{}", session_id)?;
            writeln!(stdout, "日志：{}", log_path.display())?;
            stdout.flush()?;
            append_repl_log(&log_path, &format!("# session={} model={} started_at={}\n", session_id, args.model, Utc::now().to_rfc3339()))?;
            (session_id, log_path, 0)
        };

        let session = Session::new(session_id.clone()).with_model(args.model.clone());

        for line in stdin.lock().lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                append_repl_log(&log_path, "\n# session ended by user\n")?;
                break;
            }
            if trimmed.is_empty() {
                continue;
            }

            turn += 1;
            let call = ToolCall {
                arguments: json!({ "message": trimmed }),
            };
            let plan = OrchestrationPlan {
                steps: vec![
                    OrchestrationStep::Tool {
                        spec: spec.clone(),
                        call,
                    },
                    OrchestrationStep::Model {
                        model: args.model.clone(),
                        prompt: args.prompt.clone(),
                    },
                ],
                notes: vec![format!("interactive turn {}", turn)],
            };

            let results = engine.run_plan(&session, &context, &EchoTool, &plan).await?;
            let rendered = serde_json::to_string_pretty(&results)?;
            writeln!(stdout, "{}", rendered)?;
            stdout.flush()?;

            // Extract tool_output and model_output for history
            let mut tool_output_opt: Option<&ToolOutput> = None;
            let mut model_output_opt: Option<&String> = None;
            for step in &results {
                match step {
                    StepResult::Tool(to) => {
                        tool_output_opt = Some(to);
                    }
                    StepResult::Model { model: _, output } => {
                        model_output_opt = Some(output);
                    }
                }
            }
            if let (Some(tool_output), Some(model_output)) = (tool_output_opt, model_output_opt) {
                engine.orchestrator().push_turn(turn, tool_output.clone(), model_output.clone());
            }

            append_repl_log(
                &log_path,
                &format!(
                    "\n## turn {}\ninput: {}\noutput:\n{}\n",
                    turn, trimmed, rendered
                ),
            )?;
        }

        Ok(format!("interactive session ended ({session_id})"))
    }

    async fn run_demo_with_provider(
        &self,
        provider: Arc<dyn Provider + Send + Sync>,
        args: DemoArgs,
    ) -> anyhow::Result<String> {
        let engine = if args.provider.no_plugin {
            RuntimeEngine::new(provider)
        } else {
            let plugins = Self::build_plugins(&args.provider.plugins);
            RuntimeEngine::with_plugins(provider, plugins)
        };
        let session = Session::new("demo-session").with_model(args.model.clone());

        let mut permissions = PermissionSet::new();
        permissions.allow(Permission::ReadFile);
        let context = ExecutionContext::new(permissions);

        let spec = ToolSpec::new("echo", "Echo demo tool")
            .require_permission(Permission::ReadFile);
        let call = ToolCall {
            arguments: json!({
                "message": args.message
            }),
        };

        let plan = OrchestrationPlan {
            steps: vec![
                OrchestrationStep::Tool { spec, call },
                OrchestrationStep::Model {
                    model: args.model,
                    prompt: args.prompt,
                },
            ],
            notes: vec!["demo plan from claw-cli".to_string()],
        };

        let results = engine.run_plan(&session, &context, &EchoTool, &plan).await?;
        Ok(serde_json::to_string_pretty(&results)?)
    }
}

#[derive(Debug, Serialize)]
struct ProviderStatusReport {
    provider: String,
    plugin_enabled: bool,
    api_key_present: bool,
    api_key_source: Option<String>,
    base_url: Option<String>,
    base_url_source: Option<String>,
    ready: bool,
    notes: Vec<String>,
}

fn repl_log_path(session_id: &str) -> anyhow::Result<PathBuf> {
    let dir = PathBuf::from("/root/.openclaw/workspace/openclaw-rs/logs/repl");
    fs::create_dir_all(&dir)?;
    Ok(dir.join(format!("{}.md", session_id)))
}

fn append_repl_log(path: &PathBuf, content: &str) -> anyhow::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn count_existing_turns(log_path: &PathBuf) -> anyhow::Result<usize> {
    if !log_path.exists() {
        return Ok(0);
    }
    let file = fs::File::open(log_path)?;
    let reader = io::BufReader::new(file);
    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if line.trim().starts_with("## turn") {
            count += 1;
        }
    }
    Ok(count)
}

fn provider_status(args: &SharedProviderArgs) -> ProviderStatusReport {
    match args.provider {
        ProviderMode::Mock => ProviderStatusReport {
            provider: "mock".to_string(),
            plugin_enabled: !args.no_plugin,
            api_key_present: false,
            api_key_source: None,
            base_url: None,
            base_url_source: None,
            ready: true,
            notes: vec!["mock provider is always ready for local demo runs".to_string()],
        },
        ProviderMode::Openai => {
            let cli_api_key = args.api_key.clone();
            let env_openclaw_api_key = env::var("OPENCLAW_API_KEY").ok();
            let env_openai_api_key = env::var("OPENAI_API_KEY").ok();
            let cli_base_url = args.base_url.clone();
            let env_openclaw_base_url = env::var("OPENCLAW_BASE_URL").ok();
            let env_openai_base_url = env::var("OPENAI_BASE_URL").ok();

            let (api_key_present, api_key_source) = if cli_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("cli --api-key".to_string()))
            } else if env_openclaw_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env OPENCLAW_API_KEY".to_string()))
            } else if env_openai_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env OPENAI_API_KEY".to_string()))
            } else {
                (false, None)
            };

            let (base_url, base_url_source) = if cli_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (cli_base_url, Some("cli --base-url".to_string()))
            } else if env_openclaw_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (env_openclaw_base_url, Some("env OPENCLAW_BASE_URL".to_string()))
            } else if env_openai_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (env_openai_base_url, Some("env OPENAI_BASE_URL".to_string()))
            } else {
                (Some("https://api.openai.com".to_string()), Some("default".to_string()))
            };

            let mut notes = Vec::new();
            if !api_key_present {
                notes.push("missing api key: pass --api-key or set OPENCLAW_API_KEY / OPENAI_API_KEY".to_string());
            }
            if base_url.is_none() {
                notes.push("base_url is missing".to_string());
            }

            ProviderStatusReport {
                provider: "openai".to_string(),
                plugin_enabled: !args.no_plugin,
                api_key_present,
                api_key_source,
                base_url,
                base_url_source,
                ready: api_key_present,
                notes,
            }
        }
        ProviderMode::Anthropic => {
            let cli_api_key = args.api_key.clone();
            let env_openclaw_api_key = env::var("OPENCLAW_API_KEY").ok();
            let env_anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
            let cli_base_url = args.base_url.clone();
            let env_openclaw_base_url = env::var("OPENCLAW_BASE_URL").ok();

            let (api_key_present, api_key_source) = if cli_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("cli --api-key".to_string()))
            } else if env_openclaw_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env OPENCLAW_API_KEY".to_string()))
            } else if env_anthropic_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env ANTHROPIC_API_KEY".to_string()))
            } else {
                (false, None)
            };

            let (base_url, base_url_source) = if cli_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (cli_base_url, Some("cli --base-url".to_string()))
            } else if env_openclaw_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (env_openclaw_base_url, Some("env OPENCLAW_BASE_URL".to_string()))
            } else {
                (Some("https://api.anthropic.com/v1".to_string()), Some("default".to_string()))
            };

            let mut notes = Vec::new();
            if !api_key_present {
                notes.push("missing api key: pass --api-key or set OPENCLAW_API_KEY / ANTHROPIC_API_KEY".to_string());
            }

            ProviderStatusReport {
                provider: "anthropic".to_string(),
                plugin_enabled: !args.no_plugin,
                api_key_present,
                api_key_source,
                base_url,
                base_url_source,
                ready: api_key_present,
                notes,
            }
        }
        ProviderMode::Gemini => {
            let cli_api_key = args.api_key.clone();
            let env_openclaw_api_key = env::var("OPENCLAW_API_KEY").ok();
            let env_gemini_api_key = env::var("GEMINI_API_KEY").ok();
            let cli_base_url = args.base_url.clone();
            let env_openclaw_base_url = env::var("OPENCLAW_BASE_URL").ok();

            let (api_key_present, api_key_source) = if cli_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("cli --api-key".to_string()))
            } else if env_openclaw_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env OPENCLAW_API_KEY".to_string()))
            } else if env_gemini_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (true, Some("env GEMINI_API_KEY".to_string()))
            } else {
                (false, None)
            };

            let (base_url, base_url_source) = if cli_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (cli_base_url, Some("cli --base-url".to_string()))
            } else if env_openclaw_base_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                (env_openclaw_base_url, Some("env OPENCLAW_BASE_URL".to_string()))
            } else {
                (Some("https://generativelanguage.googleapis.com".to_string()), Some("default".to_string()))
            };

            let mut notes = Vec::new();
            if !api_key_present {
                notes.push("missing api key: pass --api-key or set OPENCLAW_API_KEY / GEMINI_API_KEY".to_string());
            }

            ProviderStatusReport {
                provider: "gemini".to_string(),
                plugin_enabled: !args.no_plugin,
                api_key_present,
                api_key_source,
                base_url,
                base_url_source,
                ready: api_key_present,
                notes,
            }
        }
    }
}



struct DemoPlugin {
    metadata: PluginMetadata,
}

impl DemoPlugin {
    fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "demo-plugin".to_string(),
                version: "0.1.0".to_string(),
                description: Some("demonstrates runtime hook effects".to_string()),
            },
        }
    }
}

impl Plugin for DemoPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
}

#[async_trait]
impl PromptHook for DemoPlugin {}

#[async_trait]
impl ToolHook for DemoPlugin {
    async fn before_tool(&self, mut payload: ToolHookPayload) -> ToolHookPayload {
        if let Some(message) = payload.call.get("message").and_then(|v| v.as_str()) {
            payload.call["message"] = json!(format!("{} [tool-hook]", message));
        }
        payload
    }

    async fn after_tool(&self, mut payload: ToolHookPayload) -> ToolHookPayload {
        let mut output = payload.output.unwrap_or_else(|| json!({}));
        output["plugin_tag"] = json!("demo-plugin");
        payload.output = Some(output);
        payload
    }
}

#[async_trait]
impl ModelHook for DemoPlugin {
    async fn before_model(&self, mut payload: ModelHookPayload) -> ModelHookPayload {
        payload.prompt = format!("[demo-before-model] {}", payload.prompt);
        payload
    }

    async fn after_model(&self, mut payload: ModelHookPayload) -> ModelHookPayload {
        payload.output = payload.output.map(|o| format!("{} [demo-after-model]", o));
        payload
    }
}

impl RuntimePlugin for DemoPlugin {}
