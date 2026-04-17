# Tool Development Guide

本指南说明如何在 OpenClaw Rust 升级中添加新工具。

---

## Overview

工具是 OpenClaw 的核心能力单元，例如 `list_files`、`read_file`。每个工具：

- 实现 `Tool` trait
- 声明 `Permission`（访问控制）
- 提供 `ToolSchema`（输入/输出描述）
- 执行逻辑（同步函数）

---

## 实现一个工具

### 1. 创建 struct

工具通常是无状态的，用零大小类型（ZST）或持有配置：

```rust
// crates/tools/src/lib.rs

pub struct EchoTool {
    prefix: String,
}

impl EchoTool {
    pub fn new(prefix: &str) -> Self {
        Self { prefix: prefix.to_string() }
    }
}
```

### 2. 实现 `Tool` trait

```rust
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Echo back the input with optional prefix"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Echo input".into()),
            properties: Some(serde_json::json!({
                "message": { "type": "string", "description": "Message to echo" }
            })),
            required: Some(vec!["message".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Echo output".into()),
            properties: Some(serde_json::json!({
                "echoed": { "type": "string" }
            })),
            required: Some(vec!["echoed".into()]),
        }
    }

    fn permission(&self) -> Permission {
        // Echo 不需要特殊权限
        Permission::Safe
    }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let message = input.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'message' parameter".into()))?;

        let echoed = format!("{}{}", self.prefix, message);

        Ok(serde_json::json!({
            "echoed": echoed,
            "prefix_used": self.prefix
        }))
    }
}
```

### 3. 可选：添加构造函数

```rust
impl EchoTool {
    pub fn with_prefix(prefix: &str) -> Self {
        Self { prefix: prefix.to_string() }
    }
}
```

---

## 权限设计

### Permission 类型

| 类型 | 用途 | 字段 |
|------|------|------|
| `Safe` | 无需特殊权限 | - |
| `Filesystem` | 文件访问 | `allowlist: Vec<String>`, `writable: bool` |
| `Shell` | 命令执行 | `allowlist: Vec<String>`, `arg_pattern: Option<String>` |
| `Network` | 网络连接 | `destinations: Vec<String>`, `protocols: Vec<String>`, `max_connections: usize` |
| `Custom` | 自定义检查器 | `checker: String`, `config: JsonValue` |

### 路径白名单示例

```rust
Permission::Filesystem {
    allowlist: vec![
        "/home".into(),
        "/tmp".into(),
        "/workspace".into(),
    ],
    writable: false, // 只读
}
```

**注意**：路径使用前缀匹配（`starts_with`），所以 `/home/user` 允许访问 `/home/user/data`。

---

## Schema 定义

使用 `serde_json::json!` 宏快速构建 JSON Schema：

```rust
fn input_schema(&self) -> ToolSchema {
    ToolSchema {
        r#type: "object".into(),
        description: Some("描述".into()),
        properties: Some(serde_json::json!({
            "param1": { "type": "string" },
            "param2": { "type": "integer", "minimum": 0, "maximum": 100 },
            "param3": { "type": "array", "items": { "type": "string" } }
        })),
        required: Some(vec!["param1".into()]),
    }
}
```

---

## 错误处理

在 `execute()` 中返回 `ToolError`：

| 变体 | 何时使用 |
|------|----------|
| `NotFound` | 请求的资源不存在 |
| `PermissionDenied` | 权限检查失败 |
| `InvalidInput` | 参数缺失或类型错误 |
| `ExecutionFailed` | 运行时错误（I/O、外部命令失败） |
| `Timeout` | 执行超时 |
| `ResourceLimit` | 超出资源配额 |
| `Io` | 底层 I/O 错误 |

---

## 注册工具

工具必须在运行时手动注册。在 `crates/runtime/src/lib.rs` 的 `Runtime::new()` 或后续调用 `register_tool()`：

```rust
let mut runtime = Runtime::new(config)?;
runtime.register_tool(Box::new(EchoTool::new(">>> ")))?;
runtime.register_tool(Box::new(ListFilesTool::new()))?;
```

**注意**：工具放入 `Box<dyn Tool>`，所以需要 `Send + Sync + 'static`。

---

## 运行测试

### 单元测试

在 `crates/tools/tests/integration.rs` 中添加：

```rust
#[test]
fn test_echo_tool() {
    let tool = EchoTool::new(">>> ");
    let input = json!({ "message": "hello" });
    let result = tool.execute(input).unwrap();
    assert_eq!(result.get("echoed").unwrap(), ">>> hello");
}
```

运行：
```bash
cargo test --package tools
```

### 集成测试（Node.js）

在 `node-bridge/test-poc.js` 中添加工具调用验证：

```javascript
const result = runtime.executeTool(sessionId, 'echo', JSON.stringify({ message: 'test' }));
const data = JSON.parse(result);
console.log(`   ✅ echo returned: ${data.echoed}`);
```

---

## Checklist

添加新工具时确认：

- [ ] `Tool` trait 实现完整（4 个 required 方法 + `execute`）
- [ ] `Permission` 正确设置（最小权限原则）
- [ ] `ToolSchema` 准确描述输入/输出
- [ ] `execute` 返回 `Result<JsonValue, ToolError>`
- [ ] 参数验证（存在性、类型、范围）
- [ ] 错误使用正确的 `ToolError` 变体
- [ ] 添加单元测试（正常路径、错误路径）
- [ ] 在 `Runtime::new()` 或主程序中注册
- [ ] 更新 Node bridge 测试（可选）

---

## 示例：时钟工具

完整示例：

```rust
/// 获取当前时间（UTC）
pub struct NowTool;

impl Tool for NowTool {
    fn name(&self) -> &'static str { "now" }
    fn description(&self) -> &'static str { "返回当前 UTC 时间戳" }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("无参数".into()),
            properties: Some(serde_json::json!({})),
            required: Some(vec![]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("当前时间".into()),
            properties: Some(serde_json::json!({
                "timestamp": { "type": "string", "format": "date-time" },
                "unix_seconds": { "type": "integer" }
            })),
            required: Some(vec!["timestamp".into(), "unix_seconds".into()]),
        }
    }
    fn permission(&self) -> Permission { Permission::Safe }
    fn execute(&self, _input: JsonValue) -> Result<JsonValue, ToolError> {
        let now = chrono::Utc::now();
        Ok(serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "unix_seconds": now.timestamp()
        }))
    }
}
```

---

## Next Steps

- 查看 Phase 3 计划：实现**沙箱隔离**（Linux namespaces）
- 设计**异步工具**支持（`async fn execute`）
- 考虑**A/B 测试框架**（并行运行新旧实现）

---

**Happy tooling!** 🛠️