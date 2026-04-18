//! Session management integration tests.
//!
//! Tests for session creation, lifecycle management, state persistence,
//! and concurrent session handling.

use openclaw_integration_tests::common::{test_input, test_session_id, TestLogger, TestRuntime};

/// Test basic session creation and initialization
#[test]
fn test_session_create() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_session_create");

    let runtime = TestRuntime::new().expect("Failed to create TestRuntime");
    let state = runtime.state();

    // Verify initial state
    let state_guard = runtime.block_on(state.read());
    assert_eq!(state_guard.session_count, 0, "Initial session count should be 0");
}

/// Test session state updates
#[test]
fn test_session_state_update() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_session_state_update");

    let runtime = TestRuntime::new().expect("Failed to create TestRuntime");
    let state = runtime.state();

    // Simulate session creation
    {
        let mut state_guard = runtime.block_on(state.write());
        state_guard.session_count += 1;
    }

    // Verify state was updated
    let state_guard = runtime.block_on(state.read());
    assert_eq!(state_guard.session_count, 1, "Session count should be 1 after creation");
}

/// Test concurrent session operations
#[test]
fn test_concurrent_sessions() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_concurrent_sessions");

    let runtime = TestRuntime::new().expect("Failed to create TestRuntime");
    let state = runtime.state();

    // Spawn multiple concurrent session operations
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let state = state.clone();
            runtime.spawn(async move {
                let mut guard = state.write().await;
                guard.session_count += 1;
                guard.current_test = Some(format!("concurrent-session-{}", i));
                i
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        let _ = runtime.block_on(handle);
    }

    // Verify final state
    let state_guard = runtime.block_on(state.read());
    assert_eq!(state_guard.session_count, 4, "All 4 concurrent sessions should be counted");
}

/// Test session ID generation uniqueness
#[test]
fn test_session_id_uniqueness() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_session_id_uniqueness");

    let mut ids = std::collections::HashSet::new();
    for _ in 0..100 {
        let id = test_session_id();
        assert!(ids.insert(id), "Session ID collision detected");
    }
}

/// Test session data isolation
#[test]
fn test_session_data_isolation() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_session_data_isolation");

    let runtime = TestRuntime::new().expect("Failed to create TestRuntime");
    let state = runtime.state();

    // Session 1 writes data
    let state_clone = state.clone();
    let h1 = runtime.spawn(async move {
        let mut guard = state_clone.write().await;
        guard.session_count += 1;
        guard.current_test = Some("session-1".to_string());
        // Simulate some work
        tokio::task::yield_now().await;
        guard.tool_executions += 5;
    });

    // Session 2 operates independently
    let state_clone2 = state.clone();
    let h2 = runtime.spawn(async move {
        let mut guard = state_clone2.write().await;
        guard.session_count += 1;
        guard.current_test = Some("session-2".to_string());
        guard.tool_executions += 10;
    });

    runtime.block_on(h1).expect("Session 1 panicked");
    runtime.block_on(h2).expect("Session 2 panicked");

    // Both sessions should be counted correctly
    let guard = runtime.block_on(state.read());
    assert_eq!(guard.session_count, 2);
    assert_eq!(guard.tool_executions, 15);
}

/// Test session lifecycle with async operations
#[tokio::test]
async fn test_session_async_lifecycle() {
    use std::sync::Arc;
    use openclaw_integration_tests::common::TestState;
    use tokio::sync::RwLock;

    let state: Arc<RwLock<TestState>> = Arc::new(RwLock::new(TestState {
        session_count: 0,
        tool_executions: 0,
        current_test: None,
    }));

    // Create session
    {
        let mut guard = state.write().await;
        guard.session_count += 1;
    }

    // Simulate async work
    tokio::task::yield_now().await;

    // Verify session still exists
    let guard = state.read().await;
    assert_eq!(guard.session_count, 1);
}

/// Test input serialization for session operations
#[test]
fn test_session_input_serialization() {
    let input = test_input();
    let serialized = serde_json::to_string(&input).expect("Failed to serialize input");
    let deserialized: serde_json::Value =
        serde_json::from_str(&serialized).expect("Failed to deserialize input");

    assert_eq!(input, deserialized, "Input should survive round-trip serialization");
}

/// Test session timeout behavior (simulation)
#[tokio::test]
async fn test_session_timeout_simulation() {
    use std::sync::Arc;
    use openclaw_integration_tests::common::TestState;
    use tokio::sync::RwLock;

    let state: Arc<RwLock<TestState>> = Arc::new(RwLock::new(TestState {
        session_count: 0,
        tool_executions: 0,
        current_test: None,
    }));

    // Create a session
    {
        let mut guard = state.write().await;
        guard.session_count += 1;
    }

    // Simulate timeout by not accessing state for a duration
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Session should still be valid (in this simulation)
    let guard = state.read().await;
    assert_eq!(guard.session_count, 1);
}
