# 插件开发

本指南讲解如何开发 OpenClaw 插件，包括 Hook 系统、生命周期管理、插件通信和调试技巧。

## 插件概述

OpenClaw 插件是可扩展的功能模块，通过 Hook 机制在运行时注入逻辑：

- **Prompt Hook** - 修改提示词
- **Tool Hook** - 拦截/处理工具调用
- **Model Hook** - 处理模型输入输出

## 项目结构

```
my-plugin/
├── Cargo.toml
├── src/
│   └── lib.rs
└── build.sh
```

### Cargo.toml

```toml
[package]
name = "my-openclaw-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
openclaw-plugin = { path = "../crates/plugin" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

## 创建插件

### 基础结构

```rust
use openclaw_plugin::{
    Plugin, PluginMetadata, Hook, HookContext,
    HookStage, PromptHook, ToolHook, ModelHook,
    Permission, PermissionSet,
};
use async_trait::async_trait;

#[derive(Clone)]
pub struct MyPlugin {
    config: MyPluginConfig,
}

#[derive(Clone, Default)]
pub struct MyPluginConfig {
    pub enabled: bool,
    pub custom_field: String,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            config: MyPluginConfig::default(),
        }
    }

    pub fn with_config(config: MyPluginConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "A demo plugin".to_string(),
            author: Some("Your Name".to_string()),
            homepage: Some("https://github.com/you/my-plugin".to_string()),
        }
    }

    async fn initialize(&self, ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
        println!("my-plugin initialized with config: {:?}", self.config);
        
        // 可以在这里进行异步初始化
        // 比如连接数据库、加载配置等
        ctx.state().insert("initialized".to_string(), serde_json::json!(true));
        Ok(())
    }

    fn prompt_hooks(&self) -> Vec<Hook<PromptHook>> {
        vec![
            Hook::new(HookStage::BeforePrompt, self.before_prompt_hook()),
            Hook::new(HookStage::AfterPrompt, self.after_prompt_hook()),
        ]
    }

    fn tool_hooks(&self) -> Vec<Hook<ToolHook>> {
        vec![
            Hook::new(HookStage::BeforeTool, self.before_tool_hook()),
            Hook::new(HookStage::AfterTool, self.after_tool_hook()),
        ]
    }

    fn model_hooks(&self) -> Vec<Hook<ModelHook>> {
        vec![
            Hook::new(HookStage::BeforeModel, self.before_model_hook()),
            Hook::new(HookStage::AfterModel, self.after_model_hook()),
        ]
    }

    fn permissions(&self) -> PermissionSet {
        vec![
            Permission::Internet,
            Permission::FileRead,
        ].into()
    }
}
```

## Prompt Hook

### BeforePrompt Hook

在构建 prompt 之前执行：

```rust
impl MyPlugin {
    fn before_prompt_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            // 添加系统提示词前缀
            ctx.prompt.push_str("\n\n[Additional context from plugin]");
            Ok(())
        }
    }
}
```

### AfterPrompt Hook

在构建 prompt 之后、发送给模型之前执行：

```rust
impl MyPlugin {
    fn after_prompt_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            // 可以检查/修改最终 prompt
            println!("Final prompt length: {}", ctx.prompt.len());
            Ok(())
        }
    }
}
```

### 常见用途

```rust
// 动态注入上下文
fn before_prompt_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
    move |mut ctx: HookContext| {
        // 添加用户相关信息
        let user_context = ctx.state().get("user_info")
            .map(|v| v.as_str().unwrap_or(""))
            .unwrap_or("");
        
        ctx.prompt.push_str(&format!("\n\n[User Context: {}]", user_context));
        Ok(())
    }
}
```

## Tool Hook

### BeforeTool Hook

工具执行前拦截：

```rust
impl MyPlugin {
    fn before_tool_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            println!("Tool '{}' about to execute", ctx.tool_name);
            
            // 可以修改工具参数
            if let Some(args) = ctx.tool_args.as_object_mut() {
                // 在参数中添加插件注入的字段
                args.insert("plugin_tag".to_string(), serde_json::json!("my-plugin"));
            }
            
            // 可以拒绝执行
            // return Err("Tool execution blocked".into());
            
            Ok(())
        }
    }
}
```

### AfterTool Hook

工具执行后拦截：

```rust
impl MyPlugin {
    fn after_tool_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            println!("Tool '{}' returned: {:?}", ctx.tool_name, ctx.tool_output);
            
            // 可以修改工具输出
            if let Some(output) = ctx.tool_output.as_object_mut() {
                // 添加元数据
                output.insert("_plugin_processed".to_string(), serde_json::json!(true));
            }
            
            Ok(())
        }
    }
}
```

### 工具过滤

```rust
fn before_tool_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
    move |ctx: HookContext| {
        // 只对特定工具进行处理
        match ctx.tool_name.as_str() {
            "http_get" | "http_post" => {
                // 添加认证头
                if let Some(args) = ctx.tool_args.as_object_mut() {
                    if let Some(headers) = args.get_mut("headers").and_then(|h| h.as_object_mut()) {
                        headers.insert("Authorization".to_string(), serde_json::json!("Bearer token"));
                    }
                }
            }
            "dangerous_tool" => {
                return Err("Tool blocked by plugin".into());
            }
            _ => {}
        }
        Ok(())
    }
}
```

## Model Hook

### BeforeModel Hook

模型调用前拦截：

```rust
impl MyPlugin {
    fn before_model_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            println!("Calling model...");
            
            // 可以修改发送给模型的请求
            // 例如：添加自定义头、修改模型参数
            
            Ok(())
        }
    }
}
```

### AfterModel Hook

模型调用后拦截：

```rust
impl MyPlugin {
    fn after_model_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
        move |ctx: HookContext| {
            println!("Model responded");
            
            // 可以处理模型输出
            // 例如：过滤敏感内容、添加后处理
            
            Ok(())
        }
    }
}
```

## 状态共享

### 插件间通信

```rust
// 在插件 A 中设置状态
async fn initialize(&self, ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
    ctx.state().insert("shared_key".to_string(), serde_json::json!({
        "value": "shared between plugins",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }));
    Ok(())
}

// 在插件 B 中读取状态
fn before_prompt_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
    move |ctx: HookContext| {
        if let Some(shared) = ctx.state().get("shared_key") {
            println!("Found shared state: {:?}", shared);
        }
        Ok(())
    }
}
```

### 状态作用域

```rust
// 会话级状态
ctx.session_state().insert("session_data".to_string(), serde_json::json!(...));

// 全局状态
ctx.global_state().insert("global_data".to_string(), serde_json::json!(...));
```

## 异步插件

完全支持异步初始化和异步 Hook：

```rust
#[async_trait]
impl Plugin for AsyncPlugin {
    async fn initialize(&self, ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
        // 异步数据库连接
        let db = self.database.connect().await?;
        ctx.state().insert("db".to_string(), serde_json::json!({
            "connected": true
        }));
        Ok(())
    }

    async fn before_prompt_async(&self, mut ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
        // 异步获取额外上下文
        let context = self.external_api.fetch_context().await?;
        ctx.prompt.push_str(&context);
        Ok(())
    }
}
```

## 配置加载

### 从配置文件读取

```rust
impl MyPlugin {
    pub fn load_from_config(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_content = std::fs::read_to_string(path)?;
        let config: MyPluginConfig = toml::from_str(&config_content)?;
        Ok(Self::with_config(config))
    }
}
```

### 通过环境变量

```rust
impl MyPlugin {
    fn from_env() -> Self {
        let config = MyPluginConfig {
            enabled: std::env::var("MY_PLUGIN_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
            custom_field: std::env::var("MY_PLUGIN_FIELD")
                .unwrap_or_default(),
        };
        Self::with_config(config)
    }
}
```

## 构建和发布

### 构建为动态库

```bash
# Linux
cargo build --release --target x86_64-unknown-linux-gnu
# 输出: target/x86_64-unknown-linux-gnu/release/libmy_plugin.so

# macOS
cargo build --release --target x86_64-apple-darwin
# 输出: target/x86_64-apple-darwin/release/libmy_plugin.dylib

# Windows
cargo build --release --target x86_64-pc-windows-msvc
# 输出: target/x86_64-pc-windows-msvc/release/my_plugin.dll
```

### 跨平台构建脚本

```bash
#!/bin/bash
# build.sh

PLUGIN_NAME="my_plugin"

# Linux
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/lib${PLUGIN_NAME}.so plugins/

# macOS (需要 macOS 机器或交叉编译)
# cargo build --release --target x86_64-apple-darwin

# Windows (需要 Windows 机器或交叉编译)
# cargo build --release --target x86_64-pc-windows-msvc

echo "Build complete: plugins/${PLUGIN_NAME}.so"
```

## 调试插件

### 日志调试

```rust
use log::{info, warn, error};

async fn initialize(&self, ctx: HookContext) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing my-plugin");
    
    if let Err(e) = self.setup().await {
        error!("Setup failed: {}", e);
        return Err(e.into());
    }
    
    info!("my-plugin initialized successfully");
    Ok(())
}
```

### 运行时调试

```bash
# 启用详细日志
RUST_LOG=openclaw_plugin=debug,my_plugin=trace openclaw repl

# 特定插件日志
RUST_LOG=my_plugin=debug openclaw repl
```

### Hook 调试

```rust
fn before_prompt_hook(&self) -> impl Fn(HookContext) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static {
    move |ctx: HookContext| {
        eprintln!("[DEBUG] before_prompt: {}", ctx.prompt);
        Ok(())
    }
}
```

## 常用插件模式

### 认证插件

```rust
struct AuthPlugin {
    jwt_secret: String,
}

impl AuthPlugin {
    fn authenticate_request(&self, ctx: &HookContext) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(token) = ctx.headers.get("Authorization") {
            let claims = self.validate_jwt(token)?;
            ctx.state().insert("user_id".to_string(), serde_json::json!(claims.sub));
        }
        Ok(())
    }
}
```

### 日志记录插件

```rust
struct LoggerPlugin {
    log_channel: mpsc::Sender<LogEntry>,
}
```

### 限流插件

```rust
struct RateLimitPlugin {
    limiter: Mutex<RateLimiter>,
}

impl RateLimitPlugin {
    fn check_rate_limit(&self, ctx: &HookContext) -> Result<(), Box<dyn std::error::Error>> {
        let key = ctx.session_id.clone().unwrap_or_default();
        if !self.limiter.lock().unwrap().check(&key) {
            return Err("Rate limit exceeded".into());
        }
        Ok(())
    }
}
```

## 下一步

- [自定义工具](custom-tools.md)
- [会话管理](session-management.md)
- [官方插件实现](../../guide/plugins.md)
