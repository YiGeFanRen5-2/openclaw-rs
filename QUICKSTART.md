# OpenClaw Rust Workspace — Quickstart

> Build MCP servers, Node.js bindings, and LSP tools in Rust.

## Prerequisites

- Rust 1.75+ (`rustup install stable`)
- Node.js 18+ (for Node.js bindings)
- Python 3.8+ (for e2e test scripts)

## Build Everything

```bash
cd openclaw-rs
cargo build --release
```

## Run Tests

```bash
# All tests (57 unit tests)
cargo test --all

# Clippy lints
cargo clippy --all

# Benchmarks
cargo bench --all
```

## MCP Server

```bash
# Build
cargo build --release -p mcp-server

# Run
./target/release/mcp-server

# CLI tools
./target/release/mcp-server --version
./target/release/mcp-server --list-tools
./target/release/mcp-server --tool-info read_file

# Test with Claude Desktop → add to ~/Library/Application Support/Claude/claude_desktop_config.json:
# See CLAUDE.md for full instructions
```

## Node.js Bindings

```bash
cargo build --release -p openclaw-node-bridge

node -e "
const { ProviderMode, OpenClawRuntime } = require('./target/release/openclaw_node_bridge.node');
const rt = new OpenClawRuntime(ProviderMode.Mock, null, null, 'mock-v1');
console.log(rt.listTools());
"
```

## Project Structure

```
openclaw-rs/
├── crates/
│   ├── api-client/      # Multi-provider LLM abstraction
│   ├── runtime/         # Session, tools, sandbox, compression
│   ├── tools/           # Tool registry + built-in tools
│   ├── harness/         # LSP client + sandbox
│   ├── mcp-server/      # MCP protocol server (stdio)
│   ├── mcp-client/      # MCP client
│   ├── node-bridge/     # N-API Node.js bindings
│   ├── openclaw-core/   # Core tool library
│   └── plugins/         # Plugin hot reload (libloading)
├── examples/            # example_01_session, example_02_lsp, example_03_runtime
└── scripts/             # e2e test scripts
```

## E2E Test Scripts

```bash
# MCP server (stdio protocol)
python3 scripts/mcp_server_e2e.py

# MCP client → server
python3 scripts/mcp_client_server_test.py

# Node.js bindings
node scripts/validate_node_bindings.js
```

## Key Crates

| Crate | Purpose |
|-------|---------|
| `mcp-server` | MCP protocol server for Claude Desktop |
| `runtime` | Session management, tools, sandbox |
| `tools` | Tool registry + execution framework |
| `harness` | LSP client + sandbox |
| `node-bridge` | Native Node.js addon (`.node`) |
| `plugins` | Hot-reload plugins via `libloading` |
| `api-client` | LLM provider abstraction (OpenAI, Anthropic, etc.) |

## Common Tasks

### Add a new tool

1. Define in `crates/tools/src/builtin/`
2. Register in `crates/tools/src/lib.rs`
3. Add to `McpServer` tool list in `bin/mcp-server.rs`

### Add a new MCP method

1. Add handler in `crates/mcp-server/src/lib.rs` → `handle_request()`
2. Add to capability in `initialize` response

### Add a new LSP feature

1. Implement in `crates/harness/src/lib.rs` → `LspClient`

## Troubleshooting

```bash
# Stuck build?
cargo clean && cargo build --release

# Clippy warnings
cargo clippy --fix --lib -p <crate>

# Reset to clean state
git checkout -- . && git clean -fd
```
