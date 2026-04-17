# API Overview

High-level overview of the OpenClaw API architecture.

## Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Application                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     CLI / REPL                             │
│  (claw-cli: commands, interactive mode)                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      MCP Client                            │
│  (Connect to external MCP servers)                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     MCP Server                            │
│  (Expose tools via MCP protocol)                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Runtime                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Session   │  │   Tools     │  │   Plugins   │    │
│  │  Management  │  │   Registry  │  │   Manager    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   API Client                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  Anthropic  │  │   OpenAI    │  │   Gemini     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Crate Dependencies

```
openclaw-cli
    └── openclaw-core
            ├── runtime
            │       ├── api-client
            │       └── tools
            ├── tools
            ├── plugins
            ├── mcp-server
            ├── mcp-client
            └── node-bridge
```

## Key Types

### Runtime

```rust
pub struct Runtime {
    provider: Box<dyn Provider>,
    session_manager: SessionManager,
    tool_registry: ToolRegistry,
    plugin_manager: PluginManager,
    metrics: MetricsCollector,
}
```

### Provider

```rust
pub trait Provider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<StreamingResponse, ProviderError>;
}
```

### Tool

```rust
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError>;
}
```

## Async Runtime

OpenClaw uses Tokio for async operations:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let runtime = Runtime::builder()
        .provider(AnthropicProvider::new()?)
        .tools(registry)
        .build()?;

    runtime.create_session("my-session").await?;
    // ...
    Ok(())
}
```
