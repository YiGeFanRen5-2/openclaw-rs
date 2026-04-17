# OpenClaw Rust Workspace

Production-ready Rust implementation of the OpenClaw agent runtime.

## Features

- **High Performance**: Rust-based runtime, 10-100x faster than Node.js for CPU-intensive tasks
- **Type Safety**: Full Rust type system, compile-time error checking
- **Extensible**: Plugin system with hot reload support
- **Multi-Provider**: Anthropic, OpenAI, Gemini support
- **MCP Ready**: Full Model Context Protocol implementation

## Quick Start

```bash
# Clone the repository
git clone https://github.com/YiGeFanRen5-2/openclaw-rs.git
cd openclaw-rs

# Build MCP server
cargo build --release -p mcp-server

# Run tests
cargo test --all
```

## Architecture

```
openclaw-rs/
├── crates/
│   ├── api-client/      # Multi-provider API abstraction
│   ├── runtime/         # Session management & compression
│   ├── tools/           # Tool framework & built-in tools
│   ├── plugins/         # Plugin lifecycle & hot reload
│   ├── mcp-server/      # MCP protocol server
│   ├── mcp-client/      # MCP protocol client
│   └── node-bridge/     # Node.js native bindings
└── examples/            # Usage examples
```

## Built-in Tools

| Tool | Description |
|------|-------------|
| `list_files` | List directory contents |
| `read_file` | Read file contents |
| `write_file` | Write file contents |
| `edit_file` | Edit file (exact replace) |
| `http_request` | HTTP GET/POST requests |
| `file_info` | Get file metadata |
| `image_info` | Get image dimensions/format |
| `health_check` | Service health check |
| `batch_health_check` | Batch health check |
| `validate_json` | JSON Schema validation |
| `sql_query` | SQLite query (coming soon) |

## Documentation

- [Installation Guide](guide/installation.md)
- [Getting Started](guide/getting-started.md)
- [Configuration](guide/configuration.md)
- [Tools Guide](guide/tools.md)
- [Plugins Guide](guide/plugins.md)

## License

MIT License - see [LICENSE](LICENSE)
