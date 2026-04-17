//! Health Check Tools - Service health and availability checking

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;

/// Check if a service is healthy (HTTP HEAD request)
#[derive(Debug)]
pub struct HealthCheckTool;

#[derive(Debug, Deserialize)]
pub struct HealthCheckInput {
    /// URL to check
    pub url: String,
    /// Expected status code (default: 200)
    #[serde(default = "default_status")]
    pub expected_status: u16,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_status() -> u16 {
    200
}

fn default_timeout() -> u64 {
    5
}

impl Tool for HealthCheckTool {
    fn name(&self) -> &'static str {
        "health_check"
    }

    fn description(&self) -> &'static str {
        "Check if a service is healthy by making an HTTP HEAD request"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Health check parameters".into()),
            properties: Some(serde_json::json!({
                "url": {
                    "type": "string",
                    "description": "URL to check"
                },
                "expected_status": {
                    "type": "integer",
                    "description": "Expected HTTP status code (default: 200)"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Request timeout in seconds (default: 5)"
                }
            })),
            required: Some(vec!["url".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Health check result".into()),
            properties: Some(serde_json::json!({
                "healthy": { "type": "boolean" },
                "status_code": { "type": "integer" },
                "response_time_ms": { "type": "number" },
                "error": { "type": "string" }
            })),
            required: Some(vec!["healthy".into()]),
        }
    }

    fn permission(&self) -> Permission {
        Permission::Network {
            destinations: vec!["*".into()],
            protocols: vec!["http".into(), "https".into()],
            max_connections: 10,
        }
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: HealthCheckInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        let start = std::time::Instant::now();
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(input.timeout_secs))
            .build()
            .map_err(|e| crate::ToolError::ExecutionFailed(format!("Failed to create client: {}", e)))?;

        match client.head(&input.url).send() {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as f64;
                let status = response.status().as_u16();
                let healthy = status == input.expected_status;

                Ok(serde_json::json!({
                    "healthy": healthy,
                    "status_code": status,
                    "response_time_ms": elapsed,
                    "error": if !healthy { 
                        format!("Expected {}, got {}", input.expected_status, status) 
                    } else { 
                        String::new() 
                    }
                }))
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "healthy": false,
                    "status_code": 0,
                    "response_time_ms": start.elapsed().as_millis() as f64,
                    "error": e.to_string()
                }))
            }
        }
    }
}

/// Batch health check multiple services
#[derive(Debug)]
pub struct BatchHealthCheckTool;

#[derive(Debug, Deserialize)]
pub struct BatchHealthCheckInput {
    /// URLs to check
    pub urls: Vec<String>,
    /// Expected status code
    #[serde(default = "default_status")]
    pub expected_status: u16,
}

impl Tool for BatchHealthCheckTool {
    fn name(&self) -> &'static str {
        "batch_health_check"
    }

    fn description(&self) -> &'static str {
        "Check health of multiple services at once"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Batch health check parameters".into()),
            properties: Some(serde_json::json!({
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of URLs to check"
                },
                "expected_status": {
                    "type": "integer",
                    "description": "Expected HTTP status code (default: 200)"
                }
            })),
            required: Some(vec!["urls".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "array".into(),
            description: Some("Array of health check results".into()),
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission {
        Permission::Network {
            destinations: vec!["*".into()],
            protocols: vec!["http".into(), "https".into()],
            max_connections: 50,
        }
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: BatchHealthCheckInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| crate::ToolError::ExecutionFailed(format!("Failed to create client: {}", e)))?;

        let mut results = Vec::new();
        
        for url in input.urls {
            let start = std::time::Instant::now();
            match client.head(&url).send() {
                Ok(response) => {
                    let elapsed = start.elapsed().as_millis() as f64;
                    let status = response.status().as_u16();
                    results.push(serde_json::json!({
                        "url": url,
                        "healthy": status == input.expected_status,
                        "status_code": status,
                        "response_time_ms": elapsed,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "url": url,
                        "healthy": false,
                        "status_code": 0,
                        "response_time_ms": start.elapsed().as_millis() as f64,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        Ok(serde_json::json!(results))
    }
}
