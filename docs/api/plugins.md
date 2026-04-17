# Plugin System

OpenClaw's plugin system enables dynamic extension of functionality.

## Plugin Structure

A plugin consists of:
- `plugin.json` - Manifest file
- Tool implementations
- Hook handlers (optional)

## Plugin Manifest

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "A sample plugin",
  "author": "Your Name",
  "hooks": [
    { "name": "before_tool_call" },
    { "name": "after_message" }
  ],
  "tools": [
    {
      "name": "my_tool",
      "description": "A custom tool",
      "input_schema": {
        "type": "object",
        "properties": {
          "param": { "type": "string" }
        }
      }
    }
  ],
  "permissions": [
    { "type": "safe" }
  ]
}
```

## Using Plugin Registry

```rust
use openclaw_plugins::{PluginRegistry, PluginSource};

let mut registry = PluginRegistry::new();

// Register from file
registry.register_from_file("/path/to/plugin.json")?;

// List plugins
for entry in registry.list() {
    println!("{} v{}", entry.manifest.name, entry.manifest.version);
}
```

## Hooks

| Hook | Trigger | Use Case |
|------|---------|----------|
| `before_tool_call` | Before tool execution | Logging, validation |
| `after_tool_call` | After tool execution | Result processing |
| `before_message` | Before LLM call | Prompt modification |
| `after_message` | After LLM response | Response processing |

## Example Hook

```rust
use openclaw_plugins::{Hook, HookContext, HookResult, Plugin};

pub struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn name(&self) -> &str { "My Plugin" }
    fn version(&self) -> &str { "1.0.0" }

    async fn on_hook(&self, hook: &Hook, ctx: &HookContext) -> HookResult {
        match hook {
            Hook::BeforeToolCall => {
                tracing::info!("Tool {} called", ctx.tool_name);
            }
            _ => {}
        }
        HookResult::unchanged()
    }
}
```

## Hot Reload

Plugins can be loaded/unloaded without restart:

```rust
use openclaw_plugins::hot::PluginLoader;

let loader = PluginLoader::new();

// Load plugin
loader.load("path/to/plugin.so")?;

// Hot reload
loader.reload("plugin-id")?;

// Unload
loader.unload("plugin-id")?;
```
