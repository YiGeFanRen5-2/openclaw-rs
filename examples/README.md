# OpenClaw Examples

This directory contains examples demonstrating how to use the OpenClaw Rust Workspace.

## Examples

### Core Examples

| Example | Description | Prerequisites |
|---------|-------------|---------------|
| `example_01_session.rs` | Basic session creation and management | None |
| `example_02_lsp.rs` | Using the LSP client for editor integration | None |
| `example_03_runtime.rs` | Runtime initialization and configuration | None |
| `example_04_node_integration.js` | Node.js integration with native bindings | Build node bindings |
| `example_05_complete_workflow.rs` | Complete workflow: tools, metrics, persistence | All features |
| `example_06_mcp_client.rs` | MCP client connecting to MCP server | Build mcp-server |
| `example_07_plugin_example.rs` | Plugin development with lifecycle hooks and state sharing | None |
| `example_08_http_client.rs` | HTTP client with GET/POST/PUT/DELETE, JSON, retries | None |
| `example_09_batch_processor.rs` | Concurrent batch processing with semaphore and progress | None |
| `example_10_mcp_server.rs` | MCP server providing tools, resources, and prompts | None |

### Plugin Examples

| Path | Description |
|------|-------------|
| `plugins/example-tool/` | Example plugin with manifest |
| `plugins/example-weather-plugin/` | Weather plugin example |

## Running Examples

### Build First

```bash
# Build all crates
cargo build --release

# Build specific examples
cargo build --release -p examples
```

### Run Examples

```bash
# Example 1: Session
cargo run --example example_01_session

# Example 5: Complete Workflow
cargo run --example example_05_complete_workflow

# Example 6: MCP Client (requires running server)
./target/release/mcp-server &
cargo run --example example_06_mcp_client
```

## Example Descriptions

### example_01_session.rs
Demonstrates basic session operations:
- Creating a session
- Adding messages
- Getting session info
- Deleting sessions

### example_02_lsp.rs
Shows how to use the LSP client:
- Initialize LSP connection
- Send document changes
- Handle diagnostics

### example_03_runtime.rs
Runtime configuration:
- Building a runtime with custom config
- Provider selection
- Tool registration

### example_04_node_integration.js
Node.js integration via native bindings:
- Loading the native module
- Creating runtime
- Calling tools

### example_05_complete_workflow.rs
Complete workflow example:
- Metrics collection
- Tool execution
- Session persistence
- All features combined

### example_06_mcp_client.rs
MCP protocol client:
- JSON-RPC over stdio
- Server lifecycle
- Tool listing and calling

### example_07_plugin_example.rs
Plugin system demonstration:
- Two demo plugins: `MetricsTrackerPlugin` and `SanitizerPlugin`
- Hook pipeline: Before/AfterToolCall, Before/AfterMessage, OnSessionStart/End, OnLoad/Unload
- `PluginManager` with permission checking, hot reload, and unload
- State sharing via `HookContext::metadata` (Arc<RwLock<HashMap>>)

### example_08_http_client.rs
HTTP client with the `http_request` tool:
- GET, POST, PUT, DELETE, PATCH methods
- JSON body, form body, custom headers
- Timeout configuration and retry with backoff
- Error handling for malformed requests

### example_09_batch_processor.rs
Batch task processing:
- Configurable concurrency via semaphore
- Per-task timeout and error aggregation
- Progress bar and atomic counters
- 3 demo batches: pure tool calls, mixed types, error recovery

### example_10_mcp_server.rs
MCP server implementation:
- JSON-RPC 2.0 over stdio (Claude Desktop compatible)
- 5 built-in tools, custom resources, prompts with variable substitution
- Demo mode for interactive testing

## Plugin Development

See `plugins/` directory for plugin examples and manifest format.

## Adding New Examples

1. Create `example_NN_name.rs` in this directory
2. Document in this README (no extra Cargo.toml needed — examples are auto-discovered)

## Phase 54: Additional Examples

### example_07_plugin_example.rs
Plugin development full example:
- Implements the `Plugin` trait with `#[async_trait]`
- Registers lifecycle hooks: `BeforeToolCall`, `AfterToolCall`, `BeforeMessage`, `AfterMessage`, `OnSessionStart`, `OnSessionEnd`, `OnLoad`, `OnUnload`
- `HookContext::metadata` for inter-hook state sharing
- `PluginManager` with allowlist, permission checking, hot reload, and unload
- Two demo plugins: `MetricsTrackerPlugin` (tracks counts) and `SanitizerPlugin` (redacts sensitive fields)
```bash
cargo run --example example_07_plugin_example
```

### example_08_http_client.rs
HTTP client using the built-in `http_request` tool:
- GET, POST, PUT, DELETE requests to httpbin.org
- JSON and form-encoded request bodies
- Custom headers (Authorization, Content-Type, User-Agent)
- Timeout configuration (1s timeout on slow endpoints)
- Error handling for invalid URLs and unsupported methods
- Retry logic with `with_retry` helper (attempts, delay, backoff)
- Session-based tool call history tracking
```bash
cargo run --example example_08_http_client
```

### example_09_batch_processor.rs
Batch processing with concurrency control:
- `BatchProcessor` struct with configurable concurrency (semaphore-based)
- `tokio::spawn` for parallel task execution
- Each task gets isolated session to avoid concurrency conflicts
- Atomic counters for shared metrics (succeeded/failed counts)
- Timeout per task via `tokio::time::timeout`
- Progress tracking (updates every 5 tasks)
- Barrier synchronization for clean completion
- Error aggregation and result sorting by task ID
- Demonstrates 3 batch scenarios: pure tool calls, mixed tool types, error recovery
```bash
cargo run --example example_09_batch_processor
```

### example_10_mcp_server.rs
MCP server exposing OpenClaw tools via the Model Context Protocol:
- JSON-RPC 2.0 over stdio (the Claude Desktop / Cursor integration protocol)
- Registers 5 tools: `read_file`, `text_stats`, `uuid`, `hash`, `random_string`
- Custom resources: `session://current` (live session data), file URIs
- Prompts with variable substitution: `code_review` (with {{code}} placeholder), `写作助手` (Chinese writing assistant with {{主题}} substitution)
- Two modes: `demo` (simulates a full client conversation) and `server` (stdio mode for Claude Desktop)
- Claude Desktop config example provided in file comments
```bash
# Interactive demo (simulates client conversation)
cargo run --example example_10_mcp_server -- demo

# Stdio server (for Claude Desktop integration)
cargo run --example example_10_mcp_server
```

#### Claude Desktop Integration
Add to `~/.config/Claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "openclaw": {
      "command": "cargo",
      "args": ["run", "--example", "example_10_mcp_server"],
      "cwd": "/path/to/openclaw-rs"
    }
  }
}
```
