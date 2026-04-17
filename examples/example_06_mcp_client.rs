//! Example 06: MCP Client - Using MCP Protocol
//!
//! This example demonstrates how to connect to an MCP server
//! using the JSON-RPC protocol over stdio.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

fn main() -> anyhow::Result<()> {
    println!("=== MCP Client Example ===\n");

    // Start MCP server as child process
    println!("1. Starting MCP Server...");
    let mut child = Command::new("./target/release/mcp-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    let stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));
    let mut lines = stdout.lines();

    println!("   ✓ Server started\n");

    // 2. Send initialize request
    println!("2. Send Initialize...");
    let init_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: 1,
        method: "initialize".into(),
        params: json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "example-client",
                "version": "1.0.0"
            }
        }),
    };
    
    stdin.write_all(serde_json::to_string(&init_request).unwrap().as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;

    // Read initialize response
    if let Some(line) = lines.next() {
        let response: JsonRpcResponse = serde_json::from_str(&line?)?;
        println!("   ✓ Server: {:?}\n", response.result);
    }

    // 3. Send initialized notification
    println!("3. Send Initialized Notification...");
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    stdin.write_all(serde_json::to_string(&notif).unwrap().as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    println!("   ✓ Notification sent\n");

    // 4. List tools
    println!("4. List Tools...");
    let list_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: 2,
        method: "tools/list".into(),
        params: json!({}),
    };
    stdin.write_all(serde_json::to_string(&list_request).unwrap().as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;

    if let Some(line) = lines.next() {
        let response: JsonRpcResponse = serde_json::from_str(&line?)?;
        let tools = &response.result.get("tools");
        println!("   ✓ Tools response: {:?}\n", tools.map(|t| t.as_array().map(|a| a.len()).unwrap_or(0)));
    }

    // 5. Shutdown
    println!("5. Shutdown...");
    let shutdown = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: 3,
        method: "shutdown".into(),
        params: json!({}),
    };
    stdin.write_all(serde_json::to_string(&shutdown).unwrap().as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;

    child.wait()?;
    println!("   ✓ Server stopped\n");

    println!("=== Example Complete ===");
    Ok(())
}
