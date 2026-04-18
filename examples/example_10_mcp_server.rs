//! Example 10: MCP Server
//!
//! This example demonstrates how to run an MCP server that provides:
//! - Custom tools callable by Claude Desktop / Cursor
//! - Resources (files, sessions, user data)
//! - Prompts (predefined templates with variable substitution)
//! - Integration with OpenClaw runtime for actual tool execution
//!
//! The MCP server speaks JSON-RPC 2.0 over stdio.
//!
//! Demo mode (simulates a client conversation):
//!   cargo run --example example_10_mcp_server -- demo
//!
//! Stdio server mode (for Claude Desktop integration):
//!   cargo run --example example_10_mcp_server

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use runtime::{create_runtime, RuntimeConfig, Runtime};
use runtime::SessionId;
use tools::ToolCall;

// ─── MCP Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<RequestId>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpTool {
    name: String,
    description: String,
    input_schema: JsonValue,
}

// ─── Tool executor ───────────────────────────────────────────────────────────

fn execute_openclaw_tool(
    runtime: &Mutex<Runtime>,
    session_id: &SessionId,
    tool_name: &str,
    args: &JsonValue,
) -> String {
    let mut rt = match runtime.lock() {
        Ok(rt) => rt,
        Err(e) => return format!("{{\"error\": \"lock poisoned: {}\"}}", e),
    };
    let call = ToolCall {
        name: tool_name.to_string(),
        arguments: serde_json::to_string(args).unwrap_or_else(|_| "{}".into()),
    };
    match rt.execute_tool(session_id, call) {
        Ok(result) => {
            json!({
                "content": [{ "type": "text", "text": result.content }]
            })
            .to_string()
        }
        Err(e) => {
            json!({
                "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                "isError": true
            })
            .to_string()
        }
    }
}

// ─── MCP Server Implementation ───────────────────────────────────────────────

struct ExampleMcpServer {
    runtime: Arc<Mutex<Runtime>>,
    session_id: SessionId,
    tools: Vec<McpTool>,
}

impl ExampleMcpServer {
    fn new() -> anyhow::Result<Self> {
        let config = RuntimeConfig::default();
        let runtime = create_runtime(config)?;
        let session_id = runtime.create_session("mcp-server-session")?;
        let runtime = Arc::new(Mutex::new(runtime));

        let tools = vec![
            McpTool {
                name: "read_file".to_string(),
                description: "Read the contents of a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path to read" },
                        "encoding": {
                            "type": "string",
                            "enum": ["utf8", "base64"],
                            "default": "utf8"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "text_stats".to_string(),
                description: "Get statistics about text (chars, words, lines)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Input text" }
                    },
                    "required": ["text"]
                }),
            },
            McpTool {
                name: "uuid".to_string(),
                description: "Generate a random UUID v4".to_string(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            McpTool {
                name: "hash".to_string(),
                description: "Hash data with sha256".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "data": { "type": "string" },
                        "algorithm": { "type": "string", "enum": ["sha256"], "default": "sha256" }
                    },
                    "required": ["data"]
                }),
            },
            McpTool {
                name: "random_string".to_string(),
                description: "Generate a random string".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "length": { "type": "integer", "default": 16 }
                    }
                }),
            },
        ];

        Ok(Self { runtime, session_id, tools })
    }

    fn run(&mut self) -> anyhow::Result<()> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[MCP] Read error: {}", e);
                    continue;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let err_resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: RequestId::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };
                    writeln!(out, "{}", serde_json::to_string(&err_resp).unwrap())?;
                    out.flush()?;
                    continue;
                }
            };

            let id = request.id.clone().unwrap_or(RequestId::Null);

            let result = match request.method.as_str() {
                "initialize" => {
                    Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                        "serverInfo": { "name": "openclaw-mcp-server", "version": "0.1.0" }
                    }))
                }
                "notifications/initialized" => {
                    // Notification — no response needed
                    continue;
                }
                "tools/list" => Some(json!({ "tools": self.tools.clone() })),
                "tools/call" => {
                    let params = request.params.as_ref();
                    let tool_name = params.and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                    let args = params.and_then(|p| p.get("arguments")).unwrap_or(&serde_json::Value::Null);
                    let output = execute_openclaw_tool(&self.runtime, &self.session_id, tool_name, args);
                    Some(serde_json::from_str(&output).unwrap_or_else(|_| {
                        json!({ "content": [{ "type": "text", "text": output }] })
                    }))
                }
                "resources/list" => Some(json!({
                    "resources": [
                        { "uri": "session://current", "name": "Current Session", "mimeType": "application/json" },
                        { "uri": "file:///tmp/openclaw-mcp-demo.txt", "name": "Demo File", "mimeType": "text/plain" }
                    ]
                })),
                "resources/read" => {
                    let uri = request.params.as_ref()
                        .and_then(|p| p.get("uri"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let contents = match uri {
                        "session://current" => {
                            let session = self.runtime.lock().ok().and_then(|rt| rt.get_session(&self.session_id).ok());
                            vec![json!({
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string_pretty(&session.map(|s| {
                                    json!({ "id": s.id().as_str(), "messages": s.messages().len() })
                                })).unwrap_or_default()
                            })]
                        }
                        _ => vec![json!({ "uri": uri, "mimeType": "text/plain", "text": "[resource not found]" })]
                    };
                    Some(json!({ "contents": contents }))
                }
                "prompts/list" => Some(json!({
                    "prompts": [
                        { "name": "code_review", "description": "Review code for issues", "arguments": [
                            { "name": "language", "required": true },
                            { "name": "focus", "required": false }
                        ]},
                        { "name": "写作助手", "description": "中文写作助手", "arguments": [
                            { "name": "主题", "required": true },
                            { "name": "风格", "required": false }
                        ]}
                    ]
                })),
                "prompts/get" => {
                    let params = request.params.as_ref();
                    let name = params.and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                    let args = params.and_then(|p| p.get("arguments")).unwrap_or(&serde_json::Value::Null);

                    let (description, messages) = match name {
                        "code_review" => {
                            let lang = args.get("language").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let focus = args.get("focus").and_then(|v| v.as_str()).unwrap_or("general");
                            (
                                Some(format!("Review {} code focusing on {}", lang, focus)),
                                vec![
                                    json!({ "role": "system", "content": { "type": "text", "text": format!("You are a code reviewer for {}. Focus on: {}.", lang, focus) }}),
                                    json!({ "role": "user", "content": { "type": "text", "text": "Please review the following code:\n\n{{code}}" }})
                                ],
                            )
                        }
                        "写作助手" => {
                            let theme = args.get("主题").and_then(|v| v.as_str()).unwrap_or("一般");
                            let style = args.get("风格").and_then(|v| v.as_str()).unwrap_or("正式");
                            (
                                Some(format!("风格: {}，主题: {}", style, theme)),
                                vec![
                                    json!({ "role": "system", "content": { "type": "text", "text": format!("你是擅长{}风格的中文写作助手。", style) }}),
                                    json!({ "role": "user", "content": { "type": "text", "text": format!("请写一篇关于「{{主题}}」的文章（当前主题：{}）。", theme) }})
                                ],
                            )
                        }
                        _ => (None, vec![])
                    };

                    let mut result_map = serde_json::Map::new();
                    if let Some(d) = description {
                        result_map.insert("description".into(), serde_json::Value::String(d));
                    }
                    result_map.insert("messages".into(), serde_json::Value::Array(messages));
                    Some(serde_json::Value::Object(result_map))
                }
                "shutdown" => Some(serde_json::Value::Null),
                _ => {
                    return Err(anyhow::anyhow!("Method not found: {}", request.method));
                }
            };

            if !matches!(request.id, None) {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result,
                    error: None,
                };
                writeln!(out, "{}", serde_json::to_string(&resp).unwrap_or_default())?;
                out.flush()?;
            }
        }

        Ok(())
    }
}

// ─── CLI interface ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Mode {
    Server,
    /// Interactive demo: simulates a client conversation
    Demo,
}

fn main() -> anyhow::Result<()> {
    let mode = if std::env::args().nth(1).as_deref() == Some("demo") {
        Mode::Demo
    } else {
        Mode::Server
    };

    match mode {
        Mode::Server => {
            println!("[OpenClaw MCP Server] Starting in stdio mode...");
            let mut server = ExampleMcpServer::new()?;
            server.run()?;
        }
        Mode::Demo => {
            run_demo()?;
        }
    }

    Ok(())
}

// ─── Interactive demo (simulates Claude Desktop client) ─────────────────────

fn run_demo() -> anyhow::Result<()> {
    println!("=== OpenClaw MCP Server Demo ===\n");
    println!("This demo simulates a Claude Desktop client interacting with the MCP server.\n");

    let mut server = ExampleMcpServer::new()?;

    // Simulate JSON-RPC client interaction
    println!("[Client] Sending: initialize");
    let init_resp = simulate_request(&server, "initialize", json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": { "name": "demo-client", "version": "1.0.0" }
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&init_resp).unwrap());

    println!("[Client] Sending: notifications/initialized (notification, no response)\n");

    println!("[Client] Sending: tools/list");
    let tools_resp = simulate_request(&server, "tools/list", json!({}))?;
    let tools_count = tools_resp.get("tools").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
    println!("[Server] {} tools available:", tools_count);
    if let Some(tools) = tools_resp.get("tools").and_then(|t| t.as_array()) {
        for tool in tools {
            let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let desc = tool.get("description").and_then(|v| v.as_str()).unwrap_or("");
            println!("  - {}: {}", name, desc);
        }
    }
    println!();

    println!("[Client] Sending: tools/call (text_stats)");
    let stats_resp = simulate_request(&server, "tools/call", json!({
        "name": "text_stats",
        "arguments": { "text": "Hello MCP world, this is a test!" }
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&stats_resp).unwrap());

    println!("[Client] Sending: tools/call (uuid)");
    let uuid_resp = simulate_request(&server, "tools/call", json!({
        "name": "uuid",
        "arguments": {}
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&uuid_resp).unwrap());

    println!("[Client] Sending: tools/call (hash)");
    let hash_resp = simulate_request(&server, "tools/call", json!({
        "name": "hash",
        "arguments": { "data": "hello world", "algorithm": "sha256" }
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&hash_resp).unwrap());

    println!("[Client] Sending: resources/list");
    let resources_resp = simulate_request(&server, "resources/list", json!({}))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&resources_resp).unwrap());

    println!("[Client] Sending: resources/read (session://current)");
    let session_resp = simulate_request(&server, "resources/read", json!({
        "uri": "session://current"
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&session_resp).unwrap());

    println!("[Client] Sending: prompts/list");
    let prompts_resp = simulate_request(&server, "prompts/list", json!({}))?;
    let prompts_count = prompts_resp.get("prompts").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
    println!("[Server] {} prompts available:", prompts_count);
    if let Some(prompts) = prompts_resp.get("prompts").and_then(|t| t.as_array()) {
        for prompt in prompts {
            let name = prompt.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let desc = prompt.get("description").and_then(|v| v.as_str()).unwrap_or("");
            println!("  - {}: {}", name, desc);
        }
    }
    println!();

    println!("[Client] Sending: prompts/get (code_review)");
    let review_resp = simulate_request(&server, "prompts/get", json!({
        "name": "code_review",
        "arguments": { "language": "Rust", "focus": "error handling" }
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&review_resp).unwrap());

    println!("[Client] Sending: prompts/get (写作助手)");
    let write_resp = simulate_request(&server, "prompts/get", json!({
        "name": "写作助手",
        "arguments": { "主题": "人工智能的未来", "风格": "学术" }
    }))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&write_resp).unwrap());

    println!("[Client] Sending: shutdown");
    let shutdown_resp = simulate_request(&server, "shutdown", json!({}))?;
    println!("[Server] Response: {}\n", serde_json::to_string_pretty(&shutdown_resp).unwrap());

    println!("=== MCP Server Demo Complete ===");
    println!("Demonstrated: tool registration, tools/list & tools/call,");
    println!("resources, prompts with argument substitution, JSON-RPC protocol.");
    Ok(())
}

/// Simulate a JSON-RPC request/response roundtrip with the server.
fn simulate_request(server: &ExampleMcpServer, method: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    let result = dispatch(server, method, params)?;
    Ok(result.unwrap_or(serde_json::Value::Null))
}

/// Direct dispatch without the stdio layer.
fn dispatch(server: &ExampleMcpServer, method: &str, params: JsonValue) -> anyhow::Result<Option<JsonValue>> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
            "serverInfo": { "name": "openclaw-mcp-server", "version": "0.1.0" }
        }))),
        "tools/list" => Ok(Some(json!({ "tools": server.tools.clone() }))),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").unwrap_or(&serde_json::Value::Null);
            let output = execute_openclaw_tool(&server.runtime, &server.session_id, tool_name, args);
            Ok(Some(serde_json::from_str(&output).unwrap_or_else(|_| {
                json!({ "content": [{ "type": "text", "text": output }] })
            })))
        }
        "resources/list" => Ok(Some(json!({
            "resources": [
                { "uri": "session://current", "name": "Current Session", "mimeType": "application/json" },
                { "uri": "file:///tmp/openclaw-mcp-demo.txt", "name": "Demo File", "mimeType": "text/plain" }
            ]
        }))),
        "resources/read" => {
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let contents = match uri {
                "session://current" => {
                    let session = server.runtime.lock().ok().and_then(|rt| rt.get_session(&server.session_id).ok());
                    vec![json!({
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&session.map(|s| {
                            json!({ "id": s.id().as_str(), "messages": s.messages().len() })
                        })).unwrap_or_default()
                    })]
                }
                _ => vec![json!({ "uri": uri, "mimeType": "text/plain", "text": "[resource not found]" })]
            };
            Ok(Some(json!({ "contents": contents })))
        }
        "prompts/list" => Ok(Some(json!({
            "prompts": [
                { "name": "code_review", "description": "Review code for issues", "arguments": [
                    { "name": "language", "required": true },
                    { "name": "focus", "required": false }
                ]},
                { "name": "写作助手", "description": "中文写作助手", "arguments": [
                    { "name": "主题", "required": true },
                    { "name": "风格", "required": false }
                ]}
            ]
        }))),
        "prompts/get" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").unwrap_or(&serde_json::Value::Null);
            let (description, messages) = match name {
                "code_review" => {
                    let lang = args.get("language").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let focus = args.get("focus").and_then(|v| v.as_str()).unwrap_or("general");
                    (
                        Some(format!("Review {} code focusing on {}", lang, focus)),
                        vec![
                            json!({ "role": "system", "content": { "type": "text", "text": format!("You are a code reviewer for {}. Focus on: {}.", lang, focus) }}),
                            json!({ "role": "user", "content": { "type": "text", "text": "Please review the following code:\n\n{{code}}" }})
                        ],
                    )
                }
                "写作助手" => {
                    let theme = args.get("主题").and_then(|v| v.as_str()).unwrap_or("一般");
                    let style = args.get("风格").and_then(|v| v.as_str()).unwrap_or("正式");
                    (
                        Some(format!("风格: {}，主题: {}", style, theme)),
                        vec![
                            json!({ "role": "system", "content": { "type": "text", "text": format!("你是擅长{}风格的中文写作助手。", style) }}),
                            json!({ "role": "user", "content": { "type": "text", "text": format!("请写一篇关于「{{主题}}」的文章（当前主题：{}）。", theme) }})
                        ],
                    )
                }
                _ => (None, vec![])
            };
            let mut obj = serde_json::Map::new();
            if let Some(d) = description {
                obj.insert("description".into(), serde_json::Value::String(d));
            }
            obj.insert("messages".into(), serde_json::Value::Array(messages));
            Ok(Some(serde_json::Value::Object(obj)))
        }
        "shutdown" => Ok(Some(serde_json::Value::Null)),
        _ => Err(anyhow::anyhow!("Unknown method: {}", method))
    }
}
