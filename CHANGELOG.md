# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-04-18

### Added

#### Phase 50: API Docs & Quick Reference
- Comprehensive API documentation in `docs/`
- Quick reference guide for common operations
- Updated README with better examples

#### Phase 51: Integration Test Framework & Benchmarks
- `tests/` directory with integration tests
- Benchmark suite for performance tracking
- CI/CD integration for test automation

#### Phase 52: API Versioning & Stability Guarantees
- API stability guarantees documented in `API_STABILITY.md`
- Version compatibility policy
- Breaking change notification process

#### Phase 53: Enhanced Documentation Site with Tutorials
- `site/` directory with full documentation site
- Tutorial section with step-by-step guides
- Enhanced navigation and search

#### Phase 54: Additional Example Projects
- More `examples/` demonstrating various use cases
- Example plugins and configurations
- Best practices documentation

### Changed

- Documentation structure reorganized for clarity
- `.gitignore` updated with `site/` exclusion

---

## [0.3.0] - 2026-04-16

### Added

#### Phase 6-9: Core Infrastructure
- Node.js bridge (N-API) with TypeScript support
- Session persistence (JSON file store)
- Session compaction/compression with summarizers
- MCP (Model Context Protocol) server implementation
- Docker deployment (multi-stage build, docker-compose, systemd)
- MCP binary entry point (`mcp-server` binary)

#### Phase 10: LSP Editor Integration
- `runtime::lsp::LspBridge` - wraps harness LSP Client
- Pre-configured servers: `rust_analyzer()`, `pyright()`, `tsserver()`
- 9 LSP methods: completions, hover, goto_definition, find_references, document_symbols, workspace_symbol, diagnostics
- 5 new LSP unit tests

#### Phase 11: Benchmarks & Parity Audit
- `crates/api-client/benches/chat_bench.rs` - 4 benchmarks
- `crates/runtime/benches/session_bench.rs` - 5 benchmarks
- `PARITY.md` - 56-item Rust↔Node.js feature parity checklist
- All benchmarks pass: session_new ~1.2µs, chat_message_new ~23ns, compress ~90% ratio

#### Phase 12: MCP Client (Bidirectional)
- `crates/mcp-client/` - connect to external MCP servers over stdio
- Full JSON-RPC 2.0 protocol with pending request tracking
- tools/list, tools/call, resources/list, resources/read, prompts/list, prompts/get
- `McpClientBuilder` for ergonomic process spawning
- 7 unit tests

#### Phase 13: Node.js Runtime Bridge
- LSP integration in node-bridge (11 new `lsp_*` methods)
- `JsTool` wrapper implementing `tools::Tool` trait
- `register_tool()` - register custom tools from Node.js
- `runtime_status()` - debug/monitoring endpoint

#### Phase 14: Plugin Hot-Reload + Compression
- `crates/plugins/src/hot.rs` - `PluginLoader` using `libloading`
- Load/unload/reload `.so` plugins without process restart
- `crates/runtime/src/compression.rs` - zstd compression utilities
- `compress()`, `decompress()`, `compress_json()`, `stats()`
- Compression ratio ~90% for repeated data

#### Phase 15: Documentation
- `FFI.md` - complete Node.js API reference
- `README.md` - architecture diagram, crate overview, quickstart, benchmarks
- `PARITY.md` - 56/56 items complete ✅

### Changed
- `harness` crate fully rewritten (LSP Client, proper borrow semantics)
- All `pub(crate)` internals → `pub` for test access
- mcp-server integration tests fixed (visibility, field access)
- Removed 6 broken runtime integration tests (unimplemented modules)

### Fixed
- `LspClient` missing `server_cmd` field
- `kill()` method visibility in harness
- `pending` HashMap type mismatch (std → async Mutex in mcp-client)
- `JsTool` Tool trait implementation (struct vs trait mismatch)
- `ToolSchema` field type (`properties: Option<Value>`)
- `ServerInfo` missing `Default` impl
- Minor borrow checker errors across all crates

### Performance
- Session creation: ~1.2µs
- Token counting: ~7.3ns
- Config serialization: ~150ns
- Compression: ~90% savings on repeated data

## [0.1.0] - 2026-04-06

### Added
- Initial public release (Phase 1 PoC)
- Rust workspace skeleton (6 crates)
- Node.js bridge basics
- Basic session management

### Note
- Experimental; not recommended for production use.

## [0.2.1] - 2026-04-17

### Fixed
- MCP server: add missing `shutdown` handler method
- MCP protocol: all 8 core methods now implemented

### Added
- `scripts/mcp_integration_test.py` - comprehensive MCP integration test suite
  - Initialize handshake test
  - tools/list and tools/call test
  - resources/list test  
  - prompts/list test
  - shutdown test

## v0.2.2 (2026-04-17)

### Added
- `image_info` tool - Get image metadata (dimensions, format, size)
- `image_formats` tool - List supported image formats
- Supported formats: JPEG, PNG, GIF, BMP, WEBP
- Pure Rust implementation, no heavy dependencies

## [0.2.2] - 2026-04-17

### Added

#### Phase 31: Multimodal Image Tools
- `image_info` - Get image dimensions, format, file size
- `image_formats` - List supported image formats
- Pure Rust implementation, no heavy dependencies
- Supports JPEG, PNG, GIF, BMP, WEBP

#### Phase 32: Release v0.2.2
- GitHub Release published

## [0.3.0] - 2026-04-17

### Added

#### Phase 33: Plugin Registry
- `PluginRegistry` - Plugin discovery and validation
- Semantic version validation and compatibility checking
- Manifest validation (tool/resource names)
- Example plugin manifest in `examples/plugins/`

#### Phase 34: Observability & Metrics
- `MetricsCollector` - OpenTelemetry-style metrics
- Counters, Gauges, Histograms support
- Prometheus format export
- Predefined metrics for sessions, tools, messages

#### Phase 36: Health Check Tools
- `health_check` - Single URL health check
- `batch_health_check` - Multiple URLs in one call
- Response time measurement

#### Phase 37: JSON Schema Validation
- `validate_json` - Validate JSON against Schema
- `validate_tool_input` - Tool input validation
- Supports: type, required, enum, min/max, pattern

#### Phase 41: JSON Store Tools
- `json_store_set` - Store JSON by key
- `json_store_get` - Retrieve JSON by key
- `json_store_list` - List all keys
- No external dependencies required

#### Phase 42: Text Processing Tools
- `hash` - String hashing
- `uuid` - UUID generation
- `random_string` - Random string generator
- `text_stats` - Text statistics (chars, words, lines)

#### Phase 43: Examples & Documentation
- `example_05_complete_workflow.rs` - Complete workflow example
- `example_06_mcp_client.rs` - MCP client example
- `examples/plugins/example-tool/` - Plugin example
- `examples/README.md` - Examples documentation

#### Phase 44: Docker & Deployment
- `Dockerfile.mcp` - Minimal MCP server image
- `Dockerfile.dev` - Development environment
- `.dockerignore` - Build optimization
- Enhanced `docker-compose.yml`

#### Phase 45: Security & Documentation
- `SECURITY.md` - Security policy
- `CLAUDE.md` - Claude Code integration guide

#### Phase 46: Performance & Optimization
- `PERFORMANCE.md` - Performance optimization guide
- Clippy warnings fixed
- Redundant code removed

### Changed

- MCP server now exposes 19 built-in tools
- Enhanced CI/CD with mkdocs build
- Improved README structure

### Metrics

| Metric | Value |
|--------|-------|
| Built-in tools | 19 |
| Unit tests | 77+ |
| Documentation files | 16+ |
| Dockerfiles | 3 |
