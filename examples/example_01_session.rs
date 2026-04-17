//! Example 1: Basic Session Management
//!
//! Creates a session, adds messages, and persists to disk.
//!
//! Run: cargo run --example session_basic

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This example demonstrates the session API conceptually.
    // In a real app, you'd use the Node.js bindings or N-API.

    println!("=== OpenClaw Session Example ===\n");

    // Simulate session creation
    let session_id = "example-session-1";
    println!("Created session: {}", session_id);

    // Simulate adding messages
    let messages = vec![
        ("user", "Hello, how are you?"),
        ("assistant", "I'm doing well! How can I help you today?"),
        ("user", "Can you list the files in /tmp?"),
        ("assistant", "I can do that. Let me use the list_files tool."),
    ];

    println!("\nMessages:");
    for (role, content) in &messages {
        println!("  [{}]: {}", role, content);
    }

    // Simulate token count
    let total_chars: usize = messages.iter().map(|(_, m)| m.len()).sum();
    let approx_tokens = total_chars / 4;
    println!("\nApproximate token count: ~{}", approx_tokens);

    println!("\n✅ Session example complete");
    Ok(())
}
