# 配置说明

OpenClaw 使用 TOML 配置文件，支持 CLI 参数覆盖。

## 配置文件示例

### Mock 模式 (开发测试)

```toml
# config.toml
provider = "mock"
model = "mock-1"
window_capacity = 20
session_store = "./sessions"
plugins = []
```

### Anthropic

```toml
provider = "anthropic"
model = "claude-3-sonnet-20240229"
window_capacity = 20
session_store = "./sessions"
# API key 通过环境变量 ANTHROPIC_API_KEY 提供
```

### Gemini

```toml
provider = "gemini"
model = "gemini-1.5-pro-latest"
window_capacity = 20
session_store = "./sessions"
# API key 通过环境变量 GEMINI_API_KEY 提供
```

### OpenAI 兼容

```toml
provider = "openai"
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"  # 可选，用于自定义端点
window_capacity = 20
# API key 通过环境变量 OPENAI_API_KEY 提供
```

## 配置字段详解

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `provider` | string | `"mock"` | 提供商名称: `mock`, `openai`, `anthropic`, `gemini` |
| `model` | string | provider 默认 | 模型名称 (e.g., `claude-3-opus-20240229`) |
| `base_url` | string | provider 默认 | API 端点 (仅 OpenAI 兼容需要) |
| `window_capacity` | integer | `20` | 上下文窗口最大消息数 |
| `session_store` | string | `null` | 会话持久化目录路径 |
| `plugins` | array[string] | `[]` | 启用的插件列表 (文件名或模块路径) |

## 环境变量

| 变量 | 用途 |
|------|------|
| `OPENCLAW_API_KEY` | 通用 API key (任何 provider) |
| `OPENAI_API_KEY` | OpenAI 专用 |
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `GEMINI_API_KEY` | Google Gemini |
| `OPENCLAW_BASE_URL` | 通用 base_url |
| `OPENAI_BASE_URL` | OpenAI 专用 base_url |
| `ANTHROPIC_MODEL` | 覆盖 Anthropic 模型 |
| `GEMINI_MODEL` | 覆盖 Gemini 模型 |

优先级：CLI 参数 > 环境变量 > 配置文件。

## CLI 参数覆盖

```bash
# 覆盖 provider 和 API key
openclaw repl --provider anthropic --api-key $ANTHROPIC_API_KEY

# 覆盖 base_url
openclaw repl --base-url https://custom-endpoint.example.com/v1

# 指定配置文件
openclaw repl --config ./my-config.toml

# 禁用插件
openclaw repl --no-plugin
```

## 会话存储格式

会话以 JSON 文件存储在 `session_store` 目录：

```
sessions/
├── session-1.json
├── session-2.json
└── ...
```

每个文件包含：

```json
{
  "id": "session-1",
  "created_at": "2026-04-05T02:00:00Z",
  "updated_at": "2026-04-05T02:05:00Z",
  "status": "active",
  "model": "claude-3-sonnet-20240229",
  "history": [
    [1, {"tool":"http_get","output":{...}}, "model response..."],
    ...
  ]
}
```

## 插件配置

插件可以是：

1. **内置插件**: 列在 `plugins/` 目录，使用插件名 `"demo"`, `"http"` 等
2. **动态库**: 路径 `"./plugins/my-plugin.so"` (Linux) / `"./plugins/my-plugin.dll"` (Windows)

插件的启用顺序影响 hook 执行顺序（先注册先执行）。

---

下一步：[Provider 提供商](providers.md)
