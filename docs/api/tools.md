# Tools API

OpenClaw's tool system provides a flexible framework for extending functionality.

## Tool Trait

All tools implement the `Tool` trait:

```rust
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> ToolSchema;
    fn output_schema(&self) -> ToolSchema;
    fn permission(&self) -> Permission;
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError>;
}
```

## Built-in Tools

### File Operations

| Tool | Description |
|------|-------------|
| `list_files` | List directory contents |
| `read_file` | Read file contents |
| `write_file` | Write file to path |
| `edit_file` | Edit file (exact replace) |
| `file_info` | Get file metadata |

### HTTP

| Tool | Description |
|------|-------------|
| `http_request` | Perform HTTP GET/POST requests |

### Image

| Tool | Description |
|------|-------------|
| `image_info` | Get image dimensions, format |
| `image_formats` | List supported formats |

### Health

| Tool | Description |
|------|-------------|
| `health_check` | Single URL health check |
| `batch_health_check` | Multiple URL health check |

### Validation

| Tool | Description |
|------|-------------|
| `validate_json` | Validate JSON against Schema |
| `validate_tool_input` | Validate tool input |

### Storage

| Tool | Description |
|------|-------------|
| `json_store_set` | Store JSON value |
| `json_store_get` | Get JSON value |
| `json_store_list` | List all keys |

### Text

| Tool | Description |
|------|-------------|
| `hash` | Hash string |
| `uuid` | Generate UUID |
| `random_string` | Random string |
| `text_stats` | Text statistics |

## Custom Tools

### Creating a Custom Tool

```rust
use openclaw_core::tools::{Tool, ToolSchema, Permission};
use serde_json::json;

pub struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &'static str {
        "my_tool"
    }

    fn description(&self) -> &'static str {
        "A custom tool"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("My tool input".into()),
            properties: Some(json!({
                "param": { "type": "string" }
            })),
            required: Some(vec!["param".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Result".into()),
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        // Process input
        Ok(json!({ "result": "success" }))
    }
}
```

### Registering a Tool

```rust
use openclaw_core::tools::{ToolRegistry, register_builtin_tools};

let mut registry = ToolRegistry::new();
register_builtin_tools(&mut registry);

// Register custom tool
registry.register(MyTool);
```

## Permissions

| Permission | Description |
|------------|-------------|
| `Safe` | No restrictions |
| `Filesystem { allowlist, writable }` | File access control |
| `Shell { allowlist }` | Command execution |
| `Network { destinations, protocols }` | Network access |
| `Custom { checker, config }` | Custom checker |

## Error Handling

```rust
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}
```
