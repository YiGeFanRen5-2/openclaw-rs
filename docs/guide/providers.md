# Provider 提供商

OpenClaw 支持多个 AI 提供商。本文档介绍每种提供商的特性、配置和限制。

## 支持的提供商

| 提供商 | 状态 | 流式 | 说明 |
|--------|------|------|------|
| `mock` | ✅ 稳定 | ❌ | 本地测试用，返回简单响应 |
| `openai` | 🔄 基础 | ✅ | OpenAI 兼容接口（GPT 系列） |
| `anthropic` | ✅ 稳定 | ✅ | Anthropic Claude (3.5, 3, 2.1) |
| `gemini` | ✅ 稳定 | ✅ | Google Gemini (1.5, 1.0) |

## Mock Provider

用于开发和测试，不需要 API key。

```toml
provider = "mock"
model = "mock-1"
```

响应示例：

```
Mock response: you said "hello"
```

## OpenAI Provider

支持 OpenAI 的 `/v1/chat/completions` API 及其兼容服务（如 LocalAI、vLLM）。

```toml
provider = "openai"
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"  # 可选，默认值
```

环境变量：

```bash
export OPENAI_API_KEY="sk-..."
```

**注意**: OpenAI Provider 当前使用 stub 实现，完整实现待完成。

## Anthropic Provider

支持 Claude 3.5 Sonnet、3 Opus/Sonnet/Haiku、2.1。

```toml
provider = "anthropic"
model = "claude-3-sonnet-20240229"
```

环境变量：

```bash
export ANTHROPIC_API_KEY="..."
# 可选覆盖模型
export ANTHROPIC_MODEL="claude-3-opus-20240229"
```

API 说明：

- 端点: `https://api.anthropic.com/v1/messages`
- 认证: `x-api-key` header, `anthropic-version: 2023-06-01`
- 请求体: `{ "model": "...", "messages": [...], "stream": false/true }`
- 响应: `{ "content": [{ "type": "text", "text": "..." }], "usage": {...} }`

## Gemini Provider

支持 Google Gemini 1.5 Pro/Flash、1.0 Pro。

```toml
provider = "gemini"
model = "gemini-1.5-pro-latest"
```

环境变量：

```bash
export GEMINI_API_KEY="..."
# 可选覆盖模型
export GEMINI_MODEL="gemini-1.5-flash-latest"
```

API 说明：

- 端点: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key=...`
- 请求体: `{ "contents": [{ "parts": [{ "text": "..." }] }], "generationConfig": {...} }`
- 响应: `{ "candidates": [{ "content": { "parts": [{ "text": "..." }] } }] }`

## 流式响应

所有真实 providers (OpenAI, Anthropic, Gemini) 都支持流式输出（streaming）。在 Rust 代码中使用 `Provider::stream()`，CLI 中可通过 `--stream` 标志启用（开发中）。

## 错误处理

Provider 可能返回的错误：

- `Config`: API key 缺失或配置错误
- `ApiError`: HTTP 4xx/5xx，包含状态码和消息
- `ParseError`: 响应解析失败
- `Timeout`: 请求超时

建议应用层捕获并友好展示。

## 切换提供商

只需修改 `provider` 字段和对应的 API key 环境变量即可切换。OpenClaw 会处理底层差异。

---

下一步：[工具系统](tools.md)
