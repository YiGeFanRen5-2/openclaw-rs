# Quick Reference

Essential commands and patterns for OpenClaw.

## Installation

```bash
# Clone
git clone https://github.com/YiGeFanRen5-2/openclaw-rs.git
cd openclaw-rs

# Build
cargo build --release

# Install
cargo install --path crates/openclaw-cli
```

## CLI Commands

```bash
# Help
openclaw --help

# Status
openclaw status

# Demo
openclaw demo --message "Hello"

# REPL
openclaw repl --model gpt-4

# Tools
openclaw tools

# Version
openclaw version -a

# Health
openclaw health

# Metrics
openclaw metrics --prometheus
```

## Docker

```bash
# Build image
docker build -f Dockerfile.mcp -t openclaw/mcp-server:latest .

# Run
docker run --rm openclaw/mcp-server:latest

# Docker Compose
docker compose up mcp-server
```

## MCP Server

```bash
# Start server
./target/release/mcp-server

# With API key
OPENCLAW_API_KEY=sk-... ./target/release/mcp-server
```

## Rust API

### Basic Usage

```rust
use openclaw_core::{Runtime, MockProvider};

let runtime = Runtime::builder()
    .provider(MockProvider::new("mock-v1".into()))
    .build()?;

runtime.create_session("my-session".into())?;
runtime.add_message("my-session".into(), "user", "Hello")?;
let response = runtime.chat("my-session".into(), "Hi", None).await?;
```

### Tool Execution

```rust
let result = runtime.execute_tool(
    "my-session".into(),
    "list_files",
    r#"{"path": "/tmp"}"#
).await?;
```

### Session Persistence

```rust
runtime.persist_session("my-session".into())?;
runtime.restore_session("my-session".into())?;
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENCLAW_API_KEY` | API key for providers | - |
| `OPENCLAW_BASE_URL` | Custom provider URL | - |
| `OPENCLAW_SESSION_DIR` | Session storage dir | `./sessions` |

## Configuration

 TOML config file:

```toml
[provider]
type = "anthropic"
api_key = "sk-..."

[provider]
type = "openai"
api_key = "sk-..."

[sandbox]
enabled = true
max_memory = "512MB"

[persistence]
enabled = true
path = "./sessions"
```

## Common Patterns

### Custom Tool

```rust
struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &'static str { "my_tool" }
    fn description(&self) -> &'static str { "Does something" }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        Ok(json!({ "result": "done" }))
    }
}

registry.register(MyTool);
```

### Plugin Registration

```rust
let mut registry = PluginRegistry::new();
registry.register_from_file("my-plugin/plugin.json")?;
```

## Troubleshooting

```bash
# Check version
openclaw version

# Run tests
cargo test --all

# Integration test
python3 scripts/mcp_integration_test.py

# Build with debug
cargo build

# Verbose output
RUST_LOG=debug cargo run
```

## Links

- [Documentation](https://github.com/YiGeFanRen5-2/openclaw-rs#readme)
- [API Docs](./docs/api/)
- [Examples](./examples/)
- [CHANGELOG](./CHANGELOG.md)
