use napi_derive::napi;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

// Bring in OpenClaw crates
use api_client::{ChatMessage, ChatResponse};
use runtime::lsp::LspBridge;
use runtime::{
    provider::{self, Provider},
    Role, Runtime as RustRuntime, RuntimeConfig, SessionId,
};
use tools::{register_builtin_tools, Sandbox, ToolRegistry};

#[napi]
pub enum ProviderMode {
    Mock,
    Openai,
    Anthropic,
    Gemini,
}

#[napi]
pub struct OpenClawRuntime {
    rt: Runtime,
    provider: Option<Box<dyn Provider + Send + Sync>>,
    session_store: Option<String>,
    tool_registry: ToolRegistry,
    sandbox: Sandbox,
    rust_runtime: Option<RustRuntime>, // 完整的 Rust 运行时（包含 session 管理）
    lsp_bridge: Option<LspBridge>,     // LSP 桥接（可选）
}

#[derive(Serialize, Deserialize)]
struct ChatRequestPayload {
    messages: Vec<ChatMessage>,
    model: Option<String>,
}

#[napi]
impl OpenClawRuntime {
    #[napi(constructor)]
    pub fn new(
        provider: ProviderMode,
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
    ) -> napi::Result<Self> {
        let rt = match Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                return Err(napi::Error::from_reason(format!(
                    "Failed to create runtime: {}",
                    e
                )))
            }
        };

        // Build provider config
        let kind = match provider {
            ProviderMode::Mock => "mock",
            ProviderMode::Openai => "openai",
            ProviderMode::Anthropic => "anthropic",
            ProviderMode::Gemini => "gemini",
        };
        let mut config = provider::ProviderConfig::new(kind);
        if let Some(key) = api_key {
            config = config.api_key(key);
        }
        if let Some(url) = base_url {
            config = config.base_url(url);
        }
        if let Some(m) = model {
            config = config.model(m);
        }

        // Use factory to create provider
        let provider_box = provider::create_provider(&config)
            .map_err(|e| napi::Error::from_reason(format!("Failed to create provider: {}", e)))?;

        // Initialize tool registry with built-in tools
        let mut tool_registry = ToolRegistry::new();
        register_builtin_tools(&mut tool_registry);

        // Initialize sandbox for tool execution
        let sandbox = Sandbox::new();

        // Initialize Rust runtime with session management
        let rust_config = RuntimeConfig {
            max_session_tokens: 4000,
            compaction_threshold: 0.8,
            min_recent_messages: 4,
            persist_path: None, // 将由 with_session_store 设置
        };
        let rust_runtime = RustRuntime::new(rust_config).map_err(|e| {
            napi::Error::from_reason(format!("Failed to create Rust runtime: {}", e))
        })?;

        Ok(Self {
            rt,
            provider: Some(provider_box),
            session_store: None,
            tool_registry,
            sandbox,
            rust_runtime: Some(rust_runtime),
            lsp_bridge: None,
        })
    }

    #[napi]
    pub fn with_session_store(&mut self, path: String) -> napi::Result<()> {
        self.session_store = Some(path);
        Ok(())
    }

    #[napi]
    pub fn chat(&self, messages_json: String) -> napi::Result<String> {
        // Parse incoming messages
        let payload: ChatRequestPayload = match serde_json::from_str(&messages_json) {
            Ok(p) => p,
            Err(e) => return Err(napi::Error::from_reason(format!("Invalid JSON: {}", e))),
        };

        // Build prompt: each line "role: content"
        let prompt = payload
            .messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Get provider reference (clone the Box)
        let provider = match &self.provider {
            Some(p) => p.as_ref(),
            None => {
                return Err(napi::Error::from_reason(
                    "Provider not initialized".to_string(),
                ))
            }
        };

        // Run async generate on the tokio runtime (block_on)
        let result = self.rt.block_on(async {
            provider
                .generate(&prompt)
                .await
                .map_err(|e| napi::Error::from_reason(format!("Provider error: {}", e)))
        });

        let content = result?;

        // Build response: { message: { role: "assistant", content }, model, usage? }
        let response = ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
            },
            model: payload.model.unwrap_or_else(|| "mock".to_string()),
            usage: None,
        };

        serde_json::to_string(&response)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    #[napi]
    pub fn execute_plan(&self, plan_json: String) -> napi::Result<String> {
        // For now, just echo back the plan
        let result = serde_json::json!({
            "status": "ok",
            "plan": plan_json
        });
        Ok(serde_json::to_string(&result).unwrap_or_default())
    }

    #[napi]
    pub fn save_session(&self, _session_id: String) -> napi::Result<()> {
        if self.session_store.is_some() {
            Ok(())
        } else {
            Err(napi::Error::from_reason(
                "No session store configured".to_string(),
            ))
        }
    }

    #[napi]
    pub fn shutdown(&mut self) {
        self.provider = None;
        self.rust_runtime = None;
    }

    // ===== Session Management API =====

    #[napi]
    pub fn create_session(&mut self, session_id: String) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        rt.create_session(session_id)
            .map_err(|e| napi::Error::from_reason(format!("Failed to create session: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn get_session(&self, session_id: String) -> napi::Result<String> {
        let rt = self
            .rust_runtime
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        let session = rt
            .get_session(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Session not found: {}", e)))?;
        let json = serde_json::to_string(session)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))?;
        Ok(json)
    }

    #[napi]
    pub fn list_sessions(&self) -> napi::Result<Vec<String>> {
        let rt = self
            .rust_runtime
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        let ids = rt
            .list_sessions()
            .into_iter()
            .map(|id| id.into_string())
            .collect();
        Ok(ids)
    }

    #[napi]
    pub fn delete_session(&mut self, session_id: String) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        rt.delete_session(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Failed to delete session: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn add_message(
        &mut self,
        session_id: String,
        role: String,
        content: String,
    ) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        let role_enum = match role.to_lowercase().as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "tool" => Role::Tool,
            _ => return Err(napi::Error::from_reason(format!("Invalid role: {}", role))),
        };
        let session = rt
            .get_session_mut(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Session not found: {}", e)))?;
        session
            .add_message(role_enum, content)
            .map_err(|e| napi::Error::from_reason(format!("Failed to add message: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn compact_session(&mut self, session_id: String) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        rt.compact_session(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Compaction failed: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn persist_session(&self, session_id: String) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        rt.persist_session(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Persist failed: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn restore_session(&mut self, session_id: String) -> napi::Result<()> {
        let rt = self
            .rust_runtime
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
        rt.restore_session(&SessionId::from(session_id))
            .map_err(|e| napi::Error::from_reason(format!("Restore failed: {}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn set_session_store(&mut self, path: String) -> napi::Result<()> {
        self.session_store = Some(path.clone());
        if let Some(rust_rt) = &mut self.rust_runtime {
            rust_rt.set_persist_path(Some(std::path::PathBuf::from(path)));
        }
        Ok(())
    }

    // ===== New Tool API =====

    #[napi]
    pub fn list_tools(&self) -> napi::Result<Vec<String>> {
        // Use tool_registry from node-bridge (not from Rust runtime)
        let names = self
            .tool_registry
            .list_tools()
            .into_iter()
            .map(|name: &'static str| name.to_string())
            .collect();
        Ok(names)
    }

    #[napi]
    pub fn execute_tool(
        &mut self,
        session_id: String,
        tool_name: String,
        arguments_json: String,
    ) -> napi::Result<String> {
        // Find tool in registry
        let tool = self
            .tool_registry
            .get_tool(&tool_name)
            .ok_or_else(|| napi::Error::from_reason(format!("Tool not found: {}", tool_name)))?;

        // Parse arguments
        let args = match serde_json::from_str::<serde_json::Value>(&arguments_json) {
            Ok(v) => v,
            Err(e) => return Err(napi::Error::from_reason(format!("Invalid JSON: {}", e))),
        };

        // Permission check
        Self::check_permission(tool.permission(), &args)?;

        // Execute via Rust runtime if session provided (to record to session)
        if !session_id.is_empty() {
            let rt = self
                .rust_runtime
                .as_mut()
                .ok_or_else(|| napi::Error::from_reason("Rust runtime not initialized"))?;
            let call = tools::ToolCall {
                name: tool_name.clone(),
                arguments: arguments_json.clone(),
            };
            let result = rt
                .execute_tool(&SessionId::from(session_id), call)
                .map_err(|e| napi::Error::from_reason(format!("Tool execution failed: {}", e)))?;
            serde_json::to_string(&result)
                .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
        } else {
            // No session: execute directly via sandbox
            let result = self
                .sandbox
                .execute(tool, args)
                .map_err(|e| napi::Error::from_reason(format!("Tool error: {}", e)))?;
            serde_json::to_string(&result)
                .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
        }
    }

    /// Helper: check tool permission against arguments
    fn check_permission(
        permission: tools::Permission,
        args: &serde_json::Value,
    ) -> napi::Result<()> {
        match permission {
            tools::Permission::Filesystem { .. } => {
                let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                    napi::Error::from_reason("Missing 'path' for filesystem permission check")
                })?;
                // Use Permission::check to validate path is in allowlist
                permission
                    .check("read", path)
                    .map_err(|e| napi::Error::from_reason(format!("Permission denied: {}", e)))?;
            }
            tools::Permission::Shell { .. } => {
                // For shell tools, check command
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        napi::Error::from_reason("Missing 'command' for shell permission check")
                    })?;
                permission
                    .check(command, "")
                    .map_err(|e| napi::Error::from_reason(format!("Permission denied: {}", e)))?;
            }
            tools::Permission::Network { .. } => {
                let target = args.get("target").and_then(|v| v.as_str()).ok_or_else(|| {
                    napi::Error::from_reason("Missing 'target' for network permission check")
                })?;
                permission
                    .check("connect", target)
                    .map_err(|e| napi::Error::from_reason(format!("Permission denied: {}", e)))?;
            }
            tools::Permission::Safe => {}
            tools::Permission::Custom { .. } => {
                // Custom permission: skip pre-check, assume custom checker in tool
            }
        }
        Ok(())
    }

    // ===== LSP Bridge API =====

    /// Initialize LSP bridge with a language server (e.g., "rust-analyzer").
    #[napi]
    pub fn lsp_init(&mut self, server_name: String, server_cmd: Vec<String>) -> napi::Result<()> {
        let bridge = LspBridge::new(&server_name, server_cmd);
        self.lsp_bridge = Some(bridge);
        Ok(())
    }

    /// Connect and initialize the LSP server with a root URI.
    #[napi]
    pub fn lsp_connect(&mut self, root_uri: String) -> napi::Result<()> {
        let bridge = self.lsp_bridge.as_mut().ok_or_else(|| {
            napi::Error::from_reason("LSP bridge not initialized. Call lsp_init first.")
        })?;
        self.rt.block_on(async {
            bridge
                .connect(&root_uri)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP connect failed: {}", e)))
        })
    }

    /// Open a document in the LSP server.
    #[napi]
    pub fn lsp_did_open(
        &mut self,
        uri: String,
        language_id: String,
        text: String,
    ) -> napi::Result<()> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        self.rt.block_on(async {
            bridge
                .did_open(&uri, &language_id, &text)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP did_open failed: {}", e)))
        })
    }

    /// Get completions at a cursor position.
    #[napi]
    pub fn lsp_completions(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let completions = self.rt.block_on(async {
            bridge
                .completions(&uri, line, character)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP completions failed: {}", e)))
        })?;
        serde_json::to_string(&completions)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Get hover info at a cursor position.
    #[napi]
    pub fn lsp_hover(&mut self, uri: String, line: u32, character: u32) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let hover = self.rt.block_on(async {
            bridge
                .hover(&uri, line, character)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP hover failed: {}", e)))
        })?;
        serde_json::to_string(&hover)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Go to definition at a cursor position.
    #[napi]
    pub fn lsp_goto_definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let locations = self.rt.block_on(async {
            bridge
                .goto_definition(&uri, line, character)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP goto_definition failed: {}", e)))
        })?;
        serde_json::to_string(&locations)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Find all references to a symbol.
    #[napi]
    pub fn lsp_find_references(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let locations = self.rt.block_on(async {
            bridge
                .find_references(&uri, line, character)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP find_references failed: {}", e)))
        })?;
        serde_json::to_string(&locations)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Get document symbols (outline) for a file.
    #[napi]
    pub fn lsp_document_symbols(&mut self, uri: String) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let symbols = self.rt.block_on(async {
            bridge.document_symbols(&uri).await.map_err(|e| {
                napi::Error::from_reason(format!("LSP document_symbols failed: {}", e))
            })
        })?;
        serde_json::to_string(&symbols)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Get workspace symbols matching a query.
    #[napi]
    pub fn lsp_workspace_symbol(&mut self, query: String) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let symbols = self.rt.block_on(async {
            bridge.workspace_symbol(&query).await.map_err(|e| {
                napi::Error::from_reason(format!("LSP workspace_symbol failed: {}", e))
            })
        })?;
        serde_json::to_string(&symbols)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Get current diagnostics for a file.
    #[napi]
    pub fn lsp_diagnostics(&mut self, uri: String) -> napi::Result<String> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        let diags = self.rt.block_on(async {
            bridge
                .diagnostics(&uri)
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP diagnostics failed: {}", e)))
        })?;
        serde_json::to_string(&diags)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }

    /// Shutdown the LSP server gracefully.
    #[napi]
    pub fn lsp_shutdown(&mut self) -> napi::Result<()> {
        let bridge = self
            .lsp_bridge
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("LSP bridge not initialized"))?;
        self.rt.block_on(async {
            bridge
                .shutdown()
                .await
                .map_err(|e| napi::Error::from_reason(format!("LSP shutdown failed: {}", e)))
        })
    }

    // ===== JS Tool Registration API =====

    /// Register a custom tool from JavaScript.
    /// The tool is registered in the tool registry and runtime.
    /// arguments_json: JSON schema for the tool's input parameters.
    #[napi]
    pub fn register_tool(
        &mut self,
        name: String,
        description: String,
        input_schema_json: String,
    ) -> napi::Result<()> {
        let schema: serde_json::Value = serde_json::from_str(&input_schema_json)
            .map_err(|e| napi::Error::from_reason(format!("Invalid JSON schema: {}", e)))?;

        // Create a JS tool wrapper (execution delegated to execute_js_tool)
        let tool = JsTool {
            name: name.clone(),
            description: description.clone(),
            schema: schema.clone(),
            permission: tools::Permission::Safe,
        };

        self.tool_registry.register(tool);

        // Also register in the Rust runtime if available
        if let Some(runtime) = &mut self.rust_runtime {
            let js_tool = JsTool {
                name: name.clone(),
                description: format!("(JS tool) {}", description.clone()),
                schema: serde_json::from_str(&input_schema_json).unwrap_or(serde_json::json!({})),
                permission: tools::Permission::Safe,
            };
            let _ = runtime.register_tool(Box::new(js_tool));
        }

        Ok(())
    }

    /// Get a summary of the runtime status (for debugging/monitoring).
    #[napi]
    pub fn runtime_status(&self) -> napi::Result<String> {
        let status = serde_json::json!({
            "provider": self.provider.is_some(),
            "session_store": self.session_store,
            "tools_count": self.tool_registry.list_tools().len(),
            "lsp_bridge": self.lsp_bridge.is_some(),
        });
        serde_json::to_string(&status)
            .map_err(|e| napi::Error::from_reason(format!("Serialization error: {}", e)))
    }
}

/// A tool registered from JavaScript via the Node.js API.
/// Stores metadata; execution is handled by the node-bridge's `execute_js_tool`.
#[derive(Clone)]
struct JsTool {
    name: String,
    description: String,
    schema: serde_json::Value,
    permission: tools::Permission,
}

impl tools::Tool for JsTool {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str()) as &'static str
    }
    fn description(&self) -> &'static str {
        Box::leak(self.description.clone().into_boxed_str()) as &'static str
    }
    fn input_schema(&self) -> tools::ToolSchema {
        serde_json::from_value(self.schema.clone()).unwrap_or_else(|_| tools::ToolSchema {
            r#type: "object".to_string(),
            description: None,
            properties: None,
            required: None,
        })
    }
    fn output_schema(&self) -> tools::ToolSchema {
        tools::ToolSchema {
            r#type: "object".to_string(),
            description: None,
            properties: None,
            required: None,
        }
    }
    fn permission(&self) -> tools::Permission {
        self.permission.clone()
    }
    fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, tools::ToolError> {
        // Execution is handled by node-bridge's execute_js_tool method
        Ok(serde_json::json!({ "status": "js_tool", "name": self.name }))
    }
}
