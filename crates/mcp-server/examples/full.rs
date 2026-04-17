//! OpenClaw MCP Server - Full Implementation Example
//! Demonstrates tools, resources, and prompts using openclaw-core.

use mcp_server::{
    McpPrompt, McpResource, McpServer, McpTool, PromptArgument, PromptContent, PromptMessage,
};
use openclaw_core::OpenClawCore;
use serde_json::json;
use std::sync::Arc;

fn main() {
    let core = Arc::new(OpenClawCore::new());

    // Create MCP server
    let mut server = McpServer::new("OpenClaw", "0.1.0").with_tool_executor(Box::new({
        let core = Arc::clone(&core);
        move |tool_name, args| match core.execute_tool(tool_name, args) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }));

    // Register tools from openclaw-core (convert CoreTool -> McpTool)
    for tool in core.list_tools() {
        let mcp_tool = McpTool {
            name: tool.name,
            description: tool.description,
            input_schema: json!(tool.input_schema),
        };
        server.register_tool(mcp_tool);
    }

    // Register a sample file system resource (README)
    let readme_resource = McpResource {
        uri: "file:///README.md".to_string(),
        name: "OpenClaw README".to_string(),
        description: Some("OpenClaw project documentation".to_string()),
        mime_type: Some("text/markdown".to_string()),
    };
    server.register_resource(readme_resource);

    // Register a sample session resource template
    let session_resource = McpResource {
        uri: "session://{session_id}".to_string(),
        name: "OpenClaw Session".to_string(),
        description: Some("Retrieve a specific session storage".to_string()),
        mime_type: Some("application/json".to_string()),
    };
    server.register_resource(session_resource);

    // Register prompts
    let system_prompt = McpPrompt {
        name: "openclaw_assistant".to_string(),
        description: Some("System prompt for OpenClaw assistant mode".to_string()),
        arguments: Some(vec![
            PromptArgument {
                name: "role".to_string(),
                description: "Assistant role/persona".to_string(),
                required: true,
            },
            PromptArgument {
                name: "guidelines".to_string(),
                description: "Additional behavior guidelines".to_string(),
                required: false,
            },
        ]),
        messages: vec![PromptMessage {
            role: "system".to_string(),
            content: PromptContent::Text {
                text: "You are {{role}}. You are powered by OpenClaw. {{guidelines}}".to_string(),
            },
        }],
    };
    server.register_prompt(system_prompt);

    let task_decomposition_prompt = McpPrompt {
        name: "decompose_task".to_string(),
        description: Some("Break down a complex task into steps".to_string()),
        arguments: Some(vec![PromptArgument {
            name: "task".to_string(),
            description: "The task to decompose".to_string(),
            required: true,
        }]),
        messages: vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "Break this task into clear steps:\n\n{{task}}".to_string(),
            },
        }],
    };
    server.register_prompt(task_decomposition_prompt);

    // Run server
    if let Err(e) = server.run() {
        eprintln!("MCP server error: {}", e);
        std::process::exit(1);
    }
}
