//! OpenClaw MCP Client
//!
//! Connects to an external MCP server over stdio and exposes its
//! tools, resources, and prompts as a local interface.
//!
//! Usage:
//! ```ignore
//! let mut client = McpClient::new("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]);
//! client.connect().await?;
//! let tools = client.list_tools().await?;
//! let result = client.call_tool("read_file", json!({"path": "/tmp/foo.txt"})).await?;
//! ```

use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ─── JSON-RPC Types ───────────────────────────────────────────────────────────

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    params: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
    Null,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    pub id: RequestId,
    pub result: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

// ─── MCP Protocol Types ──────────────────────────────────────────────────────

/// Server capabilities reported during initialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapability {}

/// A tool exposed by the remote MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

/// A resource exposed by the remote MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// A prompt template exposed by the remote MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

// ─── MCP Client ────────────────────────────────────────────────────────────────

/// MCP Client - connects to an external MCP server over stdio.
pub struct McpClient {
    /// Child process running the MCP server.
    child: Option<tokio::process::Child>,
    /// Stdin writer to the server process.
    stdin: Option<tokio::process::ChildStdin>,
    /// Next request ID counter.
    next_id: i64,
    /// Pending request responders (shared with reader task).
    pending: std::sync::Arc<tokio::sync::Mutex<HashMap<i64, oneshot::Sender<JsonValue>>>>,
    /// Server capabilities discovered during initialize.
    capabilities: ServerCapabilities,
    /// Server info (name + version).
    server_info: ServerInfo,
    /// Cached list of tools from the server.
    tools: Vec<Tool>,
    /// Cached list of resources from the server.
    resources: Vec<Resource>,
    /// Cached list of prompts from the server.
    prompts: Vec<Prompt>,
    /// Command to spawn (stored for builder/debugging).
    #[allow(dead_code)]
    _cmd: Vec<String>,
    /// Arguments for command.
    #[allow(dead_code)]
    _args: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("not connected to server")]
    NotConnected,
    #[error("server error: {0}")]
    Server(String),
    #[error("request timed out")]
    Timeout,
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, McpError>;

impl McpClient {
    /// Create a new MCP client that will spawn `cmd[0]` with `args`.
    ///
    /// Example:
    /// ```ignore
    /// McpClient::new("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
    /// ```
    pub fn new(cmd: &str, args: &[String]) -> Self {
        Self {
            child: None,
            stdin: None,
            next_id: 0,
            pending: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo {
                name: String::new(),
                version: String::new(),
            },
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            _cmd: vec![cmd.to_string()],
            _args: args.to_vec(),
        }
    }

    /// Connect to the MCP server, spawn the process, and perform the
    /// initialize handshake.
    pub async fn connect(&mut self) -> Result<()> {
        // Spawn the server process.
        // NOTE: We use a simplified approach here. Override spawn() for custom process setup.
        let mut child = tokio::process::Command::new(&self._cmd[0])
            .args(&self._args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().ok_or(McpError::NotConnected)?;
        let stdout = child.stdout.take().ok_or(McpError::NotConnected)?;

        self.stdin = Some(stdin);
        self.child = Some(child);

        // Start response reader.
        let pending_clone = self.pending.clone();
        let stdout = tokio::io::BufReader::new(stdout);

        tokio::spawn(async move {
            let mut lines = stdout.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    let id = match &resp.id {
                        RequestId::Number(n) => *n,
                        RequestId::String(s) => s.parse().ok().unwrap_or(-1),
                        RequestId::Null => -1,
                    };
                    let result = resp.result.unwrap_or(JsonValue::Null);
                    if let Some(p) = pending_clone.lock().await.remove(&id) {
                        let _ = p.send(result);
                    }
                }
            }
        });

        // Initialize handshake.
        self.initialize().await?;

        Ok(())
    }

    /// Perform the MCP initialize handshake.
    pub async fn initialize(&mut self) -> Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "openclaw-mcp-client",
                "version": "0.1.0"
            }
        });

        let result: JsonValue = self.request("initialize", Some(params)).await?;

        // Parse capabilities.
        if let Some(cap) = result.get("capabilities") {
            self.capabilities = serde_json::from_value(cap.clone()).unwrap_or_default();
        }
        if let Some(info) = result.get("serverInfo") {
            self.server_info = serde_json::from_value(info.clone()).unwrap_or_default();
        }

        // Send initialized notification.
        self.notify("notifications/initialized", None).await?;

        // Cache tool/resource/prompt lists.
        if self.capabilities.tools.is_some() {
            self.tools = self.list_tools().await?;
        }
        if self.capabilities.resources.is_some() {
            self.resources = self.list_resources().await?;
        }
        if self.capabilities.prompts.is_some() {
            self.prompts = self.list_prompts().await?;
        }

        Ok(())
    }

    // ── Protocol primitives ──────────────────────────────────────────────────

    /// Send a JSON-RPC request and wait for a response.
    async fn request(&mut self, method: &str, params: Option<JsonValue>) -> Result<JsonValue> {
        let stdin = self.stdin.as_mut().ok_or(McpError::NotConnected)?;

        let id = self.next_id;
        self.next_id += 1;

        let (tx, rx) = oneshot::channel();

        // Register pending response handler.
        self.pending.lock().await.insert(id, tx);

        let msg = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(id)),
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&msg)?;
        let mut line = json;
        line.push('\n');
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|_| McpError::Timeout)
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&mut self, method: &str, params: Option<JsonValue>) -> Result<()> {
        let stdin = self.stdin.as_mut().ok_or(McpError::NotConnected)?;

        let msg = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&msg)?;
        let mut line = json;
        line.push('\n');
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    // ── MCP Methods ──────────────────────────────────────────────────────────

    /// List all tools available from the server.
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        let result: JsonValue = self.request("tools/list", None).await?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| serde_json::from_value(t.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(tools)
    }

    /// Call a tool on the remote server.
    pub async fn call_tool(&mut self, name: &str, arguments: JsonValue) -> Result<JsonValue> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        self.request("tools/call", Some(params)).await
    }

    /// List all resources available from the server.
    pub async fn list_resources(&mut self) -> Result<Vec<Resource>> {
        let result: JsonValue = self.request("resources/list", None).await?;
        let resources = result
            .get("resources")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| serde_json::from_value(r.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(resources)
    }

    /// Read a resource by URI.
    pub async fn read_resource(&mut self, uri: &str) -> Result<JsonValue> {
        let params = serde_json::json!({ "uri": uri });
        self.request("resources/read", Some(params)).await
    }

    /// List all prompts available from the server.
    pub async fn list_prompts(&mut self) -> Result<Vec<Prompt>> {
        let result: JsonValue = self.request("prompts/list", None).await?;
        let prompts = result
            .get("prompts")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| serde_json::from_value(p.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(prompts)
    }

    /// Get a rendered prompt with arguments.
    pub async fn get_prompt(&mut self, name: &str, arguments: JsonValue) -> Result<JsonValue> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        self.request("prompts/get", Some(params)).await
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// Get server capabilities discovered during initialize.
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Get server info (name + version).
    pub fn server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    /// Get cached list of tools.
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// Get cached list of resources.
    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }

    /// Get cached list of prompts.
    pub fn prompts(&self) -> &[Prompt] {
        &self.prompts
    }

    /// Shut down the remote server gracefully.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _: JsonValue = self.request("shutdown", None).await?;
        self.notify("exit", None).await?;
        Ok(())
    }

    /// Kill the server process.
    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.kill();
    }
}

// ─── Builder ─────────────────────────────────────────────────────────────────

/// Builder for McpClient with custom process spawning.
pub struct McpClientBuilder {
    cmd: String,
    args: Vec<String>,
}

impl McpClientBuilder {
    pub fn new(cmd: &str) -> Self {
        Self {
            cmd: cmd.to_string(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, args: &[String]) -> Self {
        self.args.extend_from_slice(args);
        self
    }

    pub fn build(self) -> McpClient {
        McpClient::new(&self.cmd, &self.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_serialization() {
        let id = RequestId::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let id2: RequestId = serde_json::from_str("\"hello\"").unwrap();
        assert!(matches!(id2, RequestId::String(_)));
    }

    #[test]
    fn test_json_rpc_request_serde() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(1)),
            method: "tools/list".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"tools/list\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_server_capabilities_default() {
        let caps = ServerCapabilities::default();
        assert!(caps.tools.is_none());
        assert!(caps.resources.is_none());
        assert!(caps.prompts.is_none());
    }

    #[test]
    fn test_mcp_client_builder() {
        let client = McpClientBuilder::new("npx")
            .arg("-y")
            .arg("@mcp/server")
            .arg("/tmp")
            .build();
        assert_eq!(client._cmd, vec!["npx"]);
        assert_eq!(client._args, vec!["-y", "@mcp/server", "/tmp"]);
    }

    #[test]
    fn test_tool_deserialization() {
        let json =
            r#"{"name":"read_file","description":"Read a file","input_schema":{"type":"object"}}"#;
        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
    }

    #[test]
    fn test_resource_deserialization() {
        let json =
            r#"{"uri":"file:///tmp/test.txt","name":"test.txt","description":"A test file"}"#;
        let resource: Resource = serde_json::from_str(json).unwrap();
        assert_eq!(resource.uri, "file:///tmp/test.txt");
    }

    #[test]
    fn test_prompt_argument_deserialization() {
        let json = r#"{"name":"filename","description":"File to read","required":true}"#;
        let arg: PromptArgument = serde_json::from_str(json).unwrap();
        assert_eq!(arg.name, "filename");
        assert!(arg.required);
    }
}
