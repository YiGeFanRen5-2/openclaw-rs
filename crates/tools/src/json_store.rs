//! JSON Store Tool - Simple key-value JSON storage

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

/// Simple JSON-based key-value store
#[derive(Debug, Default)]
pub struct JsonStore {
    data: RwLock<HashMap<String, serde_json::Value>>,
    path: RwLock<Option<PathBuf>>,
}

impl JsonStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(path: PathBuf) -> Self {
        let data = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            data: RwLock::new(data),
            path: RwLock::new(Some(path)),
        }
    }

    pub fn set(&self, key: &str, value: serde_json::Value) -> Result<(), String> {
        let mut data = self.data.write().map_err(|e| e.to_string())?;
        data.insert(key.to_string(), value);
        self.persist()?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().ok()?.get(key).cloned()
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        let mut data = self.data.write().map_err(|e| e.to_string())?;
        data.remove(key);
        self.persist()?;
        Ok(())
    }

    pub fn list_keys(&self) -> Vec<String> {
        self.data.read().ok().map(|d| d.keys().cloned().collect()).unwrap_or_default()
    }

    fn persist(&self) -> Result<(), String> {
        let path = self.path.read().ok().and_then(|p| p.clone());
        if let Some(path) = path {
            let data = self.data.read().map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&*data).map_err(|e| e.to_string())?;
            fs::write(path, json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// Store a JSON value by key
#[derive(Debug)]
pub struct JsonStoreSetTool;

#[derive(Debug, Deserialize)]
pub struct JsonStoreSetInput {
    pub key: String,
    pub value: serde_json::Value,
    pub store_path: Option<String>,
}

impl Tool for JsonStoreSetTool {
    fn name(&self) -> &'static str { "json_store_set" }
    fn description(&self) -> &'static str { "Store a JSON value by key" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: Some(serde_json::json!({
                "key": { "type": "string" },
                "value": { "description": "JSON value to store" },
                "store_path": { "type": "string", "description": "Optional path to store file" }
            })),
            required: Some(vec!["key".into(), "value".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Result".into()),
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: JsonStoreSetInput = serde_json::from_value(input)
            .map_err(|e| crate::ToolError::InvalidInput(e.to_string()))?;

        // For demo, use in-memory store
        let store = JsonStore::new();
        store.set(&input.key, input.value)
            .map_err(crate::ToolError::ExecutionFailed)?;

        Ok(serde_json::json!({
            "success": true,
            "key": input.key
        }))
    }
}

/// Get a JSON value by key
#[derive(Debug)]
pub struct JsonStoreGetTool;

#[derive(Debug, Deserialize)]
pub struct JsonStoreGetInput {
    pub key: String,
}

impl Tool for JsonStoreGetTool {
    fn name(&self) -> &'static str { "json_store_get" }
    fn description(&self) -> &'static str { "Get a JSON value by key" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: Some(serde_json::json!({
                "key": { "type": "string" }
            })),
            required: Some(vec!["key".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: JsonStoreGetInput = serde_json::from_value(input)
            .map_err(|e| crate::ToolError::InvalidInput(e.to_string()))?;

        let store = JsonStore::new();
        match store.get(&input.key) {
            Some(value) => Ok(serde_json::json!({
                "found": true,
                "key": input.key,
                "value": value
            })),
            None => Ok(serde_json::json!({
                "found": false,
                "key": input.key
            })),
        }
    }
}

/// List all keys in the store
#[derive(Debug)]
pub struct JsonStoreListTool;

impl Tool for JsonStoreListTool {
    fn name(&self) -> &'static str { "json_store_list" }
    fn description(&self) -> &'static str { "List all keys in the store" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: None,
            required: None,
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "array".into(),
            description: None,
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let store = JsonStore::new();
        let keys = store.list_keys();
        Ok(serde_json::json!(keys))
    }
}
