# 自定义工具

本指南讲解如何在 OpenClaw 中开发和使用自定义工具，涵盖基础注册、权限系统、异步工具、工具链和高级模式。

## 工具概述

工具（Tool）让 AI 模型能够调用外部功能。OpenClaw 的工具系统支持：

- **HTTP 请求** - GET/POST 网络调用
- **文件 I/O** - 读写本地文件
- **进程执行** - 运行系统命令
- **自定义逻辑** - 任意 Rust 实现的工具

## 基础工具定义

### 工具结构

```rust
pub struct ToolSpec {
    pub name: String,           // 工具唯一标识
    pub description: String,    // 供模型理解的描述
    pub input_schema: Value,    // JSON Schema 格式的参数定义
    pub permissions: PermissionSet,
    pub handler: Box<dyn ToolHandler>,
}
```

### 基本工具示例

```rust
use openclaw_tools::{
    tool, ToolOutput, ExecutionContext,
    Permission, PermissionSet,
    ToolResult, ToolError,
};

#[tool(
    name = "get_time",
    description = "Returns the current date and time in ISO 8601 format",
    input_schema = {
        "type": "object",
        "properties": {},
        "required": []
    }
)]
fn get_time(_ctx: ExecutionContext) -> Result<ToolOutput, ToolError> {
    let now = chrono::Utc::now().to_rfc3339();
    Ok(ToolOutput::text(now))
}
```

## 注册工具

### 在运行时注册

```rust
use openclaw_runtime::{Runtime, RuntimeConfig, ToolRegistry};
use openclaw_tools::{tool, ToolOutput, ExecutionContext};

#[tool(
    name = "hello",
    description = "Returns a greeting message",
)]
fn hello(ctx: ExecutionContext, name: String) -> Result<ToolOutput, ToolError> {
    Ok(ToolOutput::text(format!("Hello, {}!", name)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig::builder()
        .provider("mock")
        .model("mock-1")
        .tool("hello", hello)
        .build()?;

    let runtime = Runtime::new(config).await?;
    Ok(())
}
```

### 注册多个工具

```rust
let config = RuntimeConfig::builder()
    .provider("anthropic")
    .model("claude-3-haiku-20240307")
    .tool("get_time", get_time)
    .tool("hello", hello)
    .tool("calculator", calculator)
    .build()?;
```

### 通过插件注册工具

在插件中定义工具：

```rust
use openclaw_plugin::{Plugin, ToolProvider};

struct MyToolPlugin;

impl ToolProvider for MyToolPlugin {
    fn tools(&self) -> Vec<ToolSpec> {
        vec![
            my_tool_1(),
            my_tool_2(),
        ]
    }
}
```

## 输入模式

### 基础参数

```rust
#[tool(
    name = "weather",
    description = "Get weather for a city",
    input_schema = {
        "type": "object",
        "properties": {
            "city": {
                "type": "string",
                "description": "City name"
            }
        },
        "required": ["city"]
    }
)]
fn weather(ctx: ExecutionContext, city: String) -> Result<ToolOutput, ToolError> {
    // ...
}
```

### 多种参数类型

```rust
#[tool(
    name = "create_reminder",
    description = "Create a timed reminder",
    input_schema = {
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "minutes_from_now": { "type": "integer", "minimum": 1 },
            "priority": {
                "type": "string",
                "enum": ["low", "medium", "high"]
            }
        },
        "required": ["title", "minutes_from_now"]
    }
)]
fn create_reminder(
    ctx: ExecutionContext,
    title: String,
    minutes_from_now: i64,
    priority: String,  // Optional: defaults handled in code
) -> Result<ToolOutput, ToolError> {
    // ...
}
```

### 嵌套对象

```rust
#[tool(
    name = "send_notification",
    description = "Send a notification",
    input_schema = {
        "type": "object",
        "properties": {
            "recipient": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "email": { "type": "string" }
                },
                "required": ["name", "email"]
            },
            "message": { "type": "string" },
            "urgent": { "type": "boolean" }
        },
        "required": ["recipient", "message"]
    }
)]
fn send_notification(
    ctx: ExecutionContext,
    recipient: RecipientData,
    message: String,
    urgent: bool,
) -> Result<ToolOutput, ToolError> {
    // recipient.name, recipient.email 可用
}
```

### 数组参数

```rust
#[tool(
    name = "batch_process",
    description = "Process multiple items",
    input_schema = {
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": { "type": "string" }
            },
            "parallel": { "type": "boolean" }
        },
        "required": ["items"]
    }
)]
fn batch_process(
    ctx: ExecutionContext,
    items: Vec<String>,
    parallel: bool,
) -> Result<ToolOutput, ToolError> {
    // ...
}
```

## 权限系统

### 权限类型

```rust
use openclaw_tools::Permission;

let permissions = PermissionSet::from([
    Permission::Internet,      // 网络访问
    Permission::FileRead,       // 读取文件
    Permission::FileWrite,      // 写入文件
    Permission::Execute,       // 执行命令
    Permission::EnvRead,        // 读取环境变量
    Permission::Plugin,         // 调用其他插件
]);
```

### 工具级权限

```rust
#[tool(
    name = "fetch_url",
    description = "Fetch content from a URL",
    permissions = [Permission::Internet]
)]
fn fetch_url(ctx: ExecutionContext, url: String) -> Result<ToolOutput, ToolError> {
    // ...
}
```

### 权限检查

```rust
fn sensitive_tool(ctx: ExecutionContext, path: String) -> Result<ToolOutput, ToolError> {
    // 手动检查权限
    if !ctx.can_read_file(&path) {
        return Err(ToolError::PermissionDenied(
            format!("Cannot read: {}", path)
        ));
    }
    // 执行操作
}
```

## 异步工具

### 异步 HTTP 请求

```rust
use openclaw_tools::{tool, ToolOutput, ExecutionContext};

#[tool(
    name = "http_get",
    description = "Perform HTTP GET request",
)]
async fn http_get(ctx: ExecutionContext, url: String) -> Result<ToolOutput, ToolError> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(&url)
        .header("User-Agent", "OpenClaw/1.0")
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("Request failed: {}", e)))?;
    
    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| ToolError::Execution(e.to_string()))?;
    
    Ok(ToolOutput::json(serde_json::json!({
        "status": status,
        "body": body,
    })))
}
```

### 异步文件操作

```rust
#[tool(
    name = "read_file",
    description = "Read file contents",
)]
async fn read_file(ctx: ExecutionContext, path: String) -> Result<ToolOutput, ToolError> {
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ToolError::Execution(format!("Read failed: {}", e)))?;
    
    Ok(ToolOutput::text(contents))
}

#[tool(
    name = "write_file",
    description = "Write content to a file",
)]
async fn write_file(
    ctx: ExecutionContext,
    path: String,
    content: String,
) -> Result<ToolOutput, ToolError> {
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;
    
    Ok(ToolOutput::text(format!("Written {} bytes to {}", content.len(), path)))
}
```

## 错误处理

### 工具错误类型

```rust
use openclaw_tools::{ToolError, ToolErrorKind};

match result {
    Ok(output) => Ok(output),
    Err(ToolError::Execution(msg)) => {
        // 执行失败，可重试
        Err(ToolError::Execution(msg))
    }
    Err(ToolError::PermissionDenied(msg)) => {
        // 权限不足，不应重试
        Err(ToolError::PermissionDenied(msg))
    }
    Err(ToolError::InvalidInput(msg)) => {
        // 输入参数错误，不应重试
        Err(ToolError::InvalidInput(msg))
    }
    Err(ToolError::RateLimited(msg)) => {
        // 限流，稍后重试
        Err(ToolError::RateLimited(msg))
    }
}
```

### 返回错误信息

```rust
#[tool(name = "divide")]
fn divide(ctx: ExecutionContext, a: f64, b: f64) -> Result<ToolOutput, ToolError> {
    if b == 0.0 {
        return Err(ToolError::Execution(
            "Division by zero is not allowed".to_string()
        ));
    }
    Ok(ToolOutput::json(serde_json::json!({
        "result": a / b
    })))
}
```

## 工具链

### 顺序执行

```rust
// 工具 A 的输出作为工具 B 的输入
let result_a = tool_a.execute(args_a).await?;
let input_b = extract_value_from_result(&result_a)?;
let result_b = tool_b.execute(input_b).await?;
```

### 条件执行

```rust
#[tool(name = "process_if_exists")]
async fn process_if_exists(
    ctx: ExecutionContext,
    path: String,
) -> Result<ToolOutput, ToolError> {
    if tokio::fs::metadata(&path).await.is_ok() {
        // 文件存在，处理
        let contents = tokio::fs::read_to_string(&path).await?;
        Ok(ToolOutput::text(format!("Processed: {}", contents)))
    } else {
        // 文件不存在，返回提示
        Ok(ToolOutput::text(format!("File not found: {}", path)))
    }
}
```

### 并行执行

```rust
use tokio::task::JoinSet;

#[tool(name = "parallel_fetch")]
async fn parallel_fetch(
    ctx: ExecutionContext,
    urls: Vec<String>,
) -> Result<ToolOutput, ToolError> {
    let mut set = JoinSet::new();
    
    for url in urls {
        set.spawn(async move {
            let client = reqwest::Client::new();
            client.get(&url).send().await?.text().await
        });
    }
    
    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(text)) => results.push(text),
            Ok(Err(e)) => results.push(format!("Error: {}", e)),
            Err(e) => results.push(format!("Join error: {}", e)),
        }
    }
    
    Ok(ToolOutput::json(serde_json::json!({ "results": results })))
}
```

## 工具输出格式化

### JSON 结构输出

```rust
Ok(ToolOutput::json(serde_json::json!({
    "status": "success",
    "data": {
        "id": 123,
        "name": "example"
    },
    "metadata": {
        "processed_at": chrono::Utc::now().to_rfc3339()
    }
})))
```

### 表格输出

```rust
Ok(ToolOutput::table(vec![
    ["Name", "Age", "City"],
    ["Alice", "30", "Beijing"],
    ["Bob", "25", "Shanghai"],
]))
```

### 分页输出

```rust
Ok(ToolOutput::paginated(
    items,
    PageMeta {
        page: 1,
        per_page: 10,
        total: 100,
    },
))
```

## 工具测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculator_add() {
        let ctx = ExecutionContext::mock();
        let result = calculator(ctx, 2.0, 3.0, "add".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "5");
    }

    #[tokio::test]
    async fn test_calculator_divide_by_zero() {
        let ctx = ExecutionContext::mock();
        let result = calculator(ctx, 1.0, 0.0, "div".to_string()).await;
        assert!(result.is_err());
    }
}
```

### 集成测试

```rust
#[tokio::test]
async fn test_tool_in_runtime() {
    let config = RuntimeConfig::builder()
        .provider("mock")
        .tool("hello", hello)
        .build()
        .unwrap();
    
    let runtime = Runtime::new(config).await.unwrap();
    let response = runtime.chat("Say hello to Alice").await.unwrap();
    
    assert!(response.contains("Alice"));
}
```

## 工具注册表

### 动态注册

```rust
let registry = ToolRegistry::new();

registry.register(hello)?;
registry.register(get_time)?;
registry.register(calculator)?;

let config = RuntimeConfig::builder()
    .provider("mock")
    .tool_registry(registry)
    .build()?;
```

### 按条件注册

```rust
// 只在特定 provider 下注册
if config.provider() == "anthropic" {
    registry.register(anthropic_specific_tool())?;
}
```

### 工具发现

```rust
// 从目录加载所有工具
let plugins_dir = "./tools";
for entry in std::fs::read_dir(plugins_dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) == Some("so") {
        registry.load_from_plugin(&path)?;
    }
}
```

## 下一步

- [插件开发](plugin-development.md)
- [会话管理](session-management.md)
- [内置工具](../../guide/tools.md)
