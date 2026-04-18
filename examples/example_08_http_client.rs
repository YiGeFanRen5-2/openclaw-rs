//! Example 08: HTTP Client Usage
//!
//! This example demonstrates how to use the `http_request` tool:
//! - GET, POST, PUT, DELETE requests
//! - JSON request/response handling
//! - Custom headers
//! - Timeout configuration
//! - Error handling
//! - Response parsing
//!
//! Run with: cargo run --example example_08_http_client

use runtime::{create_runtime, RuntimeConfig, Runtime};
use runtime::SessionId;
use tools::ToolCall;

// ─── Execute tool helper ───────────────────────────────────────────────────────

fn execute_tool(
    runtime: &Runtime,
    session_id: &SessionId,
    tool_name: &str,
    args: serde_json::Value,
) -> Result<String, String> {
    let call = ToolCall {
        name: tool_name.to_string(),
        arguments: serde_json::to_string(&args).map_err(|e| e.to_string())?,
    };
    runtime
        .execute_tool(session_id, call)
        .map(|r| r.to_string())
        .map_err(|e| e.to_string())
}

// ─── JSON pretty printer ──────────────────────────────────────────────────────

fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw)
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()))
        .unwrap_or_else(|_| raw.to_string())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== OpenClaw HTTP Client Example ===\n");

    // 1. Setup runtime with http_request tool
    println!("1. Setup Runtime with HTTP tool...");
    let config = RuntimeConfig::default();
    let runtime = create_runtime(config)?;
    println!("   ✓ Runtime ready\n");

    // 2. Create a session for tool call history
    println!("2. Create session...");
    let session_id = runtime.create_session("http-demo-session")?;
    println!("   ✓ Session: {}\n", session_id.as_str());

    // ── GET Request ─────────────────────────────────────────────────────────
    println!("=== HTTP GET Request ===");
    println!("3. GET request to httpbin.org/get...");

    let get_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "GET",
            "url": "https://httpbin.org/get",
            "headers": {
                "Accept": "application/json",
                "User-Agent": "OpenClaw/1.0"
            },
            "timeout_seconds": 10
        }),
    );

    match get_result {
        Ok(body) => {
            println!("   ✓ Status: OK");
            println!("   Response:\n{}", pretty_json(&body));
        }
        Err(e) => println!("   ✗ GET failed: {}", e),
    }
    println!();

    // ── POST with JSON body ──────────────────────────────────────────────────
    println!("=== HTTP POST with JSON Body ===");
    println!("4. POST request to httpbin.org/post...");

    let post_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "POST",
            "url": "https://httpbin.org/post",
            "headers": {
                "Content-Type": "application/json",
                "Accept": "application/json"
            },
            "body": {
                "name": "OpenClaw",
                "version": "1.0",
                "features": ["tools", "sessions", "plugins"]
            },
            "timeout_seconds": 10
        }),
    );

    match post_result {
        Ok(body) => {
            println!("   ✓ Status: OK");
            println!("   Response:\n{}", pretty_json(&body));
        }
        Err(e) => println!("   ✗ POST failed: {}", e),
    }
    println!();

    // ── POST with form data ───────────────────────────────────────────────────
    println!("=== HTTP POST with Form Data ===");
    println!("5. POST form data to httpbin.org/post...");

    let form_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "POST",
            "url": "https://httpbin.org/post",
            "headers": {
                "Content-Type": "application/x-www-form-urlencoded"
            },
            "body": "username=openclaw&action=login",
            "timeout_seconds": 10
        }),
    );

    match form_result {
        Ok(body) => {
            println!("   ✓ Status: OK");
            println!("   Response:\n{}", pretty_json(&body));
        }
        Err(e) => println!("   ✗ POST form failed: {}", e),
    }
    println!();

    // ── PUT request ───────────────────────────────────────────────────────────
    println!("=== HTTP PUT Request ===");
    println!("6. PUT request to httpbin.org/put...");

    let put_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "PUT",
            "url": "https://httpbin.org/put",
            "headers": {
                "Content-Type": "application/json",
                "Authorization": "Bearer demo-token-abc123"
            },
            "body": {
                "update": "this is an update",
                "id": 42
            },
            "timeout_seconds": 10
        }),
    );

    match put_result {
        Ok(body) => {
            println!("   ✓ Status: OK");
            println!("   Response:\n{}", pretty_json(&body));
        }
        Err(e) => println!("   ✗ PUT failed: {}", e),
    }
    println!();

    // ── DELETE request ────────────────────────────────────────────────────────
    println!("=== HTTP DELETE Request ===");
    println!("7. DELETE request to httpbin.org/delete...");

    let delete_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "DELETE",
            "url": "https://httpbin.org/delete",
            "headers": {
                "Authorization": "Bearer demo-token-abc123"
            },
            "timeout_seconds": 10
        }),
    );

    match delete_result {
        Ok(body) => {
            println!("   ✓ Status: OK");
            println!("   Response:\n{}", pretty_json(&body));
        }
        Err(e) => println!("   ✗ DELETE failed: {}", e),
    }
    println!();

    // ── Error handling: Invalid URL ──────────────────────────────────────────
    println!("=== Error Handling ===");
    println!("8. Test invalid URL...");
    let invalid_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "GET",
            "url": "not-a-valid-url",
            "timeout_seconds": 5
        }),
    );
    match invalid_result {
        Ok(body) => println!("   Response: {}", body),
        Err(e) => println!("   ✓ Correctly rejected invalid URL: {}", e),
    }
    println!();

    // ── Error handling: Unsupported method ─────────────────────────────────
    println!("9. Test unsupported HTTP method...");
    let bad_method_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "INVALID_METHOD",
            "url": "https://httpbin.org/get",
            "timeout_seconds": 5
        }),
    );
    match bad_method_result {
        Ok(body) => println!("   Response: {}", body),
        Err(e) => println!("   ✓ Correctly rejected bad method: {}", e),
    }
    println!();

    // ── Timeout demonstration ────────────────────────────────────────────────
    println!("10. Test with short timeout (expect timeout or failure)...");
    let timeout_result = execute_tool(
        &runtime,
        &session_id,
        "http_request",
        serde_json::json!({
            "method": "GET",
            "url": "https://httpbin.org/delay/5",
            "timeout_seconds": 1
        }),
    );
    match timeout_result {
        Ok(body) => println!("   Response: {}", body),
        Err(e) => println!("   ✓ Correctly handled timeout: {}", e),
    }
    println!();

    // ── Retry demonstration ─────────────────────────────────────────────────
    // Simulate a retry: try up to 3 times with 1s delay between attempts
    println!("11. Retry demonstration (3 attempts, 1s delay)...");
    let mut retry_count = 0;
    let max_retries = 3;
    let mut last_error = String::new();

    while retry_count < max_retries {
        retry_count += 1;
        println!("   Attempt {}/{}...", retry_count, max_retries);

        match execute_tool(
            &runtime,
            &session_id,
            "http_request",
            serde_json::json!({
                "method": "GET",
                "url": "https://httpbin.org/get",
                "timeout_seconds": 10
            }),
        ) {
            Ok(body) => {
                println!("   ✓ Retry succeeded on attempt {}", retry_count);
                println!("   Response:\n{}", pretty_json(&body));
                break;
            }
            Err(e) => {
                last_error = e;
                if retry_count < max_retries {
                    println!("   ⚠ Attempt {}/{} failed: {}, retrying...", retry_count, max_retries, &last_error);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                } else {
                    println!("   ✗ All {} retry attempts failed: {}", max_retries, last_error);
                }
            }
        }
    }
    println!();

    // ── Check session tool call history ─────────────────────────────────────
    println!("12. Session tool call history...");
    let session = runtime.get_session(&session_id)?;
    println!("   Total messages in session: {}", session.messages().len());
    for (i, msg) in session.messages().iter().enumerate() {
        let preview = if msg.content.len() > 60 {
            format!("{}...", &msg.content[..60])
        } else {
            msg.content.clone()
        };
        println!("   [{}] {:?}: {}", i + 1, msg.role, preview);
    }
    println!();

    println!("=== HTTP Client Example Complete ===");
    println!("Demonstrated: GET/POST/PUT/DELETE, JSON & form bodies,");
    println!("headers, timeouts, error handling, and retry logic.");
    Ok(())
}
