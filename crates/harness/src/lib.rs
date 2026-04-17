//! # OpenClaw LSP Client Harness
//!
//! A lightweight LSP (Language Server Protocol) client for connecting OpenClaw
//! to language servers (rust-analyzer, pyright, tsserver, etc.) over stdio.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::RwLock;
use tokio::time::Duration;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LspMessage {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u32>,
    pub code: Option<Value>,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("not connected")]
    NotConnected,
    #[error("server error: {0}")]
    ServerError(String),
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("method not supported by server")]
    MethodNotSupported,
}

pub type Result<T> = std::result::Result<T, LspError>;

// ─── LSP Client ──────────────────────────────────────────────────────────────

/// A Language Server Protocol client over stdio.
#[derive(Debug)]
pub struct LspClient {
    #[allow(dead_code)]
    server_name: String,
    child: Option<Child>,
    stdin: Option<tokio::process::ChildStdin>,
    next_id: Arc<RwLock<u64>>,
    pending: Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Value>>>>,
    diagnostics: Arc<RwLock<HashMap<String, Vec<Diagnostic>>>>,
    server_cmd: Vec<String>,
}

impl LspClient {
    /// Create a new LSP client for a language server.
    ///
    /// `cmd` = ["rust-analyzer"] or ["pyright", "--langserver"].
    pub fn new(server_name: &str, cmd: &[String]) -> Self {
        Self {
            server_name: server_name.to_string(),
            child: None,
            stdin: None,
            next_id: Arc::new(RwLock::new(0)),
            pending: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
            server_cmd: cmd.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Connect to the language server over stdio.
    pub async fn connect_stdio(&mut self) -> Result<()> {
        let mut child = TokioCommand::new(&self.server_cmd[0])
            .args(&self.server_cmd[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(LspError::Io)?;

        let child_stdin = child.stdin.take().ok_or(LspError::NotConnected)?;
        let child_stdout = child.stdout.take().ok_or(LspError::NotConnected)?;

        self.stdin = Some(child_stdin);
        self.child = Some(child);

        // Start the reader loop
        let pending = Arc::clone(&self.pending);
        let diagnostics = Arc::clone(&self.diagnostics);

        tokio::spawn(async move {
            let reader = BufReader::new(child_stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(msg) = serde_json::from_str::<LspMessage>(&line) {
                    // Route response to waiting request
                    if let Some(id) = msg.id {
                        if let Some(id_num) = id.as_u64() {
                            if let Some(tx) = pending.write().await.remove(&id_num) {
                                let _ = tx.send(msg.result.unwrap_or_default());
                            }
                        }
                    }
                    // Route notification
                    if let Some(method) = &msg.method {
                        if method == "textDocument/publishDiagnostics" {
                            if let Some(params) = msg.params {
                                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                                    if let Ok(mut diags) = diagnostics.try_write() {
                                        let list: Vec<Diagnostic> = serde_json::from_value(
                                            params.get("diagnostics").cloned().unwrap_or_default(),
                                        )
                                        .unwrap_or_default();
                                        diags.insert(uri.to_string(), list);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn initialize(&mut self, root_uri: &Path) -> Result<Value> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri.to_str(),
            "capabilities": {}
        });
        self.request("initialize", Some(params)).await
    }

    pub async fn did_open(&mut self, uri: &str, language_id: &str, text: &str) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri, "languageId": language_id, "text": text }
        });
        self.notify("textDocument/didOpen", Some(params)).await
    }

    pub async fn did_change(&mut self, uri: &str, changes: &[TextChange]) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "contentChanges": changes
        });
        self.notify("textDocument/didChange", Some(params)).await
    }

    pub async fn did_save(&mut self, uri: &str, text: Option<&str>) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "text": text
        });
        self.notify("textDocument/didSave", Some(params)).await
    }

    pub async fn did_close(&mut self, uri: &str) -> Result<()> {
        let params = serde_json::json!({ "textDocument": { "uri": uri } });
        self.notify("textDocument/didClose", Some(params)).await
    }

    pub async fn get_completions(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        let result = self
            .request("textDocument/completion", Some(params))
            .await?;
        if let Some(items) = result.as_array() {
            Ok(items
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect())
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverInfo>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        let result = self.request("textDocument/hover", Some(params)).await?;
        if result.is_null() {
            return Ok(None);
        }
        let contents = result.get("contents").cloned().unwrap_or_default();
        let markdown = if let Some(s) = contents.as_str() {
            s.to_string()
        } else if let Some(obj) = contents.as_object() {
            obj.get("value")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };
        Ok(Some(HoverInfo {
            contents: markdown,
            range: serde_json::from_value(result.get("range").cloned().unwrap_or_default()).ok(),
        }))
    }

    pub async fn goto_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
        });
        let result = self
            .request("textDocument/definition", Some(params))
            .await?;
        Self::parse_locations(result)
    }

    pub async fn find_references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": false }
        });
        let result = self
            .request("textDocument/references", Some(params))
            .await?;
        Self::parse_locations(result)
    }

    pub async fn document_symbols(&mut self, uri: &str) -> Result<Vec<DocumentSymbol>> {
        let params = serde_json::json!({ "textDocument": { "uri": uri } });
        let result = self
            .request("textDocument/documentSymbol", Some(params))
            .await?;
        serde_json::from_value(result).or_else(|_| Ok(vec![]))
    }

    pub async fn workspace_symbol(&mut self, query: &str) -> Result<Vec<WorkspaceSymbol>> {
        let params = serde_json::json!({ "query": query });
        let result = self.request("workspace/symbol", Some(params)).await?;
        serde_json::from_value(result).or_else(|_| Ok(vec![]))
    }

    pub async fn format_document(
        &mut self,
        uri: &str,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "options": options,
        });
        let result = self
            .request("textDocument/formatting", Some(params))
            .await?;
        serde_json::from_value(result).or_else(|_| Ok(vec![]))
    }

    pub async fn rename(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "newName": new_name,
        });
        let result = self.request("textDocument/rename", Some(params)).await?;
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(serde_json::from_value(result)?))
        }
    }

    pub async fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        self.diagnostics
            .read()
            .await
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.request("shutdown", None).await?;
        self.notify("exit", None).await?;
        Ok(())
    }

    /// Kill the child process.
    #[allow(dead_code, unused_must_use)]
    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.kill();
        }
    }

    // ── Internal ───────────────────────────────────────────────────────────────

    async fn send_raw(&mut self, msg: &LspMessage) -> Result<()> {
        let stdin = self.stdin.as_mut().ok_or(LspError::NotConnected)?;
        let json = serde_json::to_string(msg)?;
        let mut line = json;
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(LspError::Io)?;
        stdin.flush().await.map_err(LspError::Io)?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = {
            let mut n = self.next_id.write().await;
            let id = *n;
            *n += 1;
            id
        };

        let msg = LspMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(id.into())),
            method: Some(method.to_string()),
            params,
            result: None,
            error: None,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.write().await.insert(id, tx);

        self.send_raw(&msg).await?;

        tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| LspError::Timeout(Duration::from_secs(30)))?
            .map_err(|_| LspError::ServerError("response channel closed".to_string()))
    }

    async fn notify(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let msg = LspMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method.to_string()),
            params,
            result: None,
            error: None,
        };
        self.send_raw(&msg).await
    }

    fn parse_locations(result: Value) -> Result<Vec<Location>> {
        let arr = result.as_array().cloned().unwrap_or_default();
        let mut locs = vec![];
        for item in arr {
            if let (Some(uri), Ok(range)) = (
                item.get("uri").and_then(|v| v.as_str()),
                serde_json::from_value::<Range>(item.get("range").cloned().unwrap_or_default()),
            ) {
                locs.push(Location {
                    uri: uri.to_string(),
                    range,
                });
            }
        }
        Ok(locs)
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.kill();
    }
}

// ─── Supporting types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    pub range: Option<Range>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: u32,
    pub range: Range,
    pub children: Vec<DocumentSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: u32,
    pub location: Location,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsp_client_creation() {
        let client = LspClient::new("rust-analyzer", &["rust-analyzer".to_string()]);
        assert_eq!(client.server_name, "rust-analyzer");
        assert!(client.child.is_none());
        assert!(client.server_cmd == vec!["rust-analyzer"]);
    }

    #[test]
    fn test_lsp_error_display() {
        let err = LspError::NotConnected;
        assert_eq!(err.to_string(), "not connected");
    }
}
