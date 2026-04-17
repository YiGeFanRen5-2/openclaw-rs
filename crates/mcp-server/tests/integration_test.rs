//! MCP Server Unit Tests
//! Tests for JSON-RPC protocol, tools, resources, and prompts.

#[cfg(test)]
mod tests {
    use mcp_server::{
        substitute_prompt_messages, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, McpServer,
        McpTool, PromptContent, PromptMessage, RequestId,
    };
    use serde_json::json;

    #[test]
    fn test_json_rpc_request_parsing() {
        let req_str = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0" }
            }
        }"#;
        let req: JsonRpcRequest = serde_json::from_str(req_str).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.id, Some(RequestId::Number(1)));
    }

    #[test]
    fn test_json_rpc_error_response() {
        let err = JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        };
        let resp = JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            error: err,
        };
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("Method not found"));
    }

    #[test]
    fn test_tool_registration() {
        let mut server = McpServer::new("test", "1.0");
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                }
            }),
        };
        server.register_tool(tool);
        assert_eq!(server.tools.len(), 1);
        assert_eq!(server.tools[0].name, "test_tool");
    }

    #[test]
    fn test_prompt_substitution() {
        let messages = vec![PromptMessage {
            role: "user".to_string(),
            content: PromptContent::Text {
                text: "Hello, {{name}}!".to_string(),
            },
        }];
        let args = json!({ "name": "World" });
        let result = substitute_prompt_messages(&messages, &args);
        match &result[0].content {
            PromptContent::Text { text } => {
                assert_eq!(text, "Hello, World!");
            }
            _ => panic!("expected Text content"),
        }
    }
}
