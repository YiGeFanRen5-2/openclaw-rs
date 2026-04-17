# 工具 API 参考

本文档说明工具系统的核心类型和使用方法。

## 核心类型

### ToolSpec

工具规格，描述工具的元数据。

```rust
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub permissions: PermissionSet,
}
```

- `name`: 工具名称，用于调用
- `description`: 描述，模型会看到
- `input_schema`: JSON Schema 描述输入参数
- `permissions`: 所需权限集

### ToolOutput

工具执行结果。

```rust
pub enum ToolOutput {
    Json(serde_json::Value),
    Text(String),
}
```

推荐使用 `Json` 以便模型解析。

### ToolHandler

工具执行器，负责调用具体工具逻辑。

```rust
pub struct ToolHandler {
    tools: HashMap<String, Arc<dyn Tool + Send + Sync>>,
}
```

**关键方法**:

- `new() -> Self`
- `register(tool: impl Tool + Send + Sync + 'static)` - 注册工具
- `execute(spec: &ToolSpec, arguments: Value) -> Result<ToolOutput>` - 执行工具

### Tool

工具实现的 trait。

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(&self, arguments: Value, ctx: ExecutionContext) -> Result<ToolOutput, Box<dyn std::error::Error>>;
}
```

`ExecutionContext` 提供运行时信息（会话ID、用户等）。

### ExecutionContext

工具执行上下文。

```rust
pub struct ExecutionContext {
    pub session_id: Option<String>,
    pub user: Option<String>,
    pub state: HashMap<String, Value>,
}
```

工具可通过 `ctx.state` 读写共享数据。

### Permission & PermissionSet

权限系统。

```rust
bitflags::bitflags! {
    pub struct Permission: u16 {
        const Internet = 0b0000_0001;
        const FileRead = 0b0000_0010;
        const FileWrite = 0b0000_0100;
        const Execute = 0b0000_1000;
        const Plugin = 0b0001_0000;
    }
}
pub type PermissionSet = Permission;
```

在 `ToolSpec` 中声明所需权限，引擎在安全策略检查时使用。

## 内置工具

### EchoTool

调试工具，原样返回输入。

```rust
let tool = EchoTool::new();
let output = tool.execute(json!({ "echo": "hello" }), ctx)?;
assert_eq!(output.content, json!({ "echo": "hello" }));
```

### HttpGetTool / HttpPostTool

HTTP 请求工具，需要 `Permission::Internet`。

- `HttpGetTool` - GET 请求，参数：`url` (string)
- `HttpPostTool` - POST 请求，参数：`url`, `body` (any), `headers` (object, optional)

实现细节在 `crates/tools/src/http_tools.rs`。

## 注册工具

```rust
let mut handler = ToolHandler::new();
handler.register(EchoTool::new());
handler.register(HttpGetTool::new());
handler.register(HttpPostTool::new());
```

通常由 `RuntimeEngine` 或 `Orchestrator` 在初始化时自动注册默认工具集。

## 工具调用流程

1. **模型输出**: 模型生成工具调用请求（含 `name` 和 `arguments`）
2. **解析**: 引擎查找对应 `ToolSpec` 和实现
3. **权限检查**: 当前配置的 `PermissionSet` 必须包含工具所需权限
4. **执行**: 调用 `Tool::execute`
5. **结果**: `ToolOutput` 返回，成为模型下一步输入

## 编写自定义工具

使用 `#[tool]` 宏简化（开发中），或手动实现 `Tool` trait。

示例：

```rust
pub struct Calculator;

#[async_trait]
impl Tool for Calculator {
    async fn execute(&self, arguments: Value, _ctx: ExecutionContext) -> Result<ToolOutput, Box<dyn std::error::Error>> {
        #[derive(serde::Deserialize)]
        struct Args { a: f64, b: f64, op: String }
        let args: Args = serde_json::from_value(arguments)?;
        let result = match args.op.as_str() {
            "add" => args.a + args.b,
            "sub" => args.a - args.b,
            "mul" => args.a * args.b,
            "div" => args.a / args.b,
            _ => return Err("unknown operation".into()),
        };
        Ok(ToolOutput::json(serde_json::json!({ "result": result })))
    }
}
```

---

相关：[Runtime API](runtime.md), [插件系统](plugins.md)
