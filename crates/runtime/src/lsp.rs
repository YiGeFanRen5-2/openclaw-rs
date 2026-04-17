//! LSP Bridge - Integrates harness LSP Client into OpenClaw Runtime
//!
//! Exposes LSP features (completions, hover, goto-definition, etc.)
//! as OpenClaw tools that can be called from sessions.

use harness::{
    CompletionItem, Diagnostic, DocumentSymbol, HoverInfo, Location, LspClient, WorkspaceSymbol,
};
use serde::Deserialize;

/// LSP bridge - wraps harness::LspClient for use in OpenClaw tools.
#[derive(Debug, Default)]
pub struct LspBridge {
    /// Server name (e.g. "rust-analyzer", "pyright")
    pub server_name: String,
    /// Command to launch the LSP server
    pub server_cmd: Vec<String>,
    /// Active LSP client (None before connect)
    client: Option<LspClient>,
}

impl LspBridge {
    /// Create a new LSP bridge for a language server.
    pub fn new(server_name: &str, server_cmd: Vec<String>) -> Self {
        Self {
            server_name: server_name.to_string(),
            server_cmd,
            client: None,
        }
    }

    /// Launch the LSP server and initialize it.
    pub async fn connect(&mut self, root_uri: &str) -> Result<(), LspError> {
        let mut client = LspClient::new(&self.server_name, &self.server_cmd);
        client.connect_stdio().await.map_err(LspError::from)?;
        let root = std::path::Path::new(root_uri);
        client.initialize(root).await.map_err(LspError::from)?;
        self.client = Some(client);
        Ok(())
    }

    /// Get the internal client (for advanced use).
    pub fn client_mut(&mut self) -> Option<&mut LspClient> {
        self.client.as_mut()
    }

    /// Take ownership of the internal client.
    pub fn take_client(&mut self) -> Option<LspClient> {
        self.client.take()
    }

    // ── LSP Tool Implementations ────────────────────────────────────────────────

    /// Open a document in the LSP server.
    pub async fn did_open(
        &mut self,
        uri: &str,
        language_id: &str,
        text: &str,
    ) -> Result<(), LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client
            .did_open(uri, language_id, text)
            .await
            .map_err(LspError::from)
    }

    /// Get completions at a cursor position.
    pub async fn completions(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client
            .get_completions(uri, line, character)
            .await
            .map_err(LspError::from)
    }

    /// Get hover info at a cursor position.
    pub async fn hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverInfo>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client
            .get_hover(uri, line, character)
            .await
            .map_err(LspError::from)
    }

    /// Go to definition at a cursor position.
    pub async fn goto_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client
            .goto_definition(uri, line, character)
            .await
            .map_err(LspError::from)
    }

    /// Find all references to a symbol at a cursor position.
    pub async fn find_references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client
            .find_references(uri, line, character)
            .await
            .map_err(LspError::from)
    }

    /// Get document symbols (outline) for a file.
    pub async fn document_symbols(&mut self, uri: &str) -> Result<Vec<DocumentSymbol>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client.document_symbols(uri).await.map_err(LspError::from)
    }

    /// Search for symbols across the workspace.
    pub async fn workspace_symbol(
        &mut self,
        query: &str,
    ) -> Result<Vec<WorkspaceSymbol>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        client.workspace_symbol(query).await.map_err(LspError::from)
    }

    /// Get current diagnostics for a file.
    pub async fn diagnostics(&mut self, uri: &str) -> Result<Vec<Diagnostic>, LspError> {
        let client = self.client.as_mut().ok_or(LspError::NotConnected)?;
        Ok(client.get_diagnostics(uri).await)
    }

    /// Shut down the LSP server gracefully.
    pub async fn shutdown(&mut self) -> Result<(), LspError> {
        if let Some(client) = self.client.as_mut() {
            client.shutdown().await.map_err(LspError::from)?;
        }
        Ok(())
    }

    /// Kill the LSP server process.
    pub fn kill(&mut self) {
        if let Some(client) = self.client.as_mut() {
            client.kill();
        }
    }
}

impl LspBridge {
    /// Create a default bridge configured for rust-analyzer.
    pub fn rust_analyzer() -> Self {
        Self::new("rust-analyzer", vec!["rust-analyzer".to_string()])
    }

    /// Create a default bridge configured for pyright.
    pub fn pyright() -> Self {
        Self::new(
            "pyright",
            vec!["pyright".to_string(), "--langserver".to_string()],
        )
    }

    /// Create a default bridge configured for the TypeScript language server.
    pub fn tsserver() -> Self {
        Self::new(
            "tsserver",
            vec![
                "typescript-language-server".to_string(),
                "--stdio".to_string(),
            ],
        )
    }
}

/// LSP errors propagated to tool results.
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("not connected to LSP server")]
    NotConnected,
    #[error("LSP error: {0}")]
    Server(String),
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),
}

impl From<harness::LspError> for LspError {
    fn from(e: harness::LspError) -> Self {
        match e {
            harness::LspError::NotConnected => LspError::NotConnected,
            harness::LspError::Timeout(d) => LspError::Timeout(d),
            harness::LspError::ServerError(s) => LspError::Server(s),
            harness::LspError::MethodNotSupported => {
                LspError::Server("method not supported".to_string())
            }
            harness::LspError::Parse(_) | harness::LspError::Io(_) => {
                LspError::Server(e.to_string())
            }
        }
    }
}

// ── Tool input/output types for OpenClaw tool registry ────────────────────────

/// Tool input for LSP completions.
#[derive(Debug, Deserialize)]
pub struct CompletionsInput {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

/// Tool input for LSP hover.
#[derive(Debug, Deserialize)]
pub struct HoverInput {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

/// Tool input for LSP goto definition.
#[derive(Debug, Deserialize)]
pub struct GotoDefInput {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

/// Tool input for LSP find references.
#[derive(Debug, Deserialize)]
pub struct FindRefsInput {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

/// Tool input for LSP document symbols.
#[derive(Debug, Deserialize)]
pub struct DocSymbolsInput {
    pub uri: String,
}

/// Tool input for LSP workspace symbol search.
#[derive(Debug, Deserialize)]
pub struct WorkspaceSymbolInput {
    pub query: String,
}

/// Tool input for LSP diagnostics.
#[derive(Debug, Deserialize)]
pub struct DiagnosticsInput {
    pub uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_bridge_creation() {
        let bridge = LspBridge::new("rust-analyzer", vec!["rust-analyzer".to_string()]);
        assert_eq!(bridge.server_name, "rust-analyzer");
        assert_eq!(bridge.server_cmd, vec!["rust-analyzer"]);
        assert!(bridge.client.is_none());
    }

    #[test]
    fn test_lsp_bridge_rust_analyzer_default() {
        let bridge = LspBridge::rust_analyzer();
        assert_eq!(bridge.server_name, "rust-analyzer");
    }

    #[test]
    fn test_lsp_error_from_harness() {
        let err = LspError::from(harness::LspError::NotConnected);
        assert!(matches!(err, LspError::NotConnected));
    }

    #[test]
    fn test_completions_input_parse() {
        let json = r#"{"uri": "file:///src/main.rs", "line": 10, "character": 5}"#;
        let input: CompletionsInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.uri, "file:///src/main.rs");
        assert_eq!(input.line, 10);
        assert_eq!(input.character, 5);
    }

    #[test]
    fn test_lsp_bridge_pyright_default() {
        let bridge = LspBridge::pyright();
        assert_eq!(bridge.server_name, "pyright");
        assert!(bridge.server_cmd.contains(&"--langserver".to_string()));
    }
}
