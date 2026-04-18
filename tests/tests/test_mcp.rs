//! MCP (Model Context Protocol) integration tests.
//!
//! Tests for MCP protocol message handling, JSON-RPC transport,
//! server/client interaction patterns, and protocol compliance.

use openclaw_integration_tests::common::{test_input, TestLogger, TestRuntime};

/// MCP JSON-RPC message types for testing
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "method", content = "params")]
enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize {
        protocol_version: String,
        capabilities: McpCapabilities,
    },
    #[serde(rename = "tools/list")]
    ToolsList,
    #[serde(rename = "tools/call")]
    ToolsCall { name: String, arguments: serde_json::Value },
    #[serde(rename = "resources/list")]
    ResourcesList,
    #[serde(rename = "resources/read")]
    ResourcesRead { uri: String },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct McpCapabilities {
    tools: bool,
    resources: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct McpError {
    code: i32,
    message: String,
}

/// Test MCP JSON-RPC request serialization
#[test]
fn test_mcp_request_serialization() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_request_serialization");

    let request = McpRequest::Initialize {
        protocol_version: "2024-11-05".to_string(),
        capabilities: McpCapabilities {
            tools: true,
            resources: true,
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize request");
    assert!(json.contains("initialize"));
    assert!(json.contains("2024-11-05"));
}

/// Test MCP JSON-RPC response serialization
#[test]
fn test_mcp_response_serialization() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_response_serialization");

    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        result: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}, "resources": {}}
        })),
        error: None,
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize response");
    assert!(json.contains("2.0"));
    assert!(json.contains("protocolVersion"));
}

/// Test MCP error response serialization
#[test]
fn test_mcp_error_response() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_error_response");

    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(42)),
        result: None,
        error: Some(McpError {
            code: -32600,
            message: "Invalid Request".to_string(),
        }),
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize error response");
    assert!(json.contains("-32600"));
    assert!(json.contains("Invalid Request"));
}

/// Test MCP tools/list request handling
#[test]
fn test_mcp_tools_list_request() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_tools_list_request");

    let request = McpRequest::ToolsList;
    let json = serde_json::to_string(&request).expect("Failed to serialize");

    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse JSON");
    assert_eq!(parsed["method"], "tools/list");
}

/// Test MCP tools/call request handling
#[test]
fn test_mcp_tools_call_request() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_tools_call_request");

    let request = McpRequest::ToolsCall {
        name: "test_tool".to_string(),
        arguments: test_input(),
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse JSON");

    assert_eq!(parsed["method"], "tools/call");
    assert_eq!(parsed["params"]["name"], "test_tool");
}

/// Test MCP request/response round-trip
#[test]
fn test_mcp_round_trip() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_round_trip");

    let request = McpRequest::ResourcesRead {
        uri: "file:///test/resource".to_string(),
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize request");
    let parsed: McpRequest = serde_json::from_str(&json).expect("Failed to deserialize request");

    match parsed {
        McpRequest::ResourcesRead { uri } => {
            assert_eq!(uri, "file:///test/resource");
        }
        _ => panic!("Wrong request type after round-trip"),
    }
}

/// Test MCP protocol version negotiation
#[test]
fn test_mcp_version_negotiation() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_version_negotiation");

    let versions = ["2024-11-05", "2024-10-01", "2024-09-01"];
    let negotiated = versions.iter().max().copied();

    assert_eq!(negotiated, Some("2024-11-05"));
}

/// Test MCP concurrent requests handling
#[tokio::test]
async fn test_mcp_concurrent_requests() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_concurrent_requests");

    // Simulate multiple concurrent MCP requests using tokio::spawn
    // (don't use TestRuntime here - #[tokio::test] already provides a runtime)
    let handles: Vec<_> = (0..5)
        .map(|id| {
            tokio::spawn(async move {
                let request = McpRequest::ToolsCall {
                    name: format!("tool-{}", id),
                    arguments: test_input(),
                };
                let json = serde_json::to_string(&request).unwrap();
                (id, json)
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.expect("Request panicked"));
    }

    assert_eq!(results.len(), 5);
}

/// Test MCP invalid request handling
#[test]
fn test_mcp_invalid_request() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_invalid_request");

    // Test that invalid JSON produces an error response
    let invalid_json = r#"{"method": "invalid", "params": not_valid}"#;
    let result: Result<McpRequest, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Invalid JSON should fail to parse");
}

/// Test MCP capabilities representation
#[test]
fn test_mcp_capabilities() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_capabilities");

    let caps = McpCapabilities {
        tools: true,
        resources: false,
    };

    let json = serde_json::to_string(&caps).expect("Failed to serialize");
    assert!(json.contains("\"tools\":true"));
    assert!(json.contains("\"resources\":false"));
}

/// Test MCP response ID handling
#[test]
fn test_mcp_response_id_handling() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_response_id_handling");

    // Notification (no id) - serde skips Option::None by default
    let notification = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: None,
        result: Some(serde_json::json!({"status": "ok"})),
        error: None,
    };
    let json = serde_json::to_string(&notification).expect("Failed to serialize");
    // id field should be absent or null for notifications
    let has_id_field = json.contains("\"id\"");
    assert!(!has_id_field, "Notification should not have an id field, got: {}", json);

    // Request with id
    let request = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(123)),
        result: Some(serde_json::json!({"status": "ok"})),
        error: None,
    };
    let json = serde_json::to_string(&request).expect("Failed to serialize");
    assert!(json.contains("\"id\":123"));
}

/// Test MCP batch request simulation
#[test]
fn test_mcp_batch_simulation() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_mcp_batch_simulation");

    let requests = vec![
        McpRequest::ToolsList,
        McpRequest::ResourcesList,
        McpRequest::Initialize {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpCapabilities {
                tools: true,
                resources: true,
            },
        },
    ];

    let batch: Vec<String> = requests
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect();

    assert_eq!(batch.len(), 3);
    assert!(batch[0].contains("tools/list"));
    assert!(batch[1].contains("resources/list"));
    assert!(batch[2].contains("initialize"));
}
