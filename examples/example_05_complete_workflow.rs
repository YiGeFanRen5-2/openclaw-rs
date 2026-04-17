//! Example 05: Complete OpenClaw Workflow
//!
//! This example demonstrates a complete workflow including:
//! - Session creation and management
//! - Tool registration and execution
//! - Message handling with the LLM
//! - Metrics collection
//! - Persistence

use openclaw_core::runtime::Runtime;
use openclaw_core::providers::mock::MockProvider;
use openclaw_core::tools::{ToolRegistry, register_builtin_tools};
use openclaw_plugins::MetricsCollector;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== OpenClaw Complete Workflow Example ===\n");

    // 1. Initialize Metrics Collector
    println!("1. Initialize Metrics...");
    let metrics = MetricsCollector::new();
    metrics.increment_counter("workflow_started", None);
    println!("   ✓ Metrics initialized\n");

    // 2. Create Tool Registry with built-in tools
    println!("2. Setup Tool Registry...");
    let mut registry = ToolRegistry::new();
    register_builtin_tools(&mut registry);
    let tools = registry.list_schemas();
    println!("   ✓ Registered {} tools\n", tools.len());

    // 3. Create Runtime with Mock Provider
    println!("3. Initialize Runtime...");
    let provider = MockProvider::new("mock-v1".into());
    let runtime = Runtime::builder()
        .provider(provider)
        .tools(registry)
        .metrics(metrics.clone())
        .build()?;
    println!("   ✓ Runtime ready\n");

    // 4. Create a session
    println!("4. Create Session...");
    let session_id = "example-session-001";
    runtime.create_session(session_id.into())?;
    metrics.record_session_create();
    println!("   ✓ Session created: {}\n", session_id);

    // 5. Add a user message
    println!("5. Add User Message...");
    runtime.add_message(session_id.into(), "user", "Hello, what can you do?")?;
    metrics.record_message("in");
    println!("   ✓ Message added\n");

    // 6. Simulate LLM response
    println!("6. Process with LLM (Mock)...");
    let response = runtime.chat(
        session_id.into(),
        "What is 2 + 2?",
        Some("gpt-4".into()),
    ).await?;
    println!("   ✓ LLM Response: {}\n", response.content);

    // 7. Execute a tool
    println!("7. Execute Tool (text_stats)...");
    let tool_result = runtime.execute_tool(
        session_id.into(),
        "text_stats",
        r#"{"text": "Hello world this is a test"}"#,
    ).await?;
    println!("   ✓ Tool Result: {}\n", tool_result);

    // 8. Check session status
    println!("8. Session Status...");
    let session = runtime.get_session(session_id.into());
    println!("   ✓ Messages: {}", session.messages().len());
    println!("   ✓ Tokens (approx): {}\n", runtime.session_token_count(session_id.into()));

    // 9. Record metrics
    println!("9. Record Metrics...");
    metrics.record_tool_call("text_stats", true, Duration::from_millis(5));
    metrics.increment_counter("workflow_completed", None);
    println!("   ✓ Metrics recorded\n");

    // 10. Export metrics
    println!("10. Export Metrics...");
    let prom_metrics = metrics.export_prometheus();
    println!("   ✓ Prometheus format:\n{}", prom_metrics);

    // 11. Persist session
    println!("11. Persist Session...");
    runtime.persist_session(session_id.into())?;
    println!("   ✓ Session persisted\n");

    // 12. Shutdown
    println!("12. Shutdown...");
    runtime.shutdown().await?;
    println!("   ✓ Shutdown complete\n");

    println!("=== Workflow Complete ===");
    Ok(())
}
