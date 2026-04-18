# 配置指南

本指南详细说明 OpenClaw 的所有配置选项，包括配置文件格式、环境变量、CLI 参数及其优先级。

## 配置文件

OpenClaw 使用 [TOML](https://toml.io/) 格式的配置文件。

### 基本配置示例

```toml
# config/app.toml
provider = "anthropic"
model = "claude-3-haiku-20240307"
window_capacity = 20
session_store = "./sessions"
plugins = []
```

### 完整配置示例

```toml
# config/production.toml
provider = "anthropic"
model = "claude-3-sonnet-20240229"
base_url = "https://api.anthropic.com"
window_capacity = 30
session_store = "./data/sessions"
plugins = ["./plugins/http.so", "./plugins/logger.so"]
timeout_ms = 60000
max_retries = 3
log_level = "info"
```

## 配置字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `provider` | string | `"mock"` | AI 提供商：`mock`, `openai`, `anthropic`, `gemini` |
| `model` | string | provider 默认 | 模型标识符 |
| `base_url` | string | provider 默认 | API 端点（OpenAI 兼容用） |
| `window_capacity` | integer | `20` | 上下文窗口最大消息数 |
| `session_store` | string | `null` | 会话持久化目录 |
| `plugins` | array[string] | `[]` | 加载的插件列表 |
| `timeout_ms` | integer | `30000` | API 请求超时（毫秒） |
| `max_retries` | integer | `3` | 请求失败重试次数 |
| `log_level` | string | `"info"` | 日志级别：`trace`, `debug`, `info`, `warn`, `error` |

## Provider 配置

### Anthropic (推荐)

```toml
provider = "anthropic"
model = "claude-3-sonnet-20240229"
```

环境变量：

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"  # 可选
export ANTHROPIC_MODEL="claude-3-opus-20240229"         # 覆盖 model
```

可用模型：

- `claude-3-opus-20240229` - 最强，适合复杂推理
- `claude-3-sonnet-20240229` - 平衡性能和成本
- `claude-3-haiku-20240307` - 最快，适合简单任务

### OpenAI 兼容

```toml
provider = "openai"
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"
```

环境变量：

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"
```

兼容 OpenAI API 的第三方服务（如 Groq、Ollama）：

```toml
provider = "openai"
model = "llama-3.1-70b-versatile"
base_url = "https://api.groq.com/openai/v1"
```

### Google Gemini

```toml
provider = "gemini"
model = "gemini-1.5-pro-latest"
```

环境变量：

```bash
export GEMINI_API_KEY="..."
export GEMINI_MODEL="gemini-1.5-flash"
```

### Mock Provider

用于本地开发，无需 API Key：

```toml
provider = "mock"
model = "mock-1"
```

## 环境变量

### 优先级

CLI 参数 > 环境变量 > 配置文件

### 通用变量

| 变量 | 说明 |
|------|------|
| `OPENCLAW_API_KEY` | 通用 API Key（任何 provider） |
| `OPENCLAW_BASE_URL` | 通用 base URL |
| `OPENCLAW_MODEL` | 覆盖配置的模型名 |

### Provider 专用变量

| 变量 | Provider |
|------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic |
| `GEMINI_API_KEY` | Google Gemini |

## CLI 参数

### 常用命令

```bash
# REPL 模式
openclaw repl --config config.toml

# 一次性消息
openclaw demo --message "Hello" --config config.toml

# 指定 provider
openclaw repl --provider anthropic --api-key $ANTHROPIC_API_KEY

# 覆盖配置
openclaw repl --config config.toml --model claude-3-haiku-20240307

# 会话恢复
openclaw repl --resume <session-id>
```

### 完整参数列表

| 参数 | 说明 |
|------|------|
| `--config <path>` | 指定配置文件路径 |
| `--provider <name>` | 覆盖配置的 provider |
| `--model <model>` | 覆盖配置的模型 |
| `--api-key <key>` | 设置 API Key |
| `--base-url <url>` | 覆盖 base URL |
| `--session-store <dir>` | 设置会话存储目录 |
| `--no-plugin` | 禁用所有插件 |
| `--plugin <path>` | 加载指定插件 |
| `--resume <id>` | 恢复指定会话 |
| `--timeout <ms>` | 请求超时（毫秒） |
| `--log-level <level>` | 日志级别 |
| `--version` | 显示版本 |
| `--help` | 显示帮助 |

## 会话存储

### 配置

```toml
session_store = "./sessions"
```

### 存储格式

会话存储为 JSON 文件：

```json
// sessions/session-abc123.json
{
  "id": "session-abc123",
  "created_at": "2026-04-18T06:00:00Z",
  "updated_at": "2026-04-18T06:30:00Z",
  "status": "active",
  "model": "claude-3-sonnet-20240229",
  "message_count": 12,
  "history": [
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi!"}
  ]
}
```

### 会话操作

```bash
# 列出所有会话
ls sessions/

# 查看会话内容
cat sessions/session-abc123.json

# 删除会话
rm sessions/session-abc123.json
```

## 插件配置

### 加载内置插件

```toml
plugins = ["http", "logger"]
```

### 加载动态库插件

```toml
plugins = ["./plugins/my-plugin.so"]
```

### 加载多个插件

```toml
plugins = [
    "./plugins/logger.so",
    "./plugins/http.so",
    "./plugins/auth.so"
]
```

插件执行顺序：按数组顺序注册，先注册先执行。

## 多环境配置

### 开发环境

```toml
# config/dev.toml
provider = "mock"
model = "mock-1"
log_level = "debug"
session_store = "./dev-sessions"
```

### 生产环境

```toml
# config/prod.toml
provider = "anthropic"
model = "claude-3-sonnet-20240229"
log_level = "warn"
session_store = "/var/lib/openclaw/sessions"
plugins = ["./plugins/prod-plugins/http.so"]
timeout_ms = 60000
max_retries = 5
```

### 使用环境变量选择配置

```bash
# 开发
export OPENCLAW_ENV=dev
openclaw repl --config config/$OPENCLAW_ENV.toml
```

## 日志配置

### 环境变量

```bash
export RUST_LOG=openclaw_runtime=debug,openclaw_tools=info
```

### 配置文件中设置

```toml
log_level = "debug"
```

### 级别说明

| 级别 | 用途 |
|------|------|
| `trace` | 最详细（函数调用跟踪） |
| `debug` | 开发调试 |
| `info` | 一般信息（默认） |
| `warn` | 警告 |
| `error` | 仅错误 |

### 输出到文件

```bash
openclaw repl --config config.toml 2>&1 | tee openclaw.log
```

## 高级配置

### 自定义 HTTP 客户端

```rust
use openclaw_runtime::{Runtime, RuntimeConfig};
use std::time::Duration;

let config = RuntimeConfig::builder()
    .provider("anthropic")
    .model("claude-3-haiku-20240307")
    .api_key(api_key)
    .timeout(Duration::from_secs(120))  // 2 分钟超时
    .max_retries(5)
    .build()?;
```

### 上下文窗口调优

```toml
# 较小窗口：更快响应，较低成本
window_capacity = 10

# 较大窗口：更连贯的上下文，成本更高
window_capacity = 50
```

### 重试策略

```toml
max_retries = 3
timeout_ms = 30000
```

瞬时失败会自动重试，指数退避策略。

## 配置验证

```bash
# 检查配置文件语法
openclaw validate --config config.toml

# 测试连接
openclaw test --config config.toml
```

## 下一步

- [Provider 详细说明](../guide/providers.md)
- [工具系统](../guide/tools.md)
- [插件系统](../guide/plugins.md)
- [会话持久化](../guide/persistence.md)
