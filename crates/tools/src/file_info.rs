//! File Info Tool - Get file metadata

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;
use std::fs;
use std::time::UNIX_EPOCH;

/// Get file metadata (size, permissions, timestamps)
#[derive(Debug, Clone)]
pub struct FileInfoTool;

impl Default for FileInfoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileInfoTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for FileInfoTool {
    fn name(&self) -> &'static str {
        "file_info"
    }

    fn description(&self) -> &'static str {
        "Get file metadata (size, permissions, timestamps)"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Get file metadata".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string", "description": "Path to the file" }
            })),
            required: Some(vec!["path".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("File metadata".into()),
            properties: Some(serde_json::json!({
                "size": { "type": "integer" },
                "is_file": { "type": "boolean" },
                "is_dir": { "type": "boolean" },
                "readonly": { "type": "boolean" },
                "modified": { "type": "string" },
                "accessed": { "type": "string" },
                "created": { "type": "string" }
            })),
            required: None,
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        #[derive(Deserialize)]
        struct FileInfoInput {
            path: String,
        }

        let input: FileInfoInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        let path = std::path::Path::new(&input.path);

        match fs::metadata(path) {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let accessed = metadata
                    .accessed()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let created = metadata
                    .created()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let result = serde_json::json!({
                    "size": metadata.len(),
                    "is_file": metadata.is_file(),
                    "is_dir": metadata.is_dir(),
                    "readonly": !metadata.permissions().readonly(),
                    "modified": format!("{}", modified),
                    "accessed": format!("{}", accessed),
                    "created": format!("{}", created)
                });

                Ok(result)
            }
            Err(e) => Err(crate::ToolError::ExecutionFailed(format!(
                "Failed to get metadata: {}",
                e
            ))),
        }
    }
}
