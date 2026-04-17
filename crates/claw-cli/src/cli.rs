use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::repl::Repl;

#[derive(Debug, Parser)]
#[command(name = "openclaw", version, about = "OpenClaw CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Repl(ReplArgs),
    Status(StatusArgs),
    Demo(DemoArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum ProviderMode {
    Mock,
    Openai,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone, Args)]
pub struct SharedProviderArgs {
    #[arg(long, value_enum, default_value_t = ProviderMode::Mock)]
    pub provider: ProviderMode,

    #[arg(long)]
    pub api_key: Option<String>,

    #[arg(long)]
    pub base_url: Option<String>,

    #[arg(long, default_value_t = false)]
    pub no_plugin: bool,

    #[arg(long, default_value = "demo")]
    pub plugins: Vec<String>,

    #[arg(long)]
    pub resume: Option<String>,

    #[arg(long, default_value_t = 10)]
    pub window_capacity: usize,

    #[arg(long, hide = true)]
    pub config: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub provider: SharedProviderArgs,
}

#[derive(Debug, Clone, Args)]
pub struct ReplArgs {
    #[arg(long, default_value = "mock-1")]
    pub model: String,

    #[arg(long, default_value = "summarize previous tool output: {{last_tool_output}}")]
    pub prompt: String,

    #[command(flatten)]
    pub provider: SharedProviderArgs,
}

#[derive(Debug, Clone, Args)]
pub struct DemoArgs {
    #[arg(long, default_value = "hello from cli demo")]
    pub message: String,

    #[arg(long, default_value = "summarize previous tool output: {{last_tool_output}}")]
    pub prompt: String,

    #[arg(long, default_value = "mock-1")]
    pub model: String,

    #[command(flatten)]
    pub provider: SharedProviderArgs,
}

impl Default for SharedProviderArgs {
    fn default() -> Self {
        Self {
            provider: ProviderMode::Mock,
            api_key: None,
            base_url: None,
            no_plugin: false,
            plugins: vec!["demo".to_string()],
            resume: None,
            window_capacity: 10,
            config: None,
        }
    }
}

impl Default for DemoArgs {
    fn default() -> Self {
        Self {
            message: "hello from cli demo".to_string(),
            prompt: "summarize previous tool output: {{last_tool_output}}".to_string(),
            model: "mock-1".to_string(),
            provider: SharedProviderArgs::default(),
        }
    }
}

impl Default for ReplArgs {
    fn default() -> Self {
        Self {
            model: "mock-1".to_string(),
            prompt: "summarize previous tool output: {{last_tool_output}}".to_string(),
            provider: SharedProviderArgs::default(),
        }
    }
}

impl Default for StatusArgs {
    fn default() -> Self {
        Self {
            provider: SharedProviderArgs::default(),
        }
    }
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<String> {
        // Detect config path from any command
        let config_path = match &self.command {
            Some(Commands::Repl(args)) => args.provider.config.as_deref(),
            Some(Commands::Status(args)) => args.provider.config.as_deref(),
            Some(Commands::Demo(args)) => args.provider.config.as_deref(),
            None => None,
        };

        // Load config file if specified
        let config = if let Some(path) = config_path {
            match crate::config::Config::from_file(path) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    eprintln!("警告：无法加载配置文件 {}: {}", path, e);
                    None
                }
            }
        } else {
            None
        };

        match self.command.unwrap_or(Commands::Demo(DemoArgs::default())) {
            Commands::Repl(mut args) => {
                if let Some(ref cfg) = config {
                    // Merge provider args from config
                    args.provider = crate::config::merge_provider_args(args.provider, &cfg.provider);
                    // Merge repl-specific args
                    args = crate::config::merge_repl_args(args, &cfg.repl);
                }
                Repl::new().run_interactive(args).await
            }
            Commands::Status(mut args) => {
                if let Some(ref cfg) = config {
                    args.provider = crate::config::merge_provider_args(args.provider, &cfg.provider);
                }
                Repl::new().run_status(args).await
            }
            Commands::Demo(mut args) => {
                if let Some(ref cfg) = config {
                    args.provider = crate::config::merge_provider_args(args.provider, &cfg.provider);
                }
                Repl::new().run_demo(args).await
            }
        }
    }
}
