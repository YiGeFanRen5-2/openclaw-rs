# 快速入门

本指南帮助你快速启动 OpenClaw 并运行第一个示例。

## 1. 确保已构建并安装 CLI

```bash
cd /path/to/openclaw
cargo build -p openclaw-cli
cargo install --path crates/claw-cli
```

## 2. 配置 Provider

OpenClaw 支持多种 provider：

- `mock` - 本地开发测试（无需 API key）
- `openai` - OpenAI 兼容接口
- `anthropic` - Anthropic Claude
- `gemini` - Google Gemini

选择一种配置：

### 使用 Mock Provider（推荐先试）

```bash
cat > config.toml <<EOF
provider = "mock"
model = "mock-1"
EOF
```

### 使用 Anthropic

```bash
export ANTHROPIC_API_KEY="your-key"
cat > config.toml <<EOF
provider = "anthropic"
model = "claude-3-sonnet-20240229"
EOF
```

### 使用 Gemini

```bash
export GEMINI_API_KEY="your-key"
cat > config.toml <<EOF
provider = "gemini"
model = "gemini-1.5-pro-latest"
EOF
```

## 3. 运行交互式 REPL

```bash
openclaw repl --config config.toml
```

你会看到提示符：

```
OpenClaw REPL > 
```

输入消息：

```
> hello, what is 2+2?
```

Mock provider 会返回简单的响应。使用真实 provider 会调用实际 API。

## 4. 使用一次性命令

不使用 REPL，直接运行：

```bash
openclaw demo --message "Explain quantum computing in 2 sentences" --config config.toml
```

## 5. 使用 HTTP 工具

当前工具需要编写 Rust 代码或配置计划（CLI 直接支持在进行中）。

例如，自定义工具：

```rust
// 在你的插件中
#[tool]
fn get_weather(city: String) -> Result<String> {
    Ok(format!("Weather in {} is sunny", city))
}
```

## 6. 会话持久化

REPL 会话会自动保存到默认目录 `./sessions`，可在配置中修改：

```toml
session_store = "./sessions"
```

断点续聊：

```bash
openclaw repl --resume <session-id>
```

## 7. 下一步

- 阅读 [配置说明](configuration.md) 了解所有选项
- 查看 [Provider 提供商](providers.md) 了解各提供商细节
- 学习 [工具系统](tools.md) 创建自定义工具
- 探索 [插件系统](plugins.md) 扩展功能

---

遇到问题？查看 [故障排除](installation.md#故障排除) 或 [提交 Issue](https://github.com/openclaw/openclaw/issues)。
