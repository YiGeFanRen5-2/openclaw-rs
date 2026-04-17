# Provider API 参考

本文档详细说明 Provider trait 及其实现。

## Provider Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn stream(&self, request: ChatRequest) -> Result<Receiver<Result<StreamChunk>>, ProviderError>;
    fn capabilities(&self) -> ProviderCapabilities;
}
```

### Chat 方法

执行单次非流式聊天完成。

**参数**:

- `request: ChatRequest` - 包含消息列表、模型、temperature 等

**返回**:

- `Result<ChatResponse, ProviderError>` - 成功时返回完整响应

### Stream 方法

执行流式聊天完成，返回一个异步 receiver。

**参数**:

- `request: ChatRequest` - 同 `chat`

**返回**:

- `Result<Receiver<Result<StreamChunk>>, ProviderError>` - 流式块接收器

### Capabilities

返回提供商的能力标志：

```rust
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub max_context_length: usize,
    pub supported_models: Vec<String>,
}
```

## 配置结构

### ProviderConfig

构建器模式配置提供商：

```rust
let cfg = ProviderConfig::new("anthropic")
    .api_key("sk-...")
    .model("claude-3-sonnet-20240229")
    .base_url("https://api.anthropic.com/v1");
```

字段：

- `provider_name` - 提供商标识符
- `api_key` - 认证密钥（必需）
- `base_url` - API 端点（可选，有默认）
- `model` - 模型名称（可选，有默认）

## 错误类型

```rust
pub enum ProviderError {
    Config(String),          // 配置缺失或无效
    Api { status: u16, body: String }, // HTTP 错误
    Parse(String),           // 响应解析失败
    Timeout,                 // 请求超时
    Unknown(Box<dyn std::error::Error>),
}
```

建议使用 `match` 处理并给出用户友好提示。

## 实现自定义 Provider

如果你想接入新的 AI 服务，可以实现 `Provider` trait：

```rust
pub struct MyProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

#[async_trait]
impl Provider for MyProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 1. 转换 ChatRequest 到服务端格式
        // 2. 发送 HTTP 请求
        // 3. 解析响应为 ChatResponse
    }

    async fn stream(&self, request: ChatRequest) -> Result<Receiver<Result<StreamChunk>>, ProviderError> {
        // 类似，返回流式响应
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            max_context_length: 128_000,
            supported_models: vec!["my-model-v1".to_string()],
        }
    }
}
```

然后注册到 RuntimeEngine 或 Orchestrator。

---

相关：[Runtime API](runtime.md), [工具系统](tools.md)
