# OpenClaw Rust Workspace

**Production-ready Rust implementation of the OpenClaw agent runtime.**

[![Rust](https://img.shields.io/badge/Rust-1.94+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## 🎯 项目目标

将 OpenClaw 的核心运行时从 Node.js 迁移到 Rust，获得：

- **性能提升**：CPU 密集型任务（token 计数、压缩）快 10-100 倍
- **内存安全**：消除垃圾回收暂停，降低内存占用（~10MB vs ~100MB）
- **可靠性**：强类型 + 编译时检查，减少运行时错误
- **可扩展性**：插件系统 + FFI 桥接，支持混合语言生态

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        OpenClaw Runtime                          │
├──────────────┬──────────────┬───────────────┬────────────────────┤
│  api-client │   runtime    │    tools     │      plugins        │
│  Provider   │  Session     │  Tool        │  Hook Pipeline     │
│  trait      │  Compaction  │  Registry    │  Hot Reload        │
│  Mock/OAI/  │  LSP Bridge  │  Sandbox     │  libloading        │
│  Anthropic  │  zstd Compress│  Permissions │                    │
├──────────────┴──────────────┴───────────────┴────────────────────┤
│                    MCP Layer                                       │
│  ┌─────────────────────┐      ┌─────────────────────┐              │
│  │   mcp-server        │      │   mcp-client        │              │
│  │   (stdio server)    │      │   (connect ext svr)│              │
│  └─────────────────────┘      └─────────────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│  FFI / N-API                                                      │
│  ┌─────────────────────┐      ┌─────────────────────┐              │
│  │   openclaw-node-    │      │   openclaw-core     │              │
│  │   bridge            │      │   (core types)      │              │
│  └─────────────────────┘      └─────────────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│  harness / LSP Client                                             │
│  ┌─────────────────────────────────────────────────────┐          │
│  │   LspClient (rust-analyzer, pyright, tsserver)    │          │
│  └─────────────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

## 📦 Crates

| Crate | 状态 | 描述 |
|-------|------|------|
| `api-client` | ✅ | Provider trait + Mock/OpenAI/Anthropic + Retry + RateLimit |
| `runtime` | ✅ | Session 持久化 + 压缩 + LSP Bridge + zstd 压缩 |
| `tools` | ✅ | 工具注册 + 沙箱执行（namespace + seccomp） |
| `plugins` | ✅ | Hook pipeline + 权限系统 + 热重载（libloading） |
| `node-bridge` | ✅ | N-API 绑定，LSP + 工具注册 + Session API |
| `ffi` | ✅ | C FFI 接口（基础） |
| `mcp-server` | ✅ | MCP 协议服务器（stdio） |
| `mcp-client` | ✅ | MCP 客户端（连接外部 MCP 服务器） |
| `harness` | ✅ | LSP Client（rust-analyzer 等） |
| `openclaw-core` | ✅ | 核心类型库 |

## 🚀 快速开始

### 构建

```bash
cd openclaw-rs
cargo build --release
```

### 测试

```bash
cargo test --all
```

### Benchmarks

```bash
cargo bench --all
```

See [BENCHMARKS.md](./BENCHMARKS.md) for detailed benchmark results and analysis.

### Node.js 集成

```javascript
const { OpenClawRuntime, ProviderMode } = require('./target/release/openclaw_node_bridge.node');

const rt = new OpenClawRuntime(ProviderMode.Mock, null, null, 'mock-v1');

// Session management
rt.createSession('test-session');
rt.addMessage('test-session', 'user', 'Hello, OpenClaw!');

// List and execute tools
const tools = rt.listTools();
console.log(`Found ${tools.length} tools`);

const result = rt.executeTool('list_files', JSON.stringify({ path: '/tmp' }));
console.log('Files:', result);

// Chat with provider
const chatResult = rt.chat(JSON.stringify({
  messages: [{ role: 'user', content: 'Hello!' }],
  model: 'mock-model'
}));
console.log('Chat:', chatResult);
```

详细文档：[FFI.md](./FFI.md)

运行示例：`node examples/example_04_node_integration.js`

---

## 📊 性能基准

| 指标 | 目标 | 实测 |
|------|------|------|
| `session_new` | < 1ms | ~1.2µs ✅ |
| `session_add_message` | < 1ms | ~3µs ✅ |
| `session_token_count` | < 1ms | ~7.3ns ✅ |
| `provider_config_serialize` | < 1ms | ~150ns ✅ |
| `chat_message_new` | < 1ms | ~23ns ✅ |

完整 benchmark 报告：`target/criterion/`

---

## 📚 Documentation

### API Documentation

Generate Rust API docs:
```bash
cargo doc --all --no-deps
# View at: target/doc/index.html
```

Or use the helper script:
```bash
./scripts/generate_docs.sh
```

### Guides

| Document | Description |
|----------|-------------|
| [QUICKSTART.md](./QUICKSTART.md) | Quick start guide |
| [FFI.md](./FFI.md) | Node.js API reference |
| [CLAUDE.md](./CLAUDE.md) | Claude Desktop integration |
| [BENCHMARKS.md](./BENCHMARKS.md) | Performance benchmarks |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | Developer guide |
| [SECURITY.md](./SECURITY.md) | Security policy |

---

## 📋 功能对等清单

详见 [PARITY.md](./PARITY.md)

---

## 🔧 开发

### 代码结构

```
crates/
├── api-client/      # Provider trait + adapters
├── runtime/         # Session, compaction, LSP, compression
│   ├── src/
│   │   ├── lib.rs          # Core types
│   │   ├── lsp.rs          # LSP Bridge
│   │   ├── compression.rs   # zstd compression
│   │   └── persistence.rs   # Session store
│   └── benches/            # Benchmarks
├── tools/           # Tool system + sandbox
├── plugins/         # Plugin lifecycle + hot reload
├── harness/          # LSP Client
├── mcp-server/      # MCP stdio server
├── mcp-client/       # MCP client (connect to servers)
├── node-bridge/      # N-API bindings for Node.js
└── ffi/             # C FFI layer
```

### 添加新 Crate

```bash
# 1. Create crate
cargo new --lib crates/my-crate

# 2. Add to workspace Cargo.toml members

# 3. Add dependencies
[dependencies]
runtime = { path = "../runtime" }
```

---

## 📜 许可证

MIT
