//! Tool execution integration tests.
//!
//! Tests for tool registration, invocation, parameter handling,
//! error cases, and concurrent tool execution.

use openclaw_integration_tests::common::{test_input, TestLogger, TestTool};

/// Test basic tool creation and registration
#[test]
fn test_tool_creation() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_creation");

    let tool = TestTool::new("test-tool-1");
    assert_eq!(tool.id, "test-tool-1");
    assert_eq!(tool.name, "test-tool-test-tool-1");
}

/// Test tool execution success
#[tokio::test]
async fn test_tool_execution_success() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_execution_success");

    let tool = TestTool::new("success-tool");
    let input = test_input();

    let result = tool.execute(input.clone()).await;
    assert!(result.is_ok(), "Tool execution should succeed");

    let output = result.unwrap();
    assert_eq!(output["tool_id"], "success-tool");
    assert_eq!(output["status"], "success");
}

/// Test tool execution failure
#[tokio::test]
async fn test_tool_execution_failure() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_execution_failure");

    let tool = TestTool::failing("failing-tool");
    let input = test_input();

    let result = tool.execute(input).await;
    assert!(result.is_err(), "Tool execution should fail");
}

/// Test tool invocation counting
#[tokio::test]
async fn test_tool_invocation_count() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_invocation_count");

    let tool = TestTool::new("counted-tool");
    let input = test_input();

    // Execute tool multiple times
    for _ in 0..5 {
        let _ = tool.execute(input.clone()).await;
    }

    let count = tool.invocation_count().await;
    assert_eq!(count, 5, "Tool should have been invoked 5 times");
}

/// Test multiple tools with concurrent execution
#[tokio::test]
async fn test_concurrent_tool_execution() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_concurrent_tool_execution");

    let tool1 = TestTool::new("concurrent-tool-1");
    let tool2 = TestTool::new("concurrent-tool-2");
    let tool3 = TestTool::new("concurrent-tool-3");
    let input = test_input();

    // Execute all tools concurrently using tokio::spawn
    let h1 = tokio::spawn({
        let tool1 = tool1.clone();
        let input = input.clone();
        async move {
            tool1.execute(input).await
        }
    });
    let h2 = tokio::spawn({
        let tool2 = tool2.clone();
        let input = input.clone();
        async move {
            tool2.execute(input).await
        }
    });
    let h3 = tokio::spawn({
        let tool3 = tool3.clone();
        let input = input.clone();
        async move {
            tool3.execute(input).await
        }
    });

    let results = vec![
        h1.await.expect("Tool 1 panicked"),
        h2.await.expect("Tool 2 panicked"),
        h3.await.expect("Tool 3 panicked"),
    ];

    // All should succeed
    for result in results {
        assert!(result.is_ok(), "Concurrent tool execution should succeed");
    }

    // All invocation counts should be 1
    assert_eq!(tool1.invocation_count().await, 1);
    assert_eq!(tool2.invocation_count().await, 1);
    assert_eq!(tool3.invocation_count().await, 1);
}

/// Test tool with various input types
#[tokio::test]
async fn test_tool_input_variety() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_input_variety");

    let tool = TestTool::new("variety-tool");

    // Test with null
    let result = tool.execute(serde_json::json!(null)).await;
    assert!(result.is_ok());

    // Test with string
    let result = tool.execute(serde_json::json!("hello")).await;
    assert!(result.is_ok());

    // Test with number
    let result = tool.execute(serde_json::json!(42)).await;
    assert!(result.is_ok());

    // Test with array
    let result = tool.execute(serde_json::json!([1, 2, 3])).await;
    assert!(result.is_ok());

    // Test with object
    let result = tool.execute(serde_json::json!({"key": "value"})).await;
    assert!(result.is_ok());

    assert_eq!(tool.invocation_count().await, 5);
}

/// Test tool error handling
#[tokio::test]
async fn test_tool_error_handling() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_error_handling");

    let tool = TestTool::failing("error-tool");
    let result = tool.execute(test_input()).await;

    match result {
        Err(openclaw_integration_tests::common::TestToolError::ExecutionFailed(msg)) => {
            assert!(msg.contains("fail"));
        }
        _ => panic!("Expected ExecutionFailed error"),
    }
}

/// Test tool cloning shares state via Arc
#[tokio::test]
async fn test_tool_clone_shares_state() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_tool_clone_shares_state");

    let tool = TestTool::new("original-tool");
    let tool_clone = tool.clone();

    // Execute on original
    let _ = tool.execute(test_input()).await;

    // Execute on clone
    let _ = tool_clone.execute(test_input()).await;

    // Both share the same Arc counter, so both see 2 invocations
    assert_eq!(tool.invocation_count().await, 2);
    assert_eq!(tool_clone.invocation_count().await, 2);
}

/// Test parallel execution of same tool
#[tokio::test]
async fn test_parallel_same_tool() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_parallel_same_tool");

    let tool = TestTool::new("parallel-tool");
    let input = test_input();

    // Spawn 10 parallel executions of the same tool
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let tool = tool.clone();
            let input = input.clone();
            tokio::spawn(async move {
                tool.execute(input).await
            })
        })
        .collect();

    // Wait for all using futures::join_all
    let results: Vec<Result<serde_json::Value, _>> =
        futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("Task panicked"))
            .collect();

    // All should succeed
    for result in &results {
        assert!(result.is_ok(), "Parallel execution should succeed");
    }

    // Invocation count should be exactly 10
    assert_eq!(tool.invocation_count().await, 10);
}
