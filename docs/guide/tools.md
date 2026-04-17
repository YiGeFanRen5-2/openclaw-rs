# 工具系统 (Tools)

OpenClaw 的工具系统允许 AI 模型调用外部功能（HTTP、计算、文件 I/O 等）。

## 工具概述

工具（Tool）在 OpenClaw 中定义为：

```rust
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub permissions: PermissionSet,
}
```

工具通过 `Orchestrator` 执行，在计划（`OrchestrationPlan`）中按步骤调用。

## 内置工具

### HTTP 工具

提供 `http_get` 和 `http_post` 用于网络请求。

**http_get**

```toml
# 在计划步骤中
{
  "tool": "http_get",
  "args": {
    "url": "https://api.example.com/data"
  }
}
```

响应：

```json
{
  "status": 200,
  "headers": {...},
  "body": "..."
}
```

**http_post**

```json
{
  "tool": "http_post",
  "args": {
    "url": "https://api.example.com/submit",
    "body": { "key": "value" },
    "headers": { "Content-Type": "application/json" }
  }
}
```

**权限**: `Permission::Internet`

## 自定义工具

你可以创建自己的工具并通过插件或直接注册到运行时。

### Rust 实现工具

```rust
use openclaw_tools::{tool, ToolOutput, ExecutionContext, Permission, PermissionSet};

#[tool(
    name = "calculator",
    description = "Perform basic arithmetic",
    input_schema = {
        "type": "object",
        "properties": {
            "a": { "type": "number" },
            "b": { "type": "number" },
            "op": { "type": "string", "enum": ["add", "sub", "mul", "div"] }
        },
        "required": ["a", "b", "op"]
    }
)]
fn calculator(ctx: ExecutionContext, a: f64, b: f64, op: String) -> Result<ToolOutput, Box<dyn std::error::Error>> {
    let result = match op.as_str() {
        "add" => a + b,
        "sub" => a - b,
        "mul" => a * b,
        "div" => a / b,
        _ => return Err("invalid operation".into()),
    };
    Ok(ToolOutput::json(serde_json::json!({ "result": result })))
}
```

### 工具权限

定义工具需要的权限，运行时根据 `PermissionSet` 决定是否允许执行：

- `Permission::Internet` - 网络访问
- `Permission::FileRead` - 文件读取
- `Permission::FileWrite` - 文件写入
- `Permission::Execute` - 执行外部命令
- `Permission::Plugin` - 调用其他插件

通过 `permissions` 字段声明，引擎在执行前检查。

## 工具调用流程

1. **模型生成**: 模型输出包含工具调用请求（tool_calls）
2. **解析**: 引擎解析为 `OrchestrationStep::Tool`
3. **权限检查**: 验证 `PermissionSet` 是否满足当前配置
4. **执行**: 调用 `ToolHandler::execute` 运行工具
5. **结果**: `ToolOutput` 返回给模型作为下一步输入

## 工具输出格式

所有工具返回 `ToolOutput`，目前支持：

- `ToolOutput::json(Value)` - JSON 结构化输出
- `ToolOutput::text(String)` - 纯文本输出

输出会被序列化为 JSON 供模型消费。

## 调试工具

使用 `EchoTool` 进行调试：

```rust
let echo = EchoTool::new();
let output = echo.execute(args)?;
assert_eq!(output.content["echo"], "test");
```

---

下一步：[会话持久化](persistence.md)
