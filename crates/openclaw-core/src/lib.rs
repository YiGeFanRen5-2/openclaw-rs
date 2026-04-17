//! OpenClaw Core - Shared tool registry and execution logic
//! This crate contains the core tool management that can be used by both
//! node-bridge (N-API) and mcp-server (native binary).

use serde_json::Value as JsonValue;
use tools::{register_builtin_tools, ToolRegistry, ToolSchema};

/// Core OpenClaw runtime that manages tools and their schemas.
pub struct OpenClawCore {
    tool_registry: ToolRegistry,
}

impl Default for OpenClawCore {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenClawCore {
    pub fn new() -> Self {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);
        Self {
            tool_registry: registry,
        }
    }

    /// List all available tools with metadata
    pub fn list_tools(&self) -> Vec<CoreTool> {
        self.tool_registry
            .list_schemas()
            .iter()
            .map(|(name, schema)| CoreTool {
                name: (*name).to_string(),
                description: schema.description.clone().unwrap_or_default(),
                input_schema: schema.clone(),
            })
            .collect()
    }

    /// Get a specific tool's schema
    pub fn get_tool_info(&self, name: &str) -> Option<ToolInfo> {
        self.tool_registry.get_tool(name).map(|tool| {
            let schema = tool.input_schema();
            ToolInfo {
                name: tool.name().to_string(),
                description: schema.description.clone().unwrap_or_default(),
                parameters: schema.clone(),
            }
        })
    }

    /// Execute a tool by name with JSON arguments
    pub fn execute_tool(&self, name: &str, arguments: &JsonValue) -> Result<String, String> {
        let tool = self
            .tool_registry
            .get_tool(name)
            .ok_or_else(|| format!("Tool '{}' not found", name))?;

        match tool.execute(arguments.clone()) {
            Ok(result_json) => {
                if let Some(content) = result_json.get("content").and_then(|v| v.as_str()) {
                    Ok(content.to_string())
                } else {
                    Ok(result_json.to_string())
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Tool metadata (independent of MCP types)
#[derive(Debug, Clone)]
pub struct CoreTool {
    pub name: String,
    pub description: String,
    pub input_schema: ToolSchema,
}

/// Tool information (read-only)
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: ToolSchema,
}
