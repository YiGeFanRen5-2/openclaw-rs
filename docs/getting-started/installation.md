# 安装指南

本指南详细说明 OpenClaw Rust Runtime 的安装与配置。

## 系统要求

| 要求 | 版本 | 说明 |
|------|------|------|
| Rust | ≥ 1.75 | 通过 [rustup](https://rustup.rs/) 安装 |
| Git | 最新 | 用于克隆源码 |
| CMake | ≥ 3.10 | Linux 构建依赖 |
| build-essential | 最新 | Linux 编译工具链 |
| Xcode CLT | 最新 | macOS 开发工具 |

## 从源码构建

### 1. 克隆仓库

```bash
git clone https://github.com/YiGeFanRen5-2/openclaw-rs.git
cd openclaw-rs
```

### 2. 构建所有组件

```bash
# 构建完整项目（Debug）
cargo build --workspace

# 构建发布版本（推荐用于生产）
cargo build --workspace --release
```

### 3. 构建单个组件

```bash
# 构建 CLI 工具
cargo build -p openclaw-cli

# 构建核心运行时
cargo build -p openclaw-runtime

# 构建工具库
cargo build -p openclaw-tools

# 构建插件系统
cargo build -p openclaw-plugin
```

### 4. 运行测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 的测试
cargo test -p openclaw-runtime

# 带日志运行测试
RUST_LOG=debug cargo test -p openclaw-runtime
```

## 安装 CLI

### 从本地构建安装

```bash
# 安装 CLI 到 ~/.cargo/bin
cargo install --path crates/cli

# 或指定安装路径
cargo install --path crates/cli --force --bins

# 验证安装
openclaw --version
```

### 从 crates.io 安装

```bash
cargo install openclaw-cli

# 验证
openclaw --version
```

## 依赖安装

### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y cmake build-essential pkg-config libssl-dev
```

### Linux (RHEL/CentOS/Alibaba Cloud)

```bash
sudo yum install -y cmake gcc gcc-c++ make openssl-devel
```

### macOS

```bash
# 安装 Xcode Command Line Tools
xcode-select --install

# 验证 CMake
cmake --version
```

## 环境配置

### 添加到 PATH

如果 `cargo install` 后命令找不到，添加 cargo bin 到 PATH：

```bash
# 临时生效
export PATH="$HOME/.cargo/bin:$PATH"

# 永久生效 (Linux/macOS)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 环境变量

```bash
# Anthropic (推荐)
export ANTHROPIC_API_KEY="sk-ant-..."

# OpenAI (可选)
export OPENAI_API_KEY="sk-..."

# Gemini (可选)
export GEMINI_API_KEY="..."

# Rust 优化
export RUSTFLAGS="-C target-cpu=native"
```

## Docker 支持

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/openclaw /usr/local/bin/
ENTRYPOINT ["openclaw"]
```

构建并运行：

```bash
docker build -t openclaw .
docker run --rm -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY openclaw repl
```

## 验证安装

```bash
# 检查版本
openclaw --version

# 查看帮助
openclaw --help

# 查看子命令
openclaw repl --help
openclaw demo --help
```

## 故障排除

### 编译错误

**问题**: `error: linker 'cc' not found`

**解决**:
```bash
# Linux
sudo apt install build-essential   # Debian/Ubuntu
sudo yum groupinstall Development   # RHEL/CentOS
```

**问题**: `error: failed to run custom build command for `openssl-sys`

**解决**:
```bash
# Debian/Ubuntu
sudo apt install pkg-config libssl-dev

# RHEL/CentOS
sudo yum install openssl-devel
```

### 运行时错误

**问题**: `command not found: openclaw`

**解决**: 确保 `~/.cargo/bin` 在 PATH 中：

```bash
echo $PATH | grep cargo
# 如果没有，添加：
export PATH="$HOME/.cargo/bin:$PATH"
```

### 性能问题

```bash
# 使用 LTO 优化构建
cargo build --workspace --release --locked

# 使用 mold 链接器加速
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --workspace --release
```

## 下一步

- [快速入门：创建第一个项目](first-project.md)
- [配置说明](configuration.md)
- [工具系统](../guide/tools.md)
- [插件系统](../guide/plugins.md)
