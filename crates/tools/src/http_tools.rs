use crate::permissions::{Permission, PermissionError, PermissionResult};
use crate::tool::{ToolSpec, ToolCall, ToolOutput};
use reqwest::Client;

/// HTTP GET tool (standalone, no trait)
pub struct HttpGetTool;

impl HttpGetTool {
    pub fn new() -> Self {
        Self
    }

    pub fn spec() -> ToolSpec {
        ToolSpec::new("http_get", "Perform HTTP GET request")
            .require_permission(Permission::NetworkAccess)
            .with_parameters(serde_json::json!({
                "url": { "type": "string", "description": "URL to fetch" },
                "headers": { "type": "object", "description": "Optional headers map", "default": {} }
            }))
    }

    pub async fn execute(&self, call: ToolCall, _context: &crate::ExecutionContext) -> PermissionResult<ToolOutput> {
        let input = call.arguments;
        let url = input.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PermissionError::Tool("missing or invalid 'url' parameter".into()))?;

        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(header_map) = input.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in header_map {
                if let Some(val_str) = value.as_str() {
                    headers.insert(
                        reqwest::header::HeaderName::from_bytes(key.as_bytes())
                            .map_err(|e| PermissionError::Tool(format!("invalid header name {}: {}", key, e)))?,
                        reqwest::header::HeaderValue::from_str(val_str)
                            .map_err(|e| PermissionError::Tool(format!("invalid header value for {}: {}", key, e)))?
                    );
                }
            }
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| PermissionError::Tool(format!("failed to build HTTP client: {}", e)))?;

        let response = client.get(url).headers(headers).send().await
            .map_err(|e| PermissionError::Tool(format!("HTTP request failed: {}", e)))?;

        let status = response.status().as_u16();
        let text: String = response.text().await
            .map_err(|e| PermissionError::Tool(format!("failed to read response body: {}", e)))?;

        let mut result = serde_json::Map::new();
        result.insert("status".to_string(), serde_json::json!(status));
        result.insert("body".to_string(), serde_json::Value::String(text));
        result.insert("headers".to_string(), serde_json::json!({}));

        Ok(ToolOutput::json(serde_json::Value::Object(result)))
    }
}

/// HTTP POST tool (standalone, no trait)
pub struct HttpPostTool;

impl HttpPostTool {
    pub fn new() -> Self {
        Self
    }

    pub fn spec() -> ToolSpec {
        ToolSpec::new("http_post", "Perform HTTP POST request")
            .require_permission(Permission::NetworkAccess)
            .with_parameters(serde_json::json!({
                "url": { "type": "string", "description": "URL to POST to" },
                "body": { "type": "any", "description": "Optional request body", "default": null },
                "headers": { "type": "object", "description": "Optional headers map", "default": {} }
            }))
    }

    pub async fn execute(&self, call: ToolCall, _context: &crate::ExecutionContext) -> PermissionResult<ToolOutput> {
        let input = call.arguments;
        let url = input.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PermissionError::Tool("missing or invalid 'url' parameter".into()))?;

        let body = input.get("body").cloned().unwrap_or(serde_json::Value::Null);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json")
        );

        if let Some(header_map) = input.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in header_map {
                if let Some(val_str) = value.as_str() {
                    headers.insert(
                        reqwest::header::HeaderName::from_bytes(key.as_bytes())
                            .map_err(|e| PermissionError::Tool(format!("invalid header name {}: {}", key, e)))?,
                        reqwest::header::HeaderValue::from_str(val_str)
                            .map_err(|e| PermissionError::Tool(format!("invalid header value for {}: {}", key, e)))?
                    );
                }
            }
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| PermissionError::Tool(format!("failed to build HTTP client: {}", e)))?;

        let response = client.post(url).headers(headers).json(&body).send().await
            .map_err(|e| PermissionError::Tool(format!("HTTP request failed: {}", e)))?;

        let status = response.status().as_u16();
        let text: String = response.text().await
            .map_err(|e| PermissionError::Tool(format!("failed to read response body: {}", e)))?;

        let mut result = serde_json::Map::new();
        result.insert("status".to_string(), serde_json::json!(status));
        result.insert("body".to_string(), serde_json::Value::String(text));
        result.insert("headers".to_string(), serde_json::json!({}));

        Ok(ToolOutput::json(serde_json::Value::Object(result)))
    }
}
