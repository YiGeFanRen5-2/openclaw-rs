//! Common test utilities and fixtures for OpenClaw-RS integration tests.

use std::sync::Arc;
use tokio::sync::RwLock;

/// TestRuntime - Lightweight test runtime manager for integration tests.
///
/// Provides a minimal runtime environment for testing OpenClaw components
/// without requiring the full runtime initialization.
pub struct TestRuntime {
    /// Tokio runtime handle for async operations
    runtime: Option<tokio::runtime::Runtime>,
    /// Shared state for test coordination
    state: Arc<RwLock<TestState>>,
}

/// Mutable test state shared across test components
pub struct TestState {
    /// Number of active sessions
    pub session_count: usize,
    /// Number of executed tools
    pub tool_executions: usize,
    /// Test name for logging
    pub current_test: Option<String>,
}

impl TestRuntime {
    /// Create a new TestRuntime with a multi-threaded Tokio runtime
    pub fn new() -> Result<Self, std::io::Error> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let state = Arc::new(RwLock::new(TestState {
            session_count: 0,
            tool_executions: 0,
            current_test: None,
        }));

        Ok(Self { runtime: Some(runtime), state })
    }

    /// Get a handle to the Tokio runtime
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.as_ref().expect("runtime not dropped").handle().clone()
    }

    /// Get shared state
    pub fn state(&self) -> Arc<RwLock<TestState>> {
        self.state.clone()
    }

    /// Execute an async block within the test runtime
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.as_ref().expect("runtime not dropped").block_on(future)
    }

    /// Spawn a task on the test runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.as_ref().expect("runtime not dropped").spawn(future)
    }
}

impl Default for TestRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create TestRuntime")
    }
}

impl Drop for TestRuntime {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(std::time::Duration::from_millis(100));
        }
    }
}

/// TestTool - Mock tool for testing tool execution pipeline.
///
/// Represents a minimal tool that can be used to verify tool registration,
/// invocation, and result handling in integration tests.
#[derive(Debug, Clone)]
pub struct TestTool {
    /// Unique identifier for the tool
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Execution count (for verifying invocation)
    pub invocation_count: Arc<RwLock<usize>>,
    /// Whether the tool should succeed or fail
    pub should_fail: bool,
}

impl TestTool {
    /// Create a new TestTool with the given identifier
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            name: format!("test-tool-{}", id_str),
            description: "A test tool for integration testing".to_string(),
            invocation_count: Arc::new(RwLock::new(0)),
            should_fail: false,
        }
    }

    /// Create a TestTool that will fail when executed
    pub fn failing(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            name: format!("failing-test-tool-{}", id_str),
            description: "A test tool that always fails".to_string(),
            invocation_count: Arc::new(RwLock::new(0)),
            should_fail: true,
        }
    }

    /// Execute the tool with the given input
    pub async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, TestToolError> {
        // Increment invocation count
        {
            let mut count = self.invocation_count.write().await;
            *count += 1;
        }

        if self.should_fail {
            return Err(TestToolError::ExecutionFailed(
                "Tool configured to fail".to_string(),
            ));
        }

        Ok(serde_json::json!({
            "tool_id": self.id,
            "input": input,
            "status": "success"
        }))
    }

    /// Get the current invocation count
    pub async fn invocation_count(&self) -> usize {
        *self.invocation_count.read().await
    }
}

/// Error type for TestTool execution
#[derive(Debug, thiserror::Error)]
pub enum TestToolError {
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// TestLogger - Structured logger for integration tests.
///
/// Provides test-aware logging with automatic test name capture
/// and structured output format.
pub struct TestLogger {
    /// Current test name
    test_name: Option<String>,
    /// Log level for filtering
    level: tracing::Level,
}

impl TestLogger {
    /// Create a new TestLogger with INFO level
    pub fn new() -> Self {
        Self {
            test_name: None,
            level: tracing::Level::INFO,
        }
    }

    /// Create a TestLogger with a specific level
    pub fn with_level(level: tracing::Level) -> Self {
        Self {
            test_name: None,
            level,
        }
    }

    /// Set the current test name for logging context
    pub fn set_test_name(&mut self, name: impl Into<String>) {
        self.test_name = Some(name.into());
    }

    /// Log an info message
    pub fn info(&self, msg: &str) {
        self.log(tracing::Level::INFO, msg);
    }

    /// Log a warning message
    pub fn warn(&self, msg: &str) {
        self.log(tracing::Level::WARN, msg);
    }

    /// Log an error message
    pub fn error(&self, msg: &str) {
        self.log(tracing::Level::ERROR, msg);
    }

    /// Log a debug message
    pub fn debug(&self, msg: &str) {
        self.log(tracing::Level::DEBUG, msg);
    }

    fn log(&self, level: tracing::Level, msg: &str) {
        if level <= self.level {
            let prefix = self.test_name.as_deref().unwrap_or("unknown");
            eprintln!("[{}][{}] {}", level, prefix, msg);
        }
    }

    /// Initialize the test logger as the global subscriber
    pub fn init(&self) {
        let _ = tracing_subscriber::fmt()
            .with_max_level(self.level)
            .with_target(false)
            .with_writer(std::io::stderr)
            .try_init();
    }
}

impl Default for TestLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize tracing for tests - call once at test start
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

/// Helper to create a test session ID
pub fn test_session_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Helper to create test JSON input
pub fn test_input() -> serde_json::Value {
    serde_json::json!({
        "test": true,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
}
