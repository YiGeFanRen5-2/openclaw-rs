# 创建第一个项目

本指南将引导你创建一个完整的 OpenClaw 应用，从项目初始化到运行第一个交互式会话。

## 前置条件

- 已完成 [安装指南](installation.md)
- 拥有至少一个 AI Provider 的 API Key（Anthropic、OpenAI 或 Gemini）

## 1. 创建项目

```bash
# 创建新的 Rust 项目
cargo new my-openclaw-app
cd my-openclaw-app

# 添加 OpenClaw 依赖
cargo add openclaw-runtime openclaw-tools
```

或使用现有项目，只需添加依赖。

## 2. 初始化配置

```bash
# 创建配置文件
mkdir -p config
cat > config/app.toml << 'EOF'
provider = "anthropic"
model = "claude-3-haiku-20240307"
window_capacity = 20
session_store = "./sessions"
plugins = []
EOF
```

## 3. 编写第一个应用

创建 `src/main.rs`：

```rust
use openclaw_runtime::{Runtime, RuntimeConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 API Key
    let api_key = env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set");

    // 构建运行时配置
    let config = RuntimeConfig::builder()
        .provider("anthropic")
        .model("claude-3-haiku-20240307")
        .api_key(api_key)
        .build()?;

    // 创建运行时实例
    let runtime = Runtime::new(config).await?;

    // 创建交互式 REPL
    println!("OpenClaw REPL - Type 'exit' to quit\n");
    println!("> ");

    let stdin = std::io::stdin();
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            break;
        }

        // 发送消息并获取响应
        match runtime.chat(input).await {
            Ok(response) => println!("{}\n\n> ", response),
            Err(e) => eprintln!("Error: {}\n\n> ", e),
        }
    }

    Ok(())
}
```

## 4. 运行 REPL

```bash
# 设置 API Key
export ANTHROPIC_API_KEY="sk-ant-..."

# 运行
cargo run

# 输出示例:
# OpenClaw REPL - Type 'exit' to quit
#
# > hello
# Hello! I'm Claude, here to help you. How can I assist you today?
#
# >
```

## 5. 一次性命令模式

不需要交互式 REPL，直接运行一条消息：

```rust
use openclaw_runtime::{Runtime, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig::builder()
        .provider("anthropic")
        .model("claude-3-haiku-20240307")
        .api_key(std::env::var("ANTHROPIC_API_KEY")?)
        .build()?;

    let runtime = Runtime::new(config).await?;

    let response = runtime
        .chat("What is the capital of Japan?")
        .await?;

    println!("{}", response);
    Ok(())
}
```

## 6. 使用工具调用

创建带工具能力的应用：

```rust
use openclaw_runtime::{Runtime, RuntimeConfig};
use openclaw_tools::{tool, ToolOutput, ExecutionContext};

#[tool(
    name = "get_weather",
    description = "Get current weather for a city",
    input_schema = {
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }
)]
fn get_weather(ctx: ExecutionContext, city: String) -> Result<ToolOutput, Box<dyn std::error::Error>> {
    // 实际项目中调用天气 API
    let weather = format!("Sunny, 22°C in {}", city);
    Ok(ToolOutput::json(serde_json::json!({ "weather": weather })))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig::builder()
        .provider("anthropic")
        .model("claude-3-haiku-20240307")
        .api_key(std::env::var("ANTHROPIC_API_KEY")?)
        .tool("get_weather", get_weather)
        .build()?;

    let runtime = Runtime::new(config).await?;

    let response = runtime
        .chat("What's the weather in Tokyo?")
        .await?;

    println!("{}", response);
    Ok(())
}
```

## 7. 会话管理

### 保存和恢复会话

```rust
use openclaw_runtime::{Runtime, RuntimeConfig, SessionId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig::builder()
        .provider("anthropic")
        .model("claude-3-haiku-20240307")
        .api_key(std::env::var("ANTHROPIC_API_KEY")?)
        .session_store("./sessions")
        .build()?;

    let runtime = Runtime::new(config).await?;

    // 创建新会话
    let session_id = runtime.create_session().await?;
    println!("Session: {}", session_id);

    // 在会话中对话
    runtime.chat_session(&session_id, "Hello!").await?;
    runtime.chat_session(&session_id, "What's my previous message?").await?;

    // 列出所有会话
    let sessions = runtime.list_sessions().await?;
    for session in sessions {
        println!("- {} ({})", session.id, session.updated_at);
    }

    // 恢复指定会话
    let resumed = runtime.resume_session(&session_id).await?;
    let response = resumed.chat("Continue from where we left off.").await?;
    println!("{}", response);

    Ok(())
}
```

## 8. 使用 Mock Provider 测试

无需 API Key，用 Mock Provider 开发测试：

```bash
cat > config/mock.toml << 'EOF'
provider = "mock"
model = "mock-1"
EOF

# 用配置文件运行
openclaw repl --config config/mock.toml
```

或在代码中：

```rust
let config = RuntimeConfig::builder()
    .provider("mock")
    .model("mock-1")
    .build()?;
```

## 9. 项目结构推荐

```
my-openclaw-app/
├── Cargo.toml
├── config/
│   └── app.toml
├── sessions/          # 会话存储
├── plugins/           # 插件目录
├── src/
│   └── main.rs
└── target/
```

## 常见问题

### Q: 收到 "API key not found" 错误

```bash
# 检查环境变量
echo $ANTHROPIC_API_KEY

# 临时设置
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Q: 模型响应很慢

- 检查网络连接
- 使用更小的模型（如 haiku 而非 opus）
- 减少 `window_capacity`

### Q: 如何调试工具调用？

```rust
// 启用调试日志
env_logger::init();
RUST_LOG=openclaw_runtime=debug cargo run
```

## 下一步

- [深入配置](configuration.md)
- [使用工具系统](../guide/tools.md)
- [开发插件](../guide/plugins.md)
- [会话持久化](../guide/persistence.md)
