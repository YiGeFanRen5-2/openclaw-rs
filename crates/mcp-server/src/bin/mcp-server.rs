//! OpenClaw MCP Server - Main Binary
//! This is the production entry point for the MCP server.

use clap::Parser;
use mcp_server::McpServer;
use openclaw_core::OpenClawCore;
use serde_json::json;
use std::sync::Arc;

/// Simple CLI for OpenClaw MCP Server
#[derive(Parser, Debug)]
#[command(version, about = "OpenClaw MCP Server — stdio-based Model Context Protocol server", long_about = None)]
struct Args {
    /// Print version and exit
    #[arg(short, long)]
    version: bool,

    /// List all available tools and exit
    #[arg(short, long)]
    list_tools: bool,

    /// Show details of a specific tool
    #[arg(long)]
    tool_info: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Initialize OpenClaw core
    let core = Arc::new(OpenClawCore::new());
    let tools = core.list_tools();

    if args.version {
        println!("openclaw-mcp-server {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.list_tools {
        for tool in &tools {
            println!("{} — {}", tool.name, tool.description);
        }
        return;
    }

    if let Some(name) = args.tool_info {
        if let Some(tool) = tools.iter().find(|t| t.name == name) {
            println!("Tool: {}", tool.name);
            println!("Description: {}", tool.description);
            println!(
                "Input schema: {}",
                serde_json::to_string_pretty(&tool.input_schema).unwrap()
            );
        } else {
            eprintln!("Tool '{}' not found.", name);
            std::process::exit(1);
        }
        return;
    }

    // Create MCP server with tool executor
    let mut server = McpServer::new("OpenClaw", "0.1.0").with_tool_executor(Box::new({
        let core = Arc::clone(&core);
        move |tool_name, args| match core.execute_tool(tool_name, args) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }));

    // Register tools from core
    for tool in core.list_tools() {
        server.register_tool(mcp_server::McpTool {
            name: tool.name,
            description: tool.description,
            input_schema: json!(tool.input_schema),
        });
    }

    // Register sample resources (in production, these would be discovered)
    server.register_resource(mcp_server::McpResource {
        uri: "file:///README.md".to_string(),
        name: "OpenClaw README".to_string(),
        description: Some("OpenClaw project documentation".to_string()),
        mime_type: Some("text/markdown".to_string()),
    });

    // Register sample prompts
    server.register_prompt(mcp_server::McpPrompt {
        name: "openclaw_assistant".to_string(),
        description: Some("System prompt for OpenClaw assistant mode".to_string()),
        arguments: Some(vec![
            mcp_server::PromptArgument {
                name: "role".to_string(),
                description: "Assistant role/persona".to_string(),
                required: true,
            },
            mcp_server::PromptArgument {
                name: "guidelines".to_string(),
                description: "Additional behavior guidelines".to_string(),
                required: false,
            },
        ]),
        messages: vec![mcp_server::PromptMessage {
            role: "system".to_string(),
            content: mcp_server::PromptContent::Text {
                text: "You are {{role}}. You are powered by OpenClaw. {{guidelines}}".to_string(),
            },
        }],
    });

    server.register_prompt(mcp_server::McpPrompt {
        name: "decompose_task".to_string(),
        description: Some("Break down a complex task into steps".to_string()),
        arguments: Some(vec![mcp_server::PromptArgument {
            name: "task".to_string(),
            description: "The task to decompose".to_string(),
            required: true,
        }]),
        messages: vec![mcp_server::PromptMessage {
            role: "user".to_string(),
            content: mcp_server::PromptContent::Text {
                text: "Break this task into clear steps:\n\n{{task}}".to_string(),
            },
        }],
    });

    // Run the MCP server (blocking)
    if let Err(e) = server.run() {
        eprintln!("MCP server error: {}", e);
        std::process::exit(1);
    }
}
