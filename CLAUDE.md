# CLAUDE.md - Claude Code Integration

This file helps Claude Code understand the OpenClaw project structure.

## Project Overview

OpenClaw is a Rust-based agent runtime that provides:
- MCP (Model Context Protocol) server and client
- Extensible tool system with sandbox execution
- Plugin architecture with hot reload
- Session persistence with compression
- Multi-provider support (Anthropic, OpenAI, Gemini)

## Key Directories

```
openclaw-rs/
├── crates/
│   ├── api-client/      # Multi-provider API abstraction
│   ├── runtime/         # Session management, compression
│   ├── tools/           # Tool registry, built-in tools
│   ├── plugins/         # Plugin lifecycle, hot reload
│   ├── mcp-server/     # MCP protocol server
│   ├── mcp-client/     # MCP protocol client
│   └── node-bridge/     # Node.js native bindings
├── examples/            # Usage examples
├── scripts/             # Build and test scripts
└── configs/             # Provider configurations
```

## Common Commands

```bash
# Build all
cargo build --release

# Run tests
cargo test --all

# Run specific test
cargo test -p tools

# Run benchmarks
cargo bench

# Build MCP server only
cargo build --release -p mcp-server

# Format code
cargo fmt --all

# Run clippy
cargo clippy --all
```

## Architecture Notes

### Tools
- Each tool implements the `Tool` trait
- Tools declare their `Permission` requirements
- Tools can be registered in `register_builtin_tools()`

### MCP Protocol
- Server communicates via JSON-RPC over stdio
- See `mcp_integration_test.py` for protocol examples
- Tools are exposed as MCP resources

### Plugins
- Plugins define hooks for lifecycle events
- Manifest format in `examples/plugins/`
- Use `PluginRegistry` for plugin discovery

## Environment Variables

```bash
OPENCLAW_API_KEY       # API key for LLM providers
OPENCLAW_BASE_URL      # Custom provider endpoint
OPENCLAW_SESSION_DIR   # Session persistence directory
```

## Testing

```bash
# Integration tests
python3 scripts/mcp_integration_test.py

# End-to-end tests
python3 scripts/mcp_server_e2e.py

# All tests with coverage
cargo test --all
```

## Adding New Tools

1. Create tool in `crates/tools/src/<tool_name>.rs`
2. Implement `Tool` trait
3. Register in `register_builtin_tools()` in `lib.rs`
4. Add tests
5. Update README

## Adding New Crates

1. Create in `crates/<crate-name>/`
2. Add to `Cargo.toml` workspace members
3. Add tests
4. Document in relevant README sections
