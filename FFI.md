# OpenClaw FFI / Node.js API Reference

Complete reference for the OpenClaw Rust runtime exposed to Node.js via N-API.

## Installation

```bash
# Build the native addon
cd openclaw-rs
cargo build --release -p openclaw-node-bridge

# The .node file will be at:
# target/release/libopenclaw_node_bridge.node
```

## Node.js Usage

```javascript
const { OpenClawRuntime, ProviderMode } = require('./target/release/libopenclaw_node_bridge.node');

// Create runtime with mock provider
const rt = new OpenClawRuntime(ProviderMode.Mock, null, null, "mock-v1");

// ── Session Management ─────────────────────────────────────────────────────

// Create a session
rt.createSession("my-session");

// Add messages
rt.addMessage("my-session", "user", "Hello, how are you?");
rt.addMessage("my-session", "assistant", "I'm doing well!");

// List sessions
const sessions = rt.listSessions();
console.log("Active sessions:", sessions);

// Get session as JSON
const sessionJson = rt.getSession("my-session");
console.log(JSON.parse(sessionJson));

// ── Tool Execution ─────────────────────────────────────────────────────────

// List available tools
const tools = rt.listTools();
console.log("Tools:", tools);

// Execute a tool
const result = rt.executeTool("my-session", "read_file", '{"path": "/tmp/test.txt"}');
console.log("Tool result:", result);

// ── Session Persistence ───────────────────────────────────────────────────

// Set persist path
rt.setSessionStore("/tmp/openclaw-sessions");

// Persist a session
rt.persistSession("my-session");

// Restore a session
rt.restoreSession("my-session");

// Compact (compress) a session
rt.compactSession("my-session");

// ── LSP Integration ────────────────────────────────────────────────────────

// Initialize LSP bridge (e.g. rust-analyzer)
rt.lspInit("rust-analyzer", ["rust-analyzer"]);

// Connect to LSP server
rt.lspConnect("file:///project");

// Open a document
rt.lspDidOpen("file:///project/src/main.rs", "rust", "fn main() {}");

// Get completions
const completions = rt.lspCompletions("file:///project/src/main.rs", 0, 3);
console.log("Completions:", completions);

// Get hover info
const hover = rt.lspHover("file:///project/src/main.rs", 0, 3);
console.log("Hover:", hover);

// Go to definition
const defs = rt.lspGotoDefinition("file:///project/src/main.rs", 0, 3);
console.log("Definitions:", defs);

// Find references
const refs = rt.lspFindReferences("file:///project/src/main.rs", 0, 3);
console.log("References:", refs);

// Document symbols (outline)
const symbols = rt.lspDocumentSymbols("file:///project/src/main.rs");
console.log("Symbols:", symbols);

// Workspace symbol search
const wsSymbols = rt.lspWorkspaceSymbol("main");
console.log("Workspace symbols:", wsSymbols);

// Get diagnostics
const diagnostics = rt.lspDiagnostics("file:///project/src/main.rs");
console.log("Diagnostics:", diagnostics);

// Shutdown LSP server
rt.lspShutdown();

// ── JS Tool Registration ─────────────────────────────────────────────────

// Register a custom tool from JavaScript
rt.registerTool("my_tool", "Does something useful", JSON.stringify({
  type: "object",
  properties: {
    input: { type: "string", description: "The input" }
  },
  required: ["input"]
}));

// ── Runtime Status ────────────────────────────────────────────────────────

const status = rt.runtimeStatus();
console.log("Status:", status); // { provider, session_store, tools_count, lsp_bridge }

// ── Cleanup ────────────────────────────────────────────────────────────────

rt.shutdown();
```

## MCP Client (connecting to external MCP servers)

```javascript
// Note: MCP client is available as a standalone binary or module
// Build with: cargo build --release -p mcp-client

// The MCP client can connect to external MCP servers via stdio:
// npx @modelcontextprotocol/server-filesystem /tmp
// python -m mcp_server.my_server
```

## Tool Registry

Built-in tools registered by default:

| Tool | Description | Permission |
|------|-------------|------------|
| `list_files` | List directory contents | Filesystem |
| `read_file` | Read file contents | Filesystem |
| `write_file` | Write file contents | Filesystem |
| `edit_file` | Edit file with patch | Filesystem |
| `http_request` | Make HTTP requests | Network |

## Provider Modes

```javascript
const { ProviderMode } = require('./libopenclaw_node_bridge.node');

ProviderMode.Mock      // Local mock (no API calls)
ProviderMode.Openai     // OpenAI API
ProviderMode.Anthropic  // Anthropic API (Claude)
ProviderMode.Gemini     // Google Gemini API
```

## Session Format (JSON)

```json
{
  "id": "my-session",
  "messages": [
    { "role": "user", "content": "Hello", "timestamp": "..." },
    { "role": "assistant", "content": "Hi!", "timestamp": "..." }
  ],
  "metadata": {},
  "created_at": "...",
  "updated_at": "..."
}
```

## Rust FFI Exports

The `ffi` crate exposes these types to Node.js:

| Symbol | Type | Description |
|--------|------|-------------|
| `OpenClawRuntime` | napi struct | Main runtime handle |
| `SessionHandle` | napi struct | Lightweight session reference |
| `ProviderMode` | napi enum | Provider type selector |
| `create_openclaw_runtime` | fn | FFI factory |
| `openclaw_create_session` | fn | Create session |
| `openclaw_execute_tool` | fn | Execute tool |

## Performance Notes

- Session creation: < 1ms (HashMap insert)
- Tool execution: 1-5ms (includes sandbox fork)
- LSP operations: Network latency + LSP server processing
- Session compaction: < 10ms for typical sessions
- Compressed session load: ~2-3x faster than uncompressed JSON parse
