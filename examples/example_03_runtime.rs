//! Example 3: Runtime with Provider and Tools
//!
//! Demonstrates creating a runtime, registering tools, and executing them.
//!
//! Run: cargo run --example runtime_full --features runtime/runtime

// Note: This example shows the conceptual API.
// For actual execution, use the Node.js bindings or the tokio-based runtime.

fn main() {
    println!("=== OpenClaw Runtime Example ===\n");

    // This example demonstrates the runtime API structure.
    // In a real application, you would use:

    println!("1. Create runtime with config:");
    println!("   let config = RuntimeConfig {{");
    println!("       max_session_tokens: 4000,");
    println!("       compaction_threshold: 0.8,");
    println!("       min_recent_messages: 4,");
    println!("       persist_path: Some(std::path::PathBuf::from(\"/tmp/sessions\")),");
    println!("   }};");
    println!("   let mut runtime = Runtime::new(config).unwrap();\n");

    println!("2. Create a session:");
    println!("   runtime.create_session(\"my-session\").unwrap();\n");

    println!("3. Add messages:");
    println!("   runtime.add_message(\"my-session\", Role::User, \"Hello!\").unwrap();\n");

    println!("4. Register a tool:");
    println!("   runtime.register_tool(Box::new(MyTool {{}})).unwrap();\n");

    println!("5. Execute a tool:");
    println!("   let result = runtime.execute_tool(");
    println!("       &SessionId::from(\"my-session\"),");
    println!("       ToolCall {{ name: \"read_file\".into(), arguments: json!({{\"path\": \"/tmp/test.txt\"}}) }}");
    println!("   ).unwrap();\n");

    println!("6. Compact session when full:");
    println!("   if session.should_compact(4000) {{");
    println!("       runtime.compact_session(&SessionId::from(\"my-session\")).unwrap();");
    println!("   }}\n");

    println!("7. Persist to disk:");
    println!("   runtime.persist_session(&SessionId::from(\"my-session\")).unwrap();\n");

    println!("8. LSP integration:");
    println!("   let bridge = LspBridge::rust_analyzer();");
    println!("   bridge.connect(\"file:///project\").await.unwrap();");
    println!("   let completions = bridge.completions(uri, 0, 5).await.unwrap();\n");

    println!("✅ See FFI.md and PROJECT-SUMMARY.md for full API reference");
}
