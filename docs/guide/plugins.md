# 插件系统

OpenClaw 的插件系统允许扩展运行时功能，如添加新工具、修改提示词、拦截模型输出等。

## 插件类型

插件可以定义三种 hook：

- **Prompt Hook**: 修改用户 prompt 或系统提示词
- **Tool Hook**: 在工具执行前后进行拦截/转换
- **Model Hook**: 在模型输入/输出后处理

## 插件生命周期

1. **加载**: 从 `.so`/`.dll` 或 Rust crate 加载
2. **初始化**: 调用 `Plugin::initialize()` 进行设置
3. **注册**: hook 被添加到 `PluginManager`
4. **执行**: 按注册顺序在对应阶段触发
5. **卸载**: 可选，支持热重载时使用

## 编写插件

### Rust 插件示例

```rust
use openclaw_plugin::{Plugin, PluginMetadata, Hook, HookContext, HookStage, PromptHook, ToolHook, ModelHook};

#[derive(Clone)]
struct MyPlugin;

#[async_trait::async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Demo plugin".to_string(),
        }
    }

    async fn initialize(&self, _ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
        println!("my-plugin initialized");
        Ok(())
    }

    fn prompt_hooks(&self) -> Vec<Hook<PromptHook>> {
        vec![
            Hook::new(HookStage::BeforeUser, |ctx| {
                // 修改用户 prompt
                ctx.prompt.push_str("\n[Plugin note: customized]");
                Ok(())
            }),
        ]
    }

    fn tool_hooks(&self) -> Vec<Hook<ToolHook>> {
        vec![
            Hook::new(HookStage::AfterTool, |ctx| {
                // 记录工具输出
                println!("Tool {} returned: {:?}", ctx.tool_name, ctx.output);
                Ok(())
            }),
        ]
    }

    fn model_hooks(&self) -> Vec<Hook<ModelHook>> {
        vec![]
    }
}
```

### 构建为动态库

在 `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]
```

构建：

```bash
cargo build --release --target x86_64-unknown-linux-gnu
# 输出: target/x86_64-unknown-linux-gnu/release/libmy_plugin.so
```

### 配置加载

在 OpenClaw 配置中：

```toml
plugins = ["./plugins/libmy_plugin.so"]
```

或在 CLI:

```bash
openclaw repl --plugin ./plugins/libmy_plugin.so
```

## Hook 阶段

| 阶段 | 说明 |
|------|------|
| `BeforePrompt` | 在 prompt 构建前 |
| `AfterPrompt` | 在 prompt 构建后 |
| `BeforeTool` | 工具执行前 |
| `AfterTool` | 工具执行后 |
| `BeforeModel` | 模型调用前 |
| `AfterModel` | 模型调用后 |

## 插件通信

插件之间可以通过 `HookContext` 共享数据：

```rust
ctx.state.insert("my_key".to_string(), serde_json::json!(value));
```

后续插件可读取。

## 调试插件

启用日志：

```bash
RUST_LOG=debug openclaw repl
```

插件中的 `println!` 会输出到标准错误。

---

下一步：[热重载开发](hot-reload.md)
