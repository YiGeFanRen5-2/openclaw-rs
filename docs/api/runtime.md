# Runtime API 参考

本文档描述 OpenClaw 的核心运行时 API。

## 核心类型

### RuntimeEngine

主要执行引擎，负责协调 provider、工具和会话。

```rust
pub struct RuntimeEngine {
    provider: Arc<dyn Provider + Send + Sync>,
    tool_handler: ToolHandler,
    // ...
}
```

**关键方法**:

- `new(provider) -> Self` - 创建引擎
- `chat(request) -> Result<ChatResponse>` - 单次对话
- `execute_plan(plan) -> Result<PlanResult>` - 执行编排计划

### Orchestrator

编排器，负责将计划转换为执行步骤。

```rust
pub struct Orchestrator {
    provider: Arc<dyn Provider + Send + Sync>,
    tool_handler: ToolHandler,
    // ...
}
```

**关键方法**:

- `new(provider) -> Self`
- `execute_plan(plan) -> Result<PlanResult>`
- `execute_step(step) -> Result<StepResult>`

### OrchestrationPlan

执行计划，包含一系列步骤。

```rust
pub struct OrchestrationPlan {
    pub steps: Vec<OrchestrationStep>,
    pub notes: Vec<String>,
}
```

### OrchestrationStep

单个执行步骤。

```rust
pub enum OrchestrationStep {
    Tool { tool: ToolSpec, arguments: serde_json::Value },
    Model { prompt: String },
}
```

### ToolSpec

工具规格定义。

```rust
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub permissions: PermissionSet,
}
```

### ToolOutput

工具执行结果。

```rust
pub enum ToolOutput {
    Json(serde_json::Value),
    Text(String),
}
```

### Session

会话状态。

```rust
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub model: Option<String>,
    pub history: Vec<(usize, ToolOutput, String)>,
}
```

### Provider

提供商 trait，由各适配器实现。

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn stream(&self, request: ChatRequest) -> Result<Receiver<Result<StreamChunk>>, ProviderError>;
    fn capabilities(&self) -> ProviderCapabilities;
}
```

## 辅助类型

- `ChatRequest`, `ChatResponse`, `ChatMessage` - 与 provider 交互的数据结构
- `ProviderConfig` - 提供商配置构建器
- `ProviderError` - 提供商错误类型
- `Permission`, `PermissionSet` - 权限控制
- `ToolHandler` - 工具执行处理器
- `JsonFileSessionStore` - JSON 文件会话存储

---

下一步：[Provider API](api/provider.md)
