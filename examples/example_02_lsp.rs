//! Example 2: LSP Client Usage
//!
//! Demonstrates connecting to rust-analyzer and performing code navigation.
//!
//! Run: cargo run --example lsp_client -- rust-analyzer

use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <server-cmd> [args...]", args[0]);
        eprintln!("Example: {} rust-analyzer", args[0]);
        return Ok(());
    }

    let server_name = &args[1];
    let server_args: Vec<String> = args[2..].to_vec();

    println!("=== LSP Client Example ===");
    println!("Server: {} {:?}", server_name, server_args);
    println!("\nNote: This is a conceptual example.");
    println!("The full LSP client requires an async runtime (tokio).");
    println!("\nTo use in Node.js:");
    println!("  const rt = new OpenClawRuntime(...);");
    println!("  rt.lspInit('{}', {:?});", server_name, server_args);
    println!("  rt.lspConnect('file:///project');");
    println!("  const completions = rt.lspCompletions(uri, line, char);");

    Ok(())
}
