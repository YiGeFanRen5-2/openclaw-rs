//! LSP Tool Adapters — expose LSP methods as OpenClaw tools.
//!
//! Each LSP method becomes a `Tool` that wraps `harness::LspClient`.
//! The bridge runs in-process; async calls are blocked on using
//! `tokio::runtime::Handle::current().block_on()`.

use harness::{
    CompletionItem, Diagnostic, DocumentSymbol, FormattingOptions, HoverInfo, Location,
    LspClient, LspError, TextEdit, WorkspaceEdit, WorkspaceSymbol,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error as ThisError;
use tokio::runtime::Handle;
use tools::{Permission, Tool, ToolError, ToolSchema};

/// Result type for LSP tool operations.
type ToolResult<T> = std::result::Result<T, LspToolError>;

#[derive(Debug, ThisError)]
pub enum LspToolError {
    #[error("LSP error: {0}")]
    Lsp(#[from] LspError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),
    #[error("runtime error: {0}")]
    Runtime(String),
}

/// LSP server configuration parsed from tool arguments.
#[derive(Debug, Deserialize)]
pub struct LspConfig {
    /// Command to launch the LSP server, e.g. ["rust-analyzer"] or ["pyright", "--langserver"].
    pub server_cmd: Vec<String>,
    /// Root URI for the LSP server (usually the workspace root).
    #[serde(default)]
    pub root_uri: Option<String>,
}

/// Position parsed from tool arguments.
#[derive(Debug, Deserialize)]
pub struct LspPosition {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset within the line.
    pub character: u32,
}

/// Arguments common to position-based LSP tools.
#[derive(Debug, Deserialize)]
pub struct LspPositionArgs {
    /// The document URI, e.g. "file:///path/to/file.rs".
    pub document_uri: String,
    /// Cursor position.
    pub position: LspPosition,
}

/// Arguments for document-based LSP tools.
#[derive(Debug, Deserialize)]
pub struct LspDocumentArgs {
    /// The document URI.
    pub document_uri: String,
    /// Language ID (e.g. "rust", "python", "typescript").
    #[serde(default)]
    pub language_id: Option<String>,
    /// Document text content (used for did_open).
    #[serde(default)]
    pub text: Option<String>,
}

/// LSP tool response wrappers for consistent JSON output.

#[derive(Debug, Serialize)]
pub struct CompletionResponse {
    pub completions: Vec<CompletionItemPayload>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CompletionItemPayload {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}

impl From<CompletionItem> for CompletionItemPayload {
    fn from(item: CompletionItem) -> Self {
        Self {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            documentation: item.documentation,
            insert_text: item.insert_text,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HoverResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<harness::Range>,
}

impl From<Option<HoverInfo>> for HoverResponse {
    fn from(info: Option<HoverInfo>) -> Self {
        match info {
            Some(h) => HoverResponse {
                contents: Some(h.contents),
                range: h.range,
            },
            None => HoverResponse {
                contents: None,
                range: None,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LocationResponse {
    pub locations: Vec<LocationPayload>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct LocationPayload {
    pub uri: String,
    pub range: harness::Range,
}

impl From<Location> for LocationPayload {
    fn from(loc: Location) -> Self {
        Self {
            uri: loc.uri,
            range: loc.range,
        }
    }
}

impl From<Vec<Location>> for LocationResponse {
    fn from(locs: Vec<Location>) -> Self {
        let count = locs.len();
        Self {
            locations: locs.into_iter().map(LocationPayload::from).collect(),
            count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DocumentSymbolsResponse {
    pub symbols: Vec<DocumentSymbolPayload>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct DocumentSymbolPayload {
    pub name: String,
    pub kind: u32,
    pub range: harness::Range,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<DocumentSymbolPayload>,
}

impl From<DocumentSymbol> for DocumentSymbolPayload {
    fn from(ds: DocumentSymbol) -> Self {
        Self {
            name: ds.name,
            kind: ds.kind,
            range: ds.range,
            children: ds.children.into_iter().map(DocumentSymbolPayload::from).collect(),
        }
    }
}

impl From<Vec<DocumentSymbol>> for DocumentSymbolsResponse {
    fn from(symbols: Vec<DocumentSymbol>) -> Self {
        let count = symbols.len();
        Self {
            symbols: symbols.into_iter().map(DocumentSymbolPayload::from).collect(),
            count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSymbolsResponse {
    pub symbols: Vec<WorkspaceSymbolPayload>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSymbolPayload {
    pub name: String,
    pub kind: u32,
    pub uri: String,
    pub range: harness::Range,
}

impl From<WorkspaceSymbol> for WorkspaceSymbolPayload {
    fn from(ws: WorkspaceSymbol) -> Self {
        Self {
            name: ws.name,
            kind: ws.kind,
            uri: ws.location.uri,
            range: ws.location.range,
        }
    }
}

impl From<Vec<WorkspaceSymbol>> for WorkspaceSymbolsResponse {
    fn from(symbols: Vec<WorkspaceSymbol>) -> Self {
        let count = symbols.len();
        Self {
            symbols: symbols.into_iter().map(WorkspaceSymbolPayload::from).collect(),
            count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnosticsResponse {
    pub diagnostics: Vec<DiagnosticPayload>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticPayload {
    pub range: harness::Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub message: String,
}

impl From<Diagnostic> for DiagnosticPayload {
    fn from(d: Diagnostic) -> Self {
        Self {
            range: d.range,
            severity: d.severity,
            code: d.code,
            source: d.source,
            message: d.message,
        }
    }
}

impl From<Vec<Diagnostic>> for DiagnosticsResponse {
    fn from(diags: Vec<Diagnostic>) -> Self {
        let count = diags.len();
        Self {
            diagnostics: diags.into_iter().map(DiagnosticPayload::from).collect(),
            count,
        }
    }
}

// ─── LSP Bridge ───────────────────────────────────────────────────────────────

/// Wraps a `harness::LspClient` so it can be shared across tools.
/// Each tool holds an `Arc<LspBridge>` and calls `block_on` for async ops.
#[derive(Clone)]
pub struct LspBridge {
    client: Arc<tokio::sync::Mutex<LspClient>>,
}

impl LspBridge {
    /// Create a new bridge (client not yet connected).
    pub fn new(server_name: &str, server_cmd: &[String]) -> Self {
        Self {
            client: Arc::new(tokio::sync::Mutex::new(LspClient::new(server_name, server_cmd))),
        }
    }

    /// Connect to the LSP server over stdio.
    pub async fn connect_stdio(&mut self) -> std::result::Result<(), LspError> {
        self.client.lock().await.connect_stdio().await
    }

    /// Initialize the LSP session.
    pub async fn initialize(&mut self, root_uri: &std::path::Path) -> std::result::Result<JsonValue, LspError> {
        self.client.lock().await.initialize(root_uri).await
    }

    /// Notify the server that a document was opened.
    pub async fn did_open(
        &mut self,
        uri: &str,
        language_id: &str,
        text: &str,
    ) -> std::result::Result<(), LspError> {
        self.client.lock().await.did_open(uri, language_id, text).await
    }

    /// Notify the server of document changes.
    pub async fn did_change(&mut self, uri: &str, changes: &[harness::TextChange]) -> std::result::Result<(), LspError> {
        self.client.lock().await.did_change(uri, changes).await
    }

    /// Get completions at the given position.
    pub async fn get_completions(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> std::result::Result<Vec<CompletionItem>, LspError> {
        self.client.lock().await.get_completions(uri, line, character).await
    }

    /// Get hover info at the given position.
    pub async fn get_hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> std::result::Result<Option<HoverInfo>, LspError> {
        self.client.lock().await.get_hover(uri, line, character).await
    }

    /// Go to definition.
    pub async fn goto_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> std::result::Result<Vec<Location>, LspError> {
        self.client.lock().await.goto_definition(uri, line, character).await
    }

    /// Find all references.
    pub async fn find_references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> std::result::Result<Vec<Location>, LspError> {
        self.client.lock().await.find_references(uri, line, character).await
    }

    /// Get document symbols.
    pub async fn document_symbols(&mut self, uri: &str) -> std::result::Result<Vec<DocumentSymbol>, LspError> {
        self.client.lock().await.document_symbols(uri).await
    }

    /// Search workspace symbols.
    pub async fn workspace_symbol(&mut self, query: &str) -> std::result::Result<Vec<WorkspaceSymbol>, LspError> {
        self.client.lock().await.workspace_symbol(query).await
    }

    /// Get diagnostics for a document.
    pub async fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        self.client.lock().await.get_diagnostics(uri).await
    }

    /// Shutdown the LSP server.
    pub async fn shutdown(&mut self) -> std::result::Result<(), LspError> {
        self.client.lock().await.shutdown().await
    }
}

// ─── Macro to reduce boilerplate for LSP tools ───────────────────────────────

/// Helper: extract a field from JSON or return a tool error.
fn get_str<'a>(obj: &'a serde_json::Map<String, JsonValue>, key: &str) -> Result<&'a str, LspToolError> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LspToolError::Runtime(format!("missing or non-string field: {}", key)))
}

fn get_u64(obj: &serde_json::Map<String, JsonValue>, key: &str) -> Result<u64, LspToolError> {
    obj.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| LspToolError::Runtime(format!("missing or non-integer field: {}", key)))
}

// ─── LSP Tool implementations ────────────────────────────────────────────────

macro_rules! define_lsp_tool {
    (
        $name:ident,
        $desc:expr,
        $input_schema:expr,
        $output_schema:expr,
        async $method:ident ( $bridge:ident , $($arg_name:ident : $arg_ty:ty),* )
        -> $ret:ty
        $body:expr
    ) => {
        pub struct $name {
            bridge: LspBridge,
        }

        impl $name {
            pub fn new(bridge: LspBridge) -> Self {
                Self { bridge }
            }
        }

        impl Tool for $name {
            fn name(&self) -> &'static str {
                concat!("lsp_", stringify!($name).strip_prefix("Lsp").map(|s| s.to_lowercase()).unwrap_or_default().as_str())
            }

            fn description(&self) -> &'static str {
                $desc
            }

            fn input_schema(&self) -> ToolSchema {
                $input_schema
            }

            fn output_schema(&self) -> ToolSchema {
                $output_schema
            }

            fn permission(&self) -> Permission {
                Permission::Safe
            }

            fn execute(&self, input: JsonValue) -> std::result::Result<JsonValue, ToolError> {
                let rt = Handle::current();
                rt.block_on(async {
                    let result = self.$method(input).await;
                    result
                })
            }
        }

        impl $name {
            async fn $method(&self, input: JsonValue) -> std::result::Result<JsonValue, ToolError> {
                let $bridge = &self.bridge;
                let result: $ret = $body;
                Ok(serde_json::to_value(result).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
            }
        }
    };
}

/// lsp_complete — get completions at cursor position.
pub struct LspCompleteTool {
    bridge: LspBridge,
}

impl LspCompleteTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str {
        "lsp_complete"
    }

    fn description_impl(&self) -> &'static str {
        "Request completion items at a cursor position from the LSP server."
    }

    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Get LSP completions at a cursor position".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string", "description": "Document URI (file:///path)" },
                "line": { "type": "integer", "description": "Zero-based line number" },
                "character": { "type": "integer", "description": "Zero-based character offset" },
            })),
            required: Some(vec!["document_uri".into(), "line".into(), "character".into()]),
        }
    }

    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Completion response".into()),
            properties: Some(serde_json::json!({
                "completions": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["completions".into(), "count".into()]),
        }
    }
}

impl Tool for LspCompleteTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;
            let line = input
                .get("line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer line".into()))?;
            let character = input
                .get("character")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer character".into()))?;

            let items = self
                .bridge
                .get_completions(uri, line, character)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response = CompletionResponse {
                completions: items.into_iter().map(CompletionItemPayload::from).collect(),
                count: 0, // filled below
            };
            let count = response.completions.len();
            let response = CompletionResponse { count, completions: response.completions };
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_hover — get hover info at cursor position.
pub struct LspHoverTool {
    bridge: LspBridge,
}

impl LspHoverTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str {
        "lsp_hover"
    }

    fn description_impl(&self) -> &'static str {
        "Request hover documentation at a cursor position from the LSP server."
    }

    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Get LSP hover info at a cursor position".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string" },
                "line": { "type": "integer" },
                "character": { "type": "integer" },
            })),
            required: Some(vec!["document_uri".into(), "line".into(), "character".into()]),
        }
    }

    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Hover response".into()),
            properties: Some(serde_json::json!({
                "contents": { "type": ["string", "null"] },
                "range": { "type": ["object", "null"] },
            })),
            required: Some(vec!["contents".into()]),
        }
    }
}

impl Tool for LspHoverTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;
            let line = input
                .get("line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer line".into()))?;
            let character = input
                .get("character")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer character".into()))?;

            let info = self
                .bridge
                .get_hover(uri, line, character)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response: HoverResponse = info.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_goto_def — go to definition at cursor position.
pub struct LspGotoDefTool {
    bridge: LspBridge,
}

impl LspGotoDefTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str { "lsp_goto_def" }
    fn description_impl(&self) -> &'static str {
        "Jump to the definition of the symbol at a cursor position."
    }
    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Go to LSP definition".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string" },
                "line": { "type": "integer" },
                "character": { "type": "integer" },
            })),
            required: Some(vec!["document_uri".into(), "line".into(), "character".into()]),
        }
    }
    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Definition locations".into()),
            properties: Some(serde_json::json!({
                "locations": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["locations".into(), "count".into()]),
        }
    }
}

impl Tool for LspGotoDefTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;
            let line = input
                .get("line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer line".into()))?;
            let character = input
                .get("character")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer character".into()))?;

            let locs = self
                .bridge
                .goto_definition(uri, line, character)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response: LocationResponse = locs.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_find_refs — find all references to the symbol at cursor.
pub struct LspFindRefsTool {
    bridge: LspBridge,
}

impl LspFindRefsTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str { "lsp_find_refs" }
    fn description_impl(&self) -> &'static str {
        "Find all references to the symbol at a cursor position."
    }
    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Find LSP references".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string" },
                "line": { "type": "integer" },
                "character": { "type": "integer" },
            })),
            required: Some(vec!["document_uri".into(), "line".into(), "character".into()]),
        }
    }
    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Reference locations".into()),
            properties: Some(serde_json::json!({
                "locations": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["locations".into(), "count".into()]),
        }
    }
}

impl Tool for LspFindRefsTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;
            let line = input
                .get("line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer line".into()))?;
            let character = input
                .get("character")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| ToolError::InvalidInput("missing or non-integer character".into()))?;

            let locs = self
                .bridge
                .find_references(uri, line, character)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response: LocationResponse = locs.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_document_symbols — list all symbols in a document.
pub struct LspDocumentSymbolsTool {
    bridge: LspBridge,
}

impl LspDocumentSymbolsTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str { "lsp_document_symbols" }
    fn description_impl(&self) -> &'static str {
        "List all document symbols (functions, structs, etc.) from the LSP server."
    }
    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Get LSP document symbols".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string" },
            })),
            required: Some(vec!["document_uri".into()]),
        }
    }
    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Document symbols".into()),
            properties: Some(serde_json::json!({
                "symbols": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["symbols".into(), "count".into()]),
        }
    }
}

impl Tool for LspDocumentSymbolsTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;

            let symbols = self
                .bridge
                .document_symbols(uri)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response: DocumentSymbolsResponse = symbols.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_workspace_symbol — search symbols across the workspace.
pub struct LspWorkspaceSymbolTool {
    bridge: LspBridge,
}

impl LspWorkspaceSymbolTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str { "lsp_workspace_symbol" }
    fn description_impl(&self) -> &'static str {
        "Search for symbols (functions, types, etc.) across the entire workspace."
    }
    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Search LSP workspace symbols".into()),
            properties: Some(serde_json::json!({
                "query": { "type": "string", "description": "Search query (e.g. \"validate\" searches all symbols matching 'validate')" },
            })),
            required: Some(vec!["query".into()]),
        }
    }
    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Workspace symbol results".into()),
            properties: Some(serde_json::json!({
                "symbols": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["symbols".into(), "count".into()]),
        }
    }
}

impl Tool for LspWorkspaceSymbolTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let query = input
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing query".into()))?;

            let symbols = self
                .bridge
                .workspace_symbol(query)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let response: WorkspaceSymbolsResponse = symbols.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

/// lsp_diagnostics — get current diagnostics for a document.
pub struct LspDiagnosticsTool {
    bridge: LspBridge,
}

impl LspDiagnosticsTool {
    pub fn new(bridge: LspBridge) -> Self {
        Self { bridge }
    }

    fn name_impl(&self) -> &'static str { "lsp_diagnostics" }
    fn description_impl(&self) -> &'static str {
        "Get current LSP diagnostics (errors, warnings, hints) for a document."
    }
    fn input_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Get LSP diagnostics".into()),
            properties: Some(serde_json::json!({
                "document_uri": { "type": "string" },
            })),
            required: Some(vec!["document_uri".into()]),
        }
    }
    fn output_schema_impl() -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Diagnostics".into()),
            properties: Some(serde_json::json!({
                "diagnostics": { "type": "array" },
                "count": { "type": "integer" },
            })),
            required: Some(vec!["diagnostics".into(), "count".into()]),
        }
    }
}

impl Tool for LspDiagnosticsTool {
    fn name(&self) -> &'static str { self.name_impl() }
    fn description(&self) -> &'static str { self.description_impl() }
    fn input_schema(&self) -> ToolSchema { Self::input_schema_impl() }
    fn output_schema(&self) -> ToolSchema { Self::output_schema_impl() }
    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let rt = Handle::current();
        rt.block_on(async {
            let uri = input
                .get("document_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("missing document_uri".into()))?;

            let diags = self
                .bridge
                .get_diagnostics(uri)
                .await;

            let response: DiagnosticsResponse = diags.into();
            Ok(serde_json::to_value(response).map_err(|e| ToolError::InvalidInput(e.to_string()))?)
        })
    }
}

// ─── LSP Tool Registration Helper ────────────────────────────────────────────

/// Register all LSP tools with the runtime.
pub fn register_lsp_tools(runtime: &mut crate::Runtime, bridge: LspBridge) -> Result<(), crate::RuntimeError> {
    runtime.register_tool(Box::new(LspCompleteTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspHoverTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspGotoDefTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspFindRefsTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspDocumentSymbolsTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspWorkspaceSymbolTool::new(bridge.clone())))?;
    runtime.register_tool(Box::new(LspDiagnosticsTool::new(bridge.clone())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_response_serialization() {
        let response = CompletionResponse {
            completions: vec![
                CompletionItemPayload {
                    label: "Vec::new".to_string(),
                    kind: Some(1),
                    detail: Some("fn new() -> Vec<T>".to_string()),
                    documentation: None,
                    insert_text: Some("Vec::new()".to_string()),
                },
            ],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"label\":\"Vec::new\""));
        assert!(json.contains("\"count\":1"));
    }

    #[test]
    fn test_location_response_serialization() {
        let response = LocationResponse {
            locations: vec![
                LocationPayload {
                    uri: "file:///src/main.rs".to_string(),
                    range: harness::Range {
                        start: harness::Position { line: 10, character: 0 },
                        end: harness::Position { line: 10, character: 5 },
                    },
                },
            ],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"count\":1"));
        assert!(json.contains("file:///src/main.rs"));
    }

    #[test]
    fn test_diagnostics_response_serialization() {
        let response = DiagnosticsResponse {
            diagnostics: vec![
                DiagnosticPayload {
                    range: harness::Range {
                        start: harness::Position { line: 5, character: 0 },
                        end: harness::Position { line: 5, character: 15 },
                    },
                    severity: Some(1),
                    code: Some(serde_json::json!("E0432")),
                    source: Some("rustc".to_string()),
                    message: "cannot find type `Foo` in this scope".to_string(),
                },
            ],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("cannot find type"));
        assert!(json.contains("E0432"));
    }

    #[test]
    fn test_document_symbols_response_serialization() {
        let response = DocumentSymbolsResponse {
            symbols: vec![
                DocumentSymbolPayload {
                    name: "fn main".to_string(),
                    kind: 12, // Function
                    range: harness::Range {
                        start: harness::Position { line: 0, character: 0 },
                        end: harness::Position { line: 10, character: 1 },
                    },
                    children: vec![],
                },
            ],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("fn main"));
        assert!(json.contains("\"count\":1"));
    }

    #[test]
    fn test_workspace_symbols_response_serialization() {
        let response = WorkspaceSymbolsResponse {
            symbols: vec![
                WorkspaceSymbolPayload {
                    name: "Validator".to_string(),
                    kind: 5, // Class
                    uri: "file:///src/lib.rs".to_string(),
                    range: harness::Range {
                        start: harness::Position { line: 20, character: 0 },
                        end: harness::Position { line: 20, character: 9 },
                    },
                },
            ],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Validator"));
        assert!(json.contains("file:///src/lib.rs"));
    }

    #[test]
    fn test_hover_response_serialization() {
        let response: HoverResponse = Some(HoverInfo {
            contents: "```rust\nfn foo()\n```".to_string(),
            range: None,
        })
        .into();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("fn foo"));
    }

    #[test]
    fn test_hover_response_null_serialization() {
        let response: HoverResponse = None.into();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("null"));
    }
}
