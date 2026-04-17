# Getting Started - OpenClaw Rust Upgrade

本指南帮助你快速搭建开发环境并开始贡献。

---

## Prerequisites

- **Rust**：1.70+ (stable) - [安装](https://rustup.rs/)
- **Node.js**：18+ - [安装](https://nodejs.org/)
- **Git**：任意版本

验证安装：
```bash
rustc --version
node --version
npm --version
```

---

## Initial Setup

### 1. 克隆并构建

```bash
# 克隆仓库（如果你有权限）或确保在正确的路径
cd ~/.openclaw/workspace/openclaw-rs

# 编译 Rust 工作区（Release 模式）
cargo build --release

# 检查编译（快速）
cargo check --workspace
```

预期输出（最后几行）：
```
   Checking ffi v0.1.0 (/path/to/openclaw-rs/crates/ffi)
    Finished release profile [optimized] target(s) in 40s
```

### 2. 设置 Node.js Bridge

```bash
# 进入桥接目录
cd node-bridge

# 安装依赖
npm install

# 复制原生模块（从 Rust 构建输出）
./scripts/prepare.js
# 或者手动：
cp ../target/release/libffi.so ./openclaw-rs.node

# 构建 TypeScript
npm run build
```

预期输出：
```
✅ Loaded native from: .../openclaw-rs.node
```

### 3. 运行测试

```bash
# Rust 单元测试
cd ~/.openclaw/workspace/openclaw-rs
cargo test --workspace

# Node.js 集成测试
cd node-bridge
npm test
```

预期看到：
```
✅ Runtime created
✅ list_files returned X items
🎉 Phase 2.3 integration complete!
```

---

## 项目结构

```
openclaw-rs/
├── crates/
│   ├── runtime/      # 运行时核心
│   ├── tools/        # 工具框架和实现
│   ├── ffi/          # Node.js 原生绑定
│   ├── api-client/   # API 抽象（待实现）
│   ├── plugins/      # 插件系统（待实现）
│   └── harness/      # 测试 harness（待实现）
├── node-bridge/      # TypeScript 桥接
│   ├── src/
│   │   └── index.ts  # 主要导出
│   ├── test-poc.js   # 集成测试
│   └── package.json
├── docs/             # 文档
├── scripts/          # 辅助脚本
└── target/           # Cargo 构建输出（gitignore）
```

---

## 开发工作流

### 修改 Rust 代码

```bash
# 进入工作区根目录
cd ~/.openclaw/workspace/openclaw-rs

# 编辑代码（例如添加工具）
vim crates/tools/src/lib.rs

# 检查编译
cargo check

# 运行单元测试
cargo test --package tools

# 如果修改了 API，更新 FFI 和 Node bridge
cargo build --release -p ffi
cp target/release/libffi.so node-bridge/openclaw-rs.node
cd node-bridge
npm run build
```

### 修改 TypeScript 代码

```bash
cd node-bridge

# 编辑源码
vim src/index.ts

# 构建并测试
npm run build
npm test
```

### 提交变更

```bash
git add .
git commit -m "feat(tools): add new tool for..."
git push
```

---

## 常见问题

### Q: `cargo check` 报错 " unresolved import `napi`"

A: 确保在 workspace 根目录运行，并且 `crates/ffi/Cargo.toml` 中包含 `napi` workspace 依赖。如果问题持续，尝试 `cargo update`。

### Q: Node.js 加载 `.node` 文件失败

A: 检查：
  - 文件存在：`ls -l node-bridge/openclaw-rs.node`
  - 可执行权限：`chmod +x node-bridge/openclaw-rs.node`
  - 架构匹配：`file node-bridge/openclaw-rs.node` 应为 "ELF 64-bit"

### Q: `list_files` 工具报错 "Permission denied"

A: 当前白名单硬编码为 `/home`, `/tmp`, `/workspace`, `/root`。如果要访问其他路径，需要修改 `crates/tools/src/lib.rs` 中 `ListFilesTool::permission()`。

### Q: 如何添加一个新工具？

A: 参考 `docs/TOOL_DEVELOPMENT.md` 以及 `ListFilesTool` 和 `ReadFileTool` 的实现模式。

### Q: 想用 `async` 工具怎么办？

A: Phase 2 使用同步模型简化。Phase 3 将引入异步运行时（`tokio`）和沙箱。如需异步，可以在 `Tool::execute` 内使用 `tokio::task::spawn_blocking` 包装阻塞调用。

---

## 下一步

- 阅读 **Tool Development** 了解如何添加新工具
- 查看 **ADR**（架构决策记录）理解设计思路
- 查看 **Upgrade Plan** 了解长期路线图

---

## 获取帮助

- **Issues**：GitHub Issues
- **Discord**：OpenClaw 社区服务器
- **Docs**：https://docs.openclaw.ai

祝你 hacking 愉快！🐟