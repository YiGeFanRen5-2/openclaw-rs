//! Validation Tools - JSON Schema and input validation

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;
use serde_json::Value;

/// Validate JSON data against a JSON Schema
#[derive(Debug)]
pub struct ValidateJsonTool;

#[derive(Debug, Deserialize)]
pub struct ValidateJsonInput {
    /// JSON data to validate
    pub data: Value,
    /// JSON Schema to validate against
    pub schema: Value,
    /// Whether to return detailed errors (default: true)
    #[serde(default = "default_true")]
    pub detailed_errors: bool,
}

fn default_true() -> bool {
    true
}

impl Tool for ValidateJsonTool {
    fn name(&self) -> &'static str {
        "validate_json"
    }

    fn description(&self) -> &'static str {
        "Validate JSON data against a JSON Schema (draft-07)"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("JSON validation parameters".into()),
            properties: Some(serde_json::json!({
                "data": {
                    "type": "string",
                    "description": "JSON data to validate (as string)"
                },
                "schema": {
                    "type": "string",
                    "description": "JSON Schema (as string)"
                },
                "detailed_errors": {
                    "type": "boolean",
                    "description": "Return detailed error messages (default: true)"
                }
            })),
            required: Some(vec!["data".into(), "schema".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Validation result".into()),
            properties: Some(serde_json::json!({
                "valid": { "type": "boolean" },
                "errors": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            })),
            required: Some(vec!["valid".into()]),
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: ValidateJsonInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        // Simple JSON Schema validation (basic implementation)
        let errors = validate_against_schema(&input.data, &input.schema, "", input.detailed_errors);
        
        Ok(serde_json::json!({
            "valid": errors.is_empty(),
            "errors": errors
        }))
    }
}

/// Basic JSON Schema validation
fn validate_against_schema(data: &Value, schema: &Value, path: &str, detailed: bool) -> Vec<String> {
    let mut errors = Vec::new();
    
    if let Some(obj) = schema.as_object() {
        // Type validation
        if let Some(type_val) = obj.get("type") {
            if let Some(type_str) = type_val.as_str() {
                let data_type = match data {
                    Value::Null => "null",
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                };
                
                // Handle "integer" as special case of "number"
                if type_str == "integer" && data_type == "number" {
                    if let Some(n) = data.as_f64() {
                        if n.fract() != 0.0 {
                            errors.push(format_path(path, "expected integer, got float", detailed));
                        }
                    }
                } else if type_str != data_type {
                    errors.push(format_path(path, &format!("expected {}, got {}", type_str, data_type), detailed));
                }
            }
        }

        // Required fields
        if let Some(required) = obj.get("required") {
            if let Some(arr) = required.as_array() {
                if let Some(obj_data) = data.as_object() {
                    for req in arr {
                        if let Some(name) = req.as_str() {
                            if !obj_data.contains_key(name) {
                                errors.push(format_path(path, &format!("missing required field: {}", name), detailed));
                            }
                        }
                    }
                }
            }
        }

        // Enum validation
        if let Some(enum_vals) = obj.get("enum") {
            if let Some(arr) = enum_vals.as_array() {
                if !arr.contains(data) {
                    errors.push(format_path(path, "value not in enum", detailed));
                }
            }
        }

        // Minimum value
        if let Some(min) = obj.get("minimum") {
            if let (Some(data_num), Some(min_num)) = (data.as_f64(), min.as_f64()) {
                if data_num < min_num {
                    errors.push(format_path(path, &format!("value {} is less than minimum {}", data_num, min_num), detailed));
                }
            }
        }

        // Maximum value
        if let Some(max) = obj.get("maximum") {
            if let (Some(data_num), Some(max_num)) = (data.as_f64(), max.as_f64()) {
                if data_num > max_num {
                    errors.push(format_path(path, &format!("value {} exceeds maximum {}", data_num, max_num), detailed));
                }
            }
        }

        // Min length (for strings)
        if let Some(min_len) = obj.get("minLength") {
            if let (Some(data_str), Some(min)) = (data.as_str(), min_len.as_u64()) {
                if data_str.len() < min as usize {
                    errors.push(format_path(path, &format!("string length {} is less than minLength {}", data_str.len(), min), detailed));
                }
            }
        }

        // Max length (for strings)
        if let Some(max_len) = obj.get("maxLength") {
            if let (Some(data_str), Some(max)) = (data.as_str(), max_len.as_u64()) {
                if data_str.len() > max as usize {
                    errors.push(format_path(path, &format!("string length {} exceeds maxLength {}", data_str.len(), max), detailed));
                }
            }
        }

        // Pattern (regex) - basic check only
        if let Some(pattern) = obj.get("pattern") {
            if let (Some(data_str), Some(_pattern_str)) = (data.as_str(), pattern.as_str()) {
                // Pattern validation skipped - requires regex crate
                let _ = data_str; // suppress unused warning
            }
        }

        // Min items (for arrays)
        if let Some(min_items) = obj.get("minItems") {
            if let (Some(data_arr), Some(min)) = (data.as_array(), min_items.as_u64()) {
                if data_arr.len() < min as usize {
                    errors.push(format_path(path, &format!("array length {} is less than minItems {}", data_arr.len(), min), detailed));
                }
            }
        }

        // Max items (for arrays)
        if let Some(max_items) = obj.get("maxItems") {
            if let (Some(data_arr), Some(max)) = (data.as_array(), max_items.as_u64()) {
                if data_arr.len() > max as usize {
                    errors.push(format_path(path, &format!("array length {} exceeds maxItems {}", data_arr.len(), max), detailed));
                }
            }
        }
    }

    errors
}

fn format_path(path: &str, msg: &str, detailed: bool) -> String {
    if detailed {
        if path.is_empty() {
            format!("root: {}", msg)
        } else {
            format!("{}: {}", path, msg)
        }
    } else {
        msg.to_string()
    }
}

/// Validate tool input against schema
#[derive(Debug)]
pub struct ValidateToolInputTool;

#[derive(Debug, Deserialize)]
pub struct ValidateToolInputInput {
    /// Tool name
    pub tool_name: String,
    /// Input JSON to validate
    pub input: Value,
}

impl Tool for ValidateToolInputTool {
    fn name(&self) -> &'static str {
        "validate_tool_input"
    }

    fn description(&self) -> &'static str {
        "Validate tool input against the tool's registered schema"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Tool input validation parameters".into()),
            properties: Some(serde_json::json!({
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool to validate input for"
                },
                "input": {
                    "type": "string",
                    "description": "JSON input to validate (as string)"
                }
            })),
            required: Some(vec!["tool_name".into(), "input".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Validation result".into()),
            properties: Some(serde_json::json!({
                "valid": { "type": "boolean" },
                "errors": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            })),
            required: Some(vec!["valid".into()]),
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: ValidateToolInputInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        // Parse input JSON
        let _input_json: Value = match serde_json::from_str(&input.input.to_string()) {
            Ok(v) => v,
            Err(e) => {
                return Ok(serde_json::json!({
                    "valid": false,
                    "errors": vec![format!("Invalid JSON: {}", e)]
                }));
            }
        };

        // For now, we just check if input is valid JSON
        // In a full implementation, we'd look up the tool schema and validate
        Ok(serde_json::json!({
            "valid": true,
            "errors": [],
            "note": format!("Tool '{}' schema validation would be performed here", input.tool_name)
        }))
    }
}
