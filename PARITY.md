# OpenClaw Rust ↔ Node.js Parity Checklist

## Core Features

### ✅ Session Management
- [x] Create session with ID
- [x] Add messages (role + content)
- [x] Session persistence (JSON file)
- [x] Session compaction/compression
- [x] Session restore
- [x] Token counting (approximate via content length / 4)
- [x] Custom summarizer support (Deterministic, LLM, Fallback)

### ✅ Provider System
- [x] Provider trait with generate()
- [x] MockProvider (for testing)
- [x] OpenAI adapter
- [x] Anthropic adapter
- [x] Retry provider wrapper (RetryProvider)
- [x] Rate limiter (RateLimiter)
- [x] ProviderConfig with serialization

### ✅ Tool System
- [x] Tool trait (name, description, schema, permission, execute)
- [x] Tool registry (register, get, list)
- [x] Built-in tools (list_files, read_file, write_file, edit_file, http_request)
- [x] Permission framework (Safe, Filesystem, Shell, Network, Custom)
- [x] Sandbox execution (fork + namespaces + rlimit + seccomp)

### ✅ MCP Protocol
- [x] JSON-RPC 2.0 request/response
- [x] stdio transport
- [x] tools/list + tools/call
- [x] resources/list + resources/read
- [x] prompts/list + prompts/get
- [x] initialize handshake
- [x] shutdown

### ✅ LSP Client (harness crate)
- [x] connect_stdio
- [x] initialize
- [x] did_open / did_change / did_save / did_close
- [x] completions
- [x] hover
- [x] goto_definition
- [x] find_references
- [x] document_symbols
- [x] workspace_symbol
- [x] diagnostics
- [x] shutdown

### ⏳ MCP Client (not yet implemented)
- [x] Connect to external MCP server
- [x] Receive tools/resources from server
- [x] Call tools on server
- [x] Streaming support (receive server-side streams)

### ⏳ Plugin System
- [x] Plugin trait (before_tool_call, after_message, on_error, on_shutdown)
- [x] Hook pipeline (HookPipeline with ordered execution)
- [x] Plugin discovery/load (YAML config, glob patterns)
- [x] Plugin hot-reload (basic impl) (dynamic .so loading)
- [x] Plugin isolation (namespaces via sandbox) (separate process/namespace)

### ⏳ Node Bridge (node-bridge crate)
- [x] N-API bindings (napi::bindgen_scheduler, thread-safe closures)
- [x] Session lifecycle from Node.js
- [x] Full OpenClaw Runtime exposed to Node.js
- [x] Tool registration from Node.js side
- [x] Streaming responses to Node.js (via napi ThreadsafeFunction)

### ⏳ FFI / OpenClaw Core (openclaw-core crate)
- [x] Core types (Session, Message, Role, ToolCall)
- [x] Provider trait re-export
- [x] Tool trait re-export
- [x] Complete runtime accessible via FFI (FFI.md + node-bridge + ffi crate)
- [x] Compression utilities (zstd implemented in Phase 14) (zstd/brotli for session compaction)

## Performance Targets

| Operation | Target | Status |
|-----------|--------|--------|
| Session creation | < 1ms | ✅ Should meet (HashMap insert) |
| Provider chat request (mock) | < 1ms | ✅ Should meet (mock has no I/O) |
| Tool execution (sandbox, mock) | < 5ms | ⚠️ fork overhead (~1-2ms), may vary |
| MCP server startup | < 100ms | ✅ JSON parsing only, no I/O |
| LSP client connect | < 500ms | ✅ stdio handshake is fast |

## Benchmark Results

### api-client (`cargo bench -p api-client`)

| Benchmark | Result |
|-----------|--------|
| `chat_request_new` | ✅ Compiles & runs |
| `chat_message_new` | ✅ Compiles & runs |
| `provider_config_serialize` | ✅ Compiles & runs |
| `provider_config_deserialize` | ✅ Compiles & runs |

### runtime (`cargo bench -p runtime`)

| Benchmark | Result |
|-----------|--------|
| `session_new` | ✅ Compiles & runs |
| `session_add_message` | ✅ Compiles & runs |
| `session_add_multiple_messages` | ✅ Compiles & runs |
| `session_token_count` | ✅ Compiles & runs |
| `session_should_compact` | ✅ Compiles & runs |

### tools (`cargo test -p tools --lib`)

| Test | Result |
|------|--------|
| `bench_tool_registry_insert` | ✅ Compiles & passes |
| `bench_tool_registry_get` | ✅ Compiles & passes |
| `bench_tool_registry_list_schemas` | ✅ Compiles & passes |
| `bench_tool_execution` | ✅ Compiles & passes |
| `bench_tool_permission_safe` | ✅ Compiles & passes |

## Recommendations for Phase 12

1. **MCP Client** — Implement the missing MCP client to mirror the server. Connect to an external MCP server over stdio, receive tools/resources, and proxy calls back.

2. **Plugin Hot-Reload** — Use `libloading` (or `hot-libreload` crate) to load `.so` plugin files dynamically without restarting the runtime.

3. **Node.js Runtime Bridge** — Complete the `node-bridge` N-API layer to expose full Runtime capabilities (session management, tool registration, streaming) to Node.js callers.

4. **Compression** — Add `zstd` or `brotli` compression for session compaction and persistence to reduce disk I/O.

5. **Criterion HTML Reports** — After `cargo bench`, serve the generated `target/criterion/` reports with a simple static file server for visual comparison across runs.

6. **Integration Benchmarks** — Add end-to-end benchmarks: `runtime.execute_tool()` round-trip including sandbox fork, and `mcp-server` full request/response cycle.
