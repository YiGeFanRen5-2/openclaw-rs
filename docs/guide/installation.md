# 安装指南

本文档说明如何构建和安装 OpenClaw。

## 系统要求

- **Rust**: 1.75 或更高版本 (https://rustup.rs/)
- **Git**
- **CMake** 和 **build-essential** (Linux) 或 Xcode Command Line Tools (macOS)

## 从源码构建

```bash
# 克隆仓库
git clone https://github.com/openclaw/openclaw.git
cd openclaw

# 构建 runtime
cargo build -p openclaw-runtime

# 构建 CLI
cargo build -p openclaw-cli

# 运行测试
cargo test -p openclaw-runtime
```

## 安装 CLI

```bash
# 安装到系统 (release)
cargo install --path crates/claw-cli

# 验证安装
openclaw --version
```

## 可选组件

- **Node.js 桥接**: `cargo build -p openclaw-node-bridge` (需要 Node.js 开发环境)
- **文档**: `mdbook build` (需要安装 mdbook)

## 开发环境

推荐使用 `rust-analyzer` 或 `VS Code` 的 Rust 扩展。

配置环境变量：

```bash
# Anthropic
export ANTHROPIC_API_KEY="your-key"

# Gemini
export GEMINI_API_KEY="your-key"

# OpenAI (兼容)
export OPENAI_API_KEY="your-key"
```

## 故障排除

### 编译错误

如果遇到 `openssl` 相关错误，安装系统依赖：

**Ubuntu/Debian**:
```bash
sudo apt-get install pkg-config libssl-dev
```

**macOS**:
```bash
brew install openssl
```

### 无法找到 crate

确保所有子 crate 都在 workspace 中：

```bash
cargo check --workspace
```

---

下一步：[快速入门](getting-started.md)
