//! Integration tests for OpenClaw-RS
//!
//! This crate contains integration tests and benchmarks for the OpenClaw-RS project.
//!
//! ## Structure
//!
//! - `src/common/` - Shared test utilities (TestRuntime, TestTool, TestLogger)
//! - `tests/` - Integration test modules (binary integration tests)
//!   - `test_session.rs` - Session management tests
//!   - `test_tools.rs` - Tool execution tests
//!   - `test_mcp.rs` - MCP protocol tests
//!   - `test_plugins.rs` - Plugin system tests
//! - `benches/` - Performance benchmarks
//!   - `session_bench.rs` - Session-related benchmarks
//!   - `tool_bench.rs` - Tool-related benchmarks

pub mod common;
