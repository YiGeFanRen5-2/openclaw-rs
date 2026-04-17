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

## Plugin Development

See `plugins/` directory for plugin examples and manifest format.

## Adding New Examples

1. Create `example_NN_name.rs` in this directory
2. Add to `examples` path in `Cargo.toml`
3. Document in this README
