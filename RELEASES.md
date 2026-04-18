# OpenClaw Rust Upgrade - Release Notes

## Version 0.4.0 (Phase 50-54 Complete) - 2026-04-18

### 🎉 Documentation & Polish Release

This release focuses on comprehensive documentation, API stability guarantees, integration testing framework, and enhanced example projects.

#### New Features

**Phase 50: API Docs & Quick Reference**
- Comprehensive API documentation in `docs/`
- Quick reference guide for common operations
- Updated README with better examples

**Phase 51: Integration Test Framework & Benchmarks**
- `tests/` directory with integration tests
- Benchmark suite for performance tracking
- CI/CD integration for test automation

**Phase 52: API Versioning & Stability Guarantees**
- API stability guarantees documented in `API_STABILITY.md`
- Version compatibility policy
- Breaking change notification process

**Phase 53: Enhanced Documentation Site with Tutorials**
- `site/` directory with full documentation site
- Tutorial section with step-by-step guides
- Enhanced navigation and search

**Phase 54: Additional Example Projects**
- More `examples/` demonstrating various use cases
- Example plugins and configurations
- Best practices documentation

#### Architecture

```
13 crates | 77+ tests | comprehensive documentation
```

#### Breaking Changes

None - this release adds documentation and tooling only.

#### Migration from 0.3.0

No breaking API changes. This release is purely additive.

---

## Version 0.3.0 (Phase 33-46 Complete) - 2026-04-17

### 🎉 Feature-Rich Release

### What's Changed

**New Tools (19 built-in)**
- `health_check`, `batch_health_check` - URL health monitoring
- `validate_json`, `validate_tool_input` - JSON Schema validation
- `json_store_set/get/list` - JSON key-value store
- `hash`, `uuid`, `random_string`, `text_stats` - Text utilities
- `image_info`, `image_formats` - Image metadata

**Plugin System**
- `PluginRegistry` - discovery and validation
- Semantic version compatibility checking
- Manifest-based tool/resource definitions

**Observability**
- `MetricsCollector` - OpenTelemetry-style metrics
- Prometheus format export
- Counters, Gauges, Histograms

**Docker & Deployment**
- `Dockerfile.mcp` - minimal MCP server image
- `Dockerfile.dev` - development environment
- Enhanced docker-compose.yml

**Documentation**
- SECURITY.md - security policy
- CLAUDE.md - Claude Code integration
- PERFORMANCE.md - optimization guide

---

## Version 0.1.0 (Phase 2 Complete) - 2026-04-06

### 🎉 Phase 2 完成！

本版本完成 Rust 工具框架和 Node.js 桥接的完整集成。

#### 新增功能

**核心运行时**
- 工具注册和执行系统
- 会话管理（创建、列出、删除）
- 工具调用历史记录

**首批工具**
- `list_files` - 递归列出目录内容，支持 `max_depth`
- `read_file` - 读取文件，支持 UTF-8/base64 编码

**权限系统**
- 文件系统路径白名单
- Shell 命令白名单
- 网络目标白名单

**Node.js 桥接**
- NAPI-rs 原生绑定
- TypeScript 封装
- 端到端集成测试

**开发体验**
- 单元测试：8/8 通过
- 构建脚本：`scripts/build-native.sh`
- 文档：README + Getting Started + Tool Development Guide

#### 技术栈

- Rust 1.70+
- Tokio (异步运行时)
- Napi-rs (Node 绑定)
- TypeScript 2020

#### 迁移状态

- ✅ Phase 1: Rust 骨架 + Node 桥接 PoC
- ✅ Phase 2: 工具框架 + 首批工具
- ⏳ Phase 3: 沙箱执行 + 权限强化 (计划中)
- ⏳ Phase 4: 会话压缩 + 持久化 (计划中)
- ⏳ Phase 5: 技能系统集成 (计划中)

#### 已知限制

- 工具执行是同步的（可能阻塞，Phase 3 将异步化）
- 无真正沙箱隔离（Phase 3 将实现 Linux namespaces）
- 无资源限制（CPU/内存） (Phase 3)
- 内置工具预注册，缺少动态注册 API (Phase 3)

---

## Quick Start

```bash
# 构建
cargo build --release
./scripts/build-native.sh

# 测试
cargo test --workspace
cd node-bridge && npm test
```

详见 [README.md](./README.md) 和 [GETTING_STARTED.md](./docs/GETTING_STARTED.md)。

---

## Version 0.2.0 (Phase 10-19 Complete) - 2026-04-16

### 🎉 Production Ready！

This release completes the core Rust workspace modernization with all major features implemented and tested.

#### New Features

**Phase 10-14: Core Infrastructure**
- LSP Editor Integration via `runtime::lsp::LspBridge`
- Benchmarks for session and API operations
- MCP Client for bidirectional MCP communication
- Node.js Bridge with LSP + JS tool registration
- Plugin Hot-Reload via libloading
- zstd compression utilities

**Phase 15-19: Polish & Documentation**
- Full FFI.md Node.js API reference
- GitHub Actions CI/CD pipeline
- 3 example programs
- CONTRIBUTING.md developer guide
- Clippy cleanup (39→7 warnings, real bug fixes)
- PROJECT-SUMMARY.md complete overview

#### Architecture

```
10 crates | 57 tests | 9 benchmarks | 86 commits
```

#### Benchmark Results

| Operation | Result |
|-----------|--------|
| `session_new` | ~1.2µs |
| `session_token_count` | ~7.3ns |
| `chat_message_new` | ~23ns |
| `provider_config_serialize` | ~150ns |
| Compression ratio | ~90% savings |

#### Breaking Changes

None - this release adds features only.

#### Security

- Sandbox isolation (namespace + seccomp + rlimit)
- Permission allowlists for tools
- See SECURITY.md for full policy

#### Migration from 0.1.0

No breaking API changes. The Node.js bindings remain compatible.

---

## Version 0.2.1 (Phase 23 Complete) - 2026-04-17

### 🔧 Bug Fixes & Integration Improvements

**MCP Server**
- Added `shutdown` handler (previously missing)
- All 8 MCP protocol methods now implemented

**Integration Testing**
- New `scripts/mcp_integration_test.py` (6/6 tests passing)
- Comprehensive stdio JSON-RPC protocol validation

#### Test Results

```
MCP Integration Test (6/6 passing):
✓ Initialize handshake
✓ List tools (5 tools available)
✓ Call tool (list_files)
✓ List resources (1 resource)
✓ List prompts (2 prompts)
✓ Shutdown
```

#### Files Changed

- `crates/mcp-server/src/lib.rs` - shutdown handler
- `scripts/mcp_integration_test.py` - new integration test

#### Git Status

```
Git: 94 commits
Tests: 70 unit + 6 integration
```
