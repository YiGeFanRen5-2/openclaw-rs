# Project Summary - OpenClaw Rust Workspace

> Production-ready Rust implementation of the OpenClaw agent runtime

## Overview

**Repository**: https://github.com/YiGeFanRen5-2/openclaw-rs  
**Version**: 0.3.0  
**License**: MIT

## What is OpenClaw?

OpenClaw is a high-performance Rust-based agent runtime that provides:

- **MCP (Model Context Protocol)** server and client implementation
- **Extensible tool system** with sandbox execution
- **Plugin architecture** with hot reload support
- **Session persistence** with automatic compression
- **Multi-provider support** (Anthropic, OpenAI, Gemini, Mock)

## Architecture

```
openclaw-rs/
├── crates/
│   ├── api-client/       # Multi-provider API abstraction
│   ├── runtime/         # Session management & compression
│   ├── tools/           # Tool registry & built-in tools
│   ├── plugins/         # Plugin lifecycle & hot reload
│   ├── mcp-server/     # MCP protocol server (stdio)
│   ├── mcp-client/     # MCP protocol client
│   ├── node-bridge/     # Node.js native bindings (N-API)
│   ├── openclaw-core/   # Core runtime types
│   ├── harness/         # LSP client harness
│   └── ffi/            # C FFI layer
├── examples/            # Usage examples
├── scripts/            # Build & test scripts
├── configs/            # Provider configurations
└── docs/               # Documentation
```

## Features

### Built-in Tools (19 total)

| Category | Tools |
|----------|-------|
| File Operations | `list_files`, `read_file`, `write_file`, `edit_file`, `file_info` |
| HTTP | `http_request` |
| Image | `image_info`, `image_formats` |
| Health | `health_check`, `batch_health_check` |
| Validation | `validate_json`, `validate_tool_input` |
| Storage | `json_store_set`, `json_store_get`, `json_store_list` |
| Text | `hash`, `uuid`, `random_string`, `text_stats` |

### MCP Protocol

Full MCP server implementation supporting:
- `tools/list`, `tools/call`
- `resources/list`, `resources/read`
- `prompts/list`, `prompts/get`
- `initialize`, `shutdown`

### Plugin System

- Manifest-based plugin discovery
- Semantic version validation
- Hook pipeline (before/after tool calls, messages)
- Hot reload support

### Performance

| Operation | Result |
|-----------|--------|
| `session_new` | ~1.2µs (target: <1ms) ✅ |
| `session_token_count` | ~7.3ns (target: <10ns) ✅ |
| `provider_config_serialize` | ~150ns ✅ |

## Getting Started

```bash
# Clone
git clone https://github.com/YiGeFanRen5-2/openclaw-rs.git
cd openclaw-rs

# Build
cargo build --release

# Run MCP server
./target/release/mcp-server

# Run tests
cargo test --all

# List tools
cargo run --example example_05_complete_workflow
```

## Docker

```bash
# Build MCP server image
docker build -f Dockerfile.mcp -t openclaw/mcp-server:latest .

# Run
docker run --rm openclaw/mcp-server:latest
```

## Documentation

| Document | Description |
|----------|-------------|
| [README.md](./README.md) | Main documentation |
| [QUICKSTART.md](./QUICKSTART.md) | Quick start guide |
| [FFI.md](./FFI.md) | Node.js API reference |
| [CLAUDE.md](./CLAUDE.md) | Claude Code integration |
| [SECURITY.md](./SECURITY.md) | Security policy |
| [PERFORMANCE.md](./PERFORMANCE.md) | Performance optimization |
| [BENCHMARKS.md](./BENCHMARKS.md) | Benchmark results |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | Developer guide |

## Testing

```bash
# Unit tests
cargo test --all

# Integration tests
python3 scripts/mcp_integration_test.py

# Benchmarks
cargo bench
```

## Metrics

| Metric | Value |
|--------|-------|
| Built-in tools | 19 |
| Unit tests | 77+ |
| Examples | 6 |
| Documentation files | 16+ |
| Dockerfiles | 3 |
| CI jobs | 5 |

## Changelog

See [CHANGELOG.md](./CHANGELOG.md) for detailed version history.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup and guidelines.

## License

MIT License - see [LICENSE](./LICENSE)
