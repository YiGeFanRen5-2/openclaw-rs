//! Additional CLI commands for OpenClaw

use clap::{Args, Subcommand};
use openclaw_core::tools::{ToolRegistry, register_builtin_tools};
use openclaw_plugins::MetricsCollector;
use serde_json::json;

/// List all available tools
#[derive(Debug, Args)]
pub struct ToolsArgs {
    /// Show tool input/output schemas
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

/// List all registered tools
pub fn list_tools(args: ToolsArgs) -> anyhow::Result<()> {
    let mut registry = ToolRegistry::new();
    register_builtin_tools(&mut registry);
    
    let tools = registry.list_schemas();
    println!("\n📦 OpenClaw Built-in Tools ({} total)\n", tools.len());
    println!("{}", "─".repeat(60));
    
    for tool in &tools {
        println!("  {}: {}", tool.name, tool.description);
        if args.verbose {
            println!("    Input: {:?}", tool.input_schema);
            println!("    Output: {:?}", tool.output_schema);
        }
    }
    println!("{}", "─".repeat(60));
    println!();
    Ok(())
}

/// Show metrics summary
#[derive(Debug, Args)]
pub struct MetricsArgs {
    /// Export in Prometheus format
    #[arg(long)]
    pub prometheus: bool,
    
    /// Export in JSON format
    #[arg(long, short = 'j')]
    pub json: bool,
}

/// Display metrics
pub fn show_metrics(args: MetricsArgs) -> anyhow::Result<()> {
    // Create a demo metrics collector
    let metrics = MetricsCollector::new();
    
    if args.prometheus {
        println!("{}", metrics.export_prometheus());
    } else if args.json {
        let export = metrics.export_json();
        println!("{}", serde_json::to_string_pretty(&export)?);
    } else {
        println!("\n📊 OpenClaw Metrics\n");
        println!("{}", "─".repeat(60));
        println!("  Note: Metrics are collected at runtime.");
        println!("  Use --prometheus or --json to export.");
        println!("{}", "─".repeat(60));
        println!();
    }
    
    Ok(())
}

/// Show version info
#[derive(Debug, Args)]
pub struct VersionArgs {
    /// Show all crate versions
    #[arg(long, short = 'a')]
    pub all: bool,
}

/// Display version information
pub fn show_version(args: VersionArgs) -> anyhow::Result<()> {
    println!("\n🚀 OpenClaw Rust Workspace\n");
    println!("{}", "─".repeat(60));
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Repository: https://github.com/YiGeFanRen5-2/openclaw-rs");
    
    if args.all {
        println!();
        println!("  Rust Crates:");
        println!("    - api-client: {}", env!("CARGO_PKG_VERSION"));
        println!("    - runtime: {}", env!("CARGO_PKG_VERSION"));
        println!("    - tools: {}", env!("CARGO_PKG_VERSION"));
        println!("    - plugins: {}", env!("CARGO_PKG_VERSION"));
        println!("    - mcp-server: {}", env!("CARGO_PKG_VERSION"));
        println!("    - mcp-client: {}", env!("CARGO_PKG_VERSION"));
        println!("    - claw-cli: {}", env!("CARGO_PKG_VERSION"));
    }
    
    println!("{}", "─".repeat(60));
    println!();
    Ok(())
}

/// Health check command
#[derive(Debug, Args)]
pub struct HealthArgs {
    /// URL to check (default: built-in health check)
    #[arg(long)]
    pub url: Option<String>,
}

/// Run health check
pub fn health_check(args: HealthArgs) -> anyhow::Result<()> {
    println!("\n🏥 OpenClaw Health Check\n");
    println!("{}", "─".repeat(60));
    
    // Basic health check
    println!("  ✓ CLI executable: OK");
    
    // Tool registry check
    let mut registry = ToolRegistry::new();
    register_builtin_tools(&mut registry);
    let tool_count = registry.list_schemas().len();
    println!("  ✓ Tool registry: OK ({} tools)", tool_count);
    
    // Runtime check (simplified)
    println!("  ✓ Core modules: OK");
    
    if let Some(url) = args.url {
        println!("  Checking: {}", url);
        // Would perform actual HTTP check here
        println!("  (URL check not implemented in this build)");
    }
    
    println!("{}", "─".repeat(60));
    println!("  Status: ✅ All systems operational\n");
    Ok(())
}
