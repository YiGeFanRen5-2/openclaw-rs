//! OpenClaw MCP (Model Context Protocol) Server
//! Simple stdio-based MCP server for integration with Claude Desktop, Cursor, etc.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::io::{BufRead, BufReader, Write};

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
    Null,
}

/// JSON-RPC 2.0 Response (success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    id: RequestId,
    result: JsonValue,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JsonRpcError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

/// MCP Initialize params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams {
    protocol_version: String,
    capabilities: ClientCapabilities,
    client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    experimental: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sampling: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientInfo {
    name: String,
    version: String,
}

/// MCP Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeResult {
    protocol_version: String,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts: Option<PromptCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResourceCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptCapability {}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

/// MCP Tool call params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<JsonValue>,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Prompt definition (as registered)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    /// Template messages with optional {{arg}} placeholders
    pub messages: Vec<PromptMessage>,
}

/// MCP Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// MCP Prompt message template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: PromptContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PromptContent {
    Text { text: String },
    Resource { resource: ResourceReference },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub uri: String,
    pub mime_type: Option<String>,
}

/// MCP Resource Content (part of read response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

type Value = serde_json::Value;

/// MCP Server
pub struct McpServer {
    server_name: String,
    server_version: String,
    pub tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    #[allow(clippy::type_complexity)]
    tool_executor: Option<Box<dyn Fn(&str, &JsonValue) -> String + Send + Sync>>,
}

impl McpServer {
    pub fn new(server_name: &str, server_version: &str) -> Self {
        Self {
            server_name: server_name.to_string(),
            server_version: server_version.to_string(),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            tool_executor: None,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn with_tool_executor(
        mut self,
        executor: Box<dyn Fn(&str, &JsonValue) -> String + Send + Sync>,
    ) -> Self {
        self.tool_executor = Some(executor);
        self
    }

    pub fn register_tool(&mut self, tool: McpTool) {
        self.tools.push(tool);
    }

    pub fn register_resource(&mut self, resource: McpResource) {
        self.resources.push(resource);
    }

    pub fn register_prompt(&mut self, prompt: McpPrompt) {
        self.prompts.push(prompt);
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    self.send_error(&mut out, &line, -32700, format!("Parse error: {}", e))?;
                    continue;
                }
            };

            // JSON-RPC 2.0: Notifications (id = null) get no response
            if matches!(request.id, Some(RequestId::Null)) {
                continue;
            }

            // JSON-RPC 2.0: Notifications (no id) get no response
            if request.id.is_none() {
                continue;
            }

            match self.handle_request(&request) {
                Ok(value) => {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.clone().unwrap_or(RequestId::Null),
                        result: value,
                    };
                    let resp_json = serde_json::to_string(&resp)?;
                    writeln!(out, "{}", resp_json)?;
                }
                Err(err) => {
                    let resp = JsonRpcErrorResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.clone().unwrap_or(RequestId::Null),
                        error: err,
                    };
                    let resp_json = serde_json::to_string(&resp)?;
                    writeln!(out, "{}", resp_json)?;
                }
            }
            out.flush()?;
        }

        Ok(())
    }

    fn handle_request(&self, req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "shutdown" => self.handle_shutdown(req),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req),
            "resources/list" => self.handle_resources_list(req),
            "resources/read" => self.handle_resources_read(req),
            "prompts/list" => self.handle_prompts_list(req),
            "prompts/get" => self.handle_prompts_get(req),
            _ => Err(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        }
    }

    fn handle_shutdown(&self, _req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        // MCP shutdown - return null result per spec
        Ok(JsonValue::Null)
    }

    fn handle_initialize(&self, req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        let _params: InitializeParams = match req
            .params
            .as_ref()
            .and_then(|p| serde_json::from_value(p.clone()).ok())
        {
            Some(p) => p,
            None => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid initialize params".to_string(),
                    data: None,
                });
            }
        };

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolCapability {}),
                resources: Some(ResourceCapability {}),
                prompts: Some(PromptCapability {}),
            },
            server_info: ServerInfo {
                name: self.server_name.clone(),
                version: self.server_version.clone(),
            },
        };
        Ok(serde_json::to_value(result).unwrap_or_default())
    }

    fn handle_tools_list(&self, _req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        Ok(json!({ "tools": self.tools }))
    }

    fn handle_tools_call(&self, req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        let params: CallToolParams = match req
            .params
            .as_ref()
            .and_then(|p| serde_json::from_value(p.clone()).ok())
        {
            Some(p) => p,
            None => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid tool call params".to_string(),
                    data: None,
                });
            }
        };

        let result_text = if let Some(executor) = &self.tool_executor {
            let args = params.arguments.unwrap_or_default();
            executor(&params.name, &args)
        } else {
            format!("Tool '{}' called (no executor configured)", params.name)
        };

        Ok(json!({
            "content": [
                { "type": "text", "text": result_text }
            ]
        }))
    }

    fn handle_resources_list(&self, _req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        Ok(json!({ "resources": self.resources }))
    }

    fn handle_resources_read(&self, req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        // Parse params: { uri: string }
        let params: serde_json::Value = match req.params.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Missing params for resources/read".to_string(),
                    data: None,
                });
            }
        };
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing or invalid 'uri' param".to_string(),
                data: None,
            })?;

        // Handle different resource types
        let contents = match uri {
            _ if uri.starts_with("file://") => {
                let path = &uri["file://".len()..];
                // Read file content (to be implemented with proper security checks)
                // For now, simulate
                vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(format!("[mock] Content of file: {}", path)),
                }]
            }
            _ if uri.starts_with("session://") => {
                let session_id = &uri["session://".len()..];
                // Would fetch session from runtime
                vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(format!("[mock] Session data for: {}", session_id)),
                }]
            }
            _ => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Unsupported URI scheme: {}", uri),
                    data: None,
                });
            }
        };

        Ok(json!({ "contents": contents }))
    }

    fn handle_prompts_list(&self, _req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        Ok(json!({ "prompts": self.prompts }))
    }

    fn handle_prompts_get(&self, req: &JsonRpcRequest) -> Result<JsonValue, JsonRpcError> {
        // Parse params: { name: string, arguments?: object }
        let params: serde_json::Value = match req.params.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Missing params for prompts/get".to_string(),
                    data: None,
                });
            }
        };
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing or invalid 'name' param".to_string(),
                data: None,
            })?;

        // Find prompt by name
        let prompt = self
            .prompts
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| JsonRpcError {
                code: -32602,
                message: format!("Prompt '{}' not found", name),
                data: None,
            })?;

        // Get arguments (if any)
        let empty_args = json!({});
        let arguments = params.get("arguments").unwrap_or(&empty_args);

        // Substitute arguments into prompt messages (simple {{var}} replacement)
        let messages = substitute_prompt_messages(&prompt.messages, arguments);

        Ok(json!({
            "description": prompt.description.as_deref(),
            "messages": messages
        }))
    }

    fn send_error(
        &self,
        out: &mut std::io::StdoutLock,
        request_line: &str,
        code: i32,
        message: String,
    ) -> std::io::Result<()> {
        let req_id = if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(request_line) {
            req.id.unwrap_or(RequestId::Null)
        } else {
            RequestId::Null
        };
        let err_resp = JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            error: JsonRpcError {
                code,
                message,
                data: None,
            },
        };
        let resp_json = serde_json::to_string(&err_resp).unwrap_or_default();
        writeln!(out, "{}", resp_json)?;
        out.flush()?;
        Ok(())
    }
}

/// Substitute {{var}} placeholders in prompt messages with provided arguments.
pub fn substitute_prompt_messages(
    messages: &[PromptMessage],
    args: &JsonValue,
) -> Vec<PromptMessage> {
    // Convert args to a map we can iterate
    let args_map = match args.as_object() {
        Some(map) => map,
        None => return messages.to_vec(), // not an object, return as-is
    };

    let mut result = Vec::new();
    for msg in messages {
        let mut new_msg = msg.clone();
        match &mut new_msg.content {
            PromptContent::Text { text } => {
                // Simple replacement: {{arg_name}} -> value (as string)
                for (key, value) in args_map {
                    let placeholder = format!("{{{{{}}}}}", key);
                    if let Some(val_str) = value.as_str() {
                        *text = text.replace(&placeholder, val_str);
                    } else {
                        *text = text.replace(&placeholder, &value.to_string());
                    }
                }
            }
            PromptContent::Resource { .. } => {
                // Resource references don't need substitution
            }
        }
        result.push(new_msg);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_registration() {
        let mut server = McpServer::new("test", "1.0");
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                }
            }),
        };
        server.register_tool(tool);
        assert_eq!(server.tools.len(), 1);
        assert_eq!(server.tools[0].name, "test_tool");
    }

    #[test]
    fn test_prompt_substitution() {
        let messages = vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "Hello, {{name}}!".to_string(),
            },
        }];
        let args = json!({ "name": "World" });
        let result = substitute_prompt_messages(&messages, &args);
        assert_eq!(result[0].content.to_text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_prompt_substitution_multiple() {
        let messages = vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "{{greeting}}, {{name}}!".to_string(),
            },
        }];
        let args = json!({ "greeting": "Hi", "name": "Alice" });
        let result = substitute_prompt_messages(&messages, &args);
        assert_eq!(result[0].content.to_text().unwrap(), "Hi, Alice!");
    }

    // Helper trait for extracting text from PromptContent
    trait PromptContentExt {
        fn to_text(&self) -> Option<&str>;
    }
    impl PromptContentExt for PromptContent {
        fn to_text(&self) -> Option<&str> {
            match self {
                PromptContent::Text { text } => Some(text),
                _ => None,
            }
        }
    }

    #[test]
    fn test_prompt_substitution_no_args() {
        let messages = vec![PromptMessage {
            role: "system".to_string(),
            content: PromptContent::Text {
                text: "You are an assistant.".to_string(),
            },
        }];
        let result = substitute_prompt_messages(&messages, &serde_json::json!({}));
        assert_eq!(
            result[0].content.to_text().unwrap(),
            "You are an assistant."
        );
    }

    #[test]
    fn test_prompt_substitution_missing_arg() {
        let messages = vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "Hello, {{name}}!".to_string(),
            },
        }];
        let args = serde_json::json!({ "role": "admin" });
        let result = substitute_prompt_messages(&messages, &args);
        assert_eq!(result[0].content.to_text().unwrap(), "Hello, {{name}}!");
    }

    #[test]
    fn test_prompt_substitution_numeric_arg() {
        let messages = vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "Count: {{count}}".to_string(),
            },
        }];
        let args = serde_json::json!({ "count": 42 });
        let result = substitute_prompt_messages(&messages, &args);
        assert_eq!(result[0].content.to_text().unwrap(), "Count: 42");
    }

    #[test]
    fn test_server_new() {
        let server = McpServer::new("MyServer", "2.0");
        assert_eq!(server.tools.len(), 0);
        assert_eq!(server.server_name, "MyServer");
    }

    #[test]
    fn test_server_with_tool_executor() {
        let mut server =
            McpServer::new("test", "1.0").with_tool_executor(Box::new(|_name, _args| {
                r#"{"content": [{"type": "text", "text": "ok"}]}"#.to_string()
            }));
        server.register_tool(McpTool {
            name: "echo".to_string(),
            description: "Echoes input".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        });
        assert_eq!(server.tools.len(), 1);
    }
}
