//! Text Processing Tools - Simple text utilities using std only

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hash a string using simple algorithms (built-in)
#[derive(Debug)]
pub struct HashTool;

#[derive(Debug, Deserialize)]
pub struct HashInput {
    pub data: String,
}

impl Tool for HashTool {
    fn name(&self) -> &'static str { "hash" }
    fn description(&self) -> &'static str { "Hash a string (returns hex digest)" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: Some(serde_json::json!({
                "data": { "type": "string" }
            })),
            required: Some(vec!["data".into()]),
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
        let input: HashInput = serde_json::from_value(input)
            .map_err(|e| crate::ToolError::InvalidInput(e.to_string()))?;

        let mut hasher = DefaultHasher::new();
        input.data.hash(&mut hasher);
        let hash = hasher.finish();
        
        Ok(serde_json::json!({ 
            "hash": format!("{:016x}", hash),
            "algorithm": "DefaultHasher"
        }))
    }
}

/// Generate a UUID-like identifier
#[derive(Debug)]
pub struct UuidTool;

impl Tool for UuidTool {
    fn name(&self) -> &'static str { "uuid" }
    fn description(&self) -> &'static str { "Generate a unique ID" }

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
            r#type: "object".into(),
            description: None,
            properties: None,
            required: None,
        }
    }

    fn permission(&self) -> Permission { Permission::Safe }

    fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        
        let uuid = format!("{:032x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (timestamp >> 96) & 0xffffffff,
            (timestamp >> 80) & 0xffff,
            (timestamp >> 64) & 0xffff,
            ((timestamp >> 48) & 0xffff) as u16,
            timestamp & 0xffffffffffff
        );
        
        Ok(serde_json::json!({ "uuid": uuid }))
    }
}

/// Random string generator
#[derive(Debug)]
pub struct RandomStringTool;

#[derive(Debug, Deserialize)]
pub struct RandomStringInput {
    pub length: Option<usize>,
}

impl Tool for RandomStringTool {
    fn name(&self) -> &'static str { "random_string" }
    fn description(&self) -> &'static str { "Generate a random alphanumeric string" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: Some(serde_json::json!({
                "length": { "type": "integer", "default": 32 }
            })),
            required: None,
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
        let input: RandomStringInput = serde_json::from_value(input)
            .map_err(|e| crate::ToolError::InvalidInput(e.to_string()))?;

        let length = input.length.unwrap_or(32).min(128);
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars().collect();
        
        let result: String = (0..length)
            .map(|i| {
                // Use nanoseconds and index for pseudo-randomness
                let seed = (std::time::Instant::now().elapsed().as_nanos() as usize) 
                    .wrapping_mul(31)
                    .wrapping_add(i * 17);
                let idx = seed % chars.len();
                chars[idx]
            })
            .collect();

        Ok(serde_json::json!({ "random": result, "length": length }))
    }
}

/// Count characters, words, lines in text
#[derive(Debug)]
pub struct TextStatsTool;

#[derive(Debug, Deserialize)]
pub struct TextStatsInput {
    pub text: String,
}

impl Tool for TextStatsTool {
    fn name(&self) -> &'static str { "text_stats" }
    fn description(&self) -> &'static str { "Get statistics about text (chars, words, lines)" }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: None,
            properties: Some(serde_json::json!({
                "text": { "type": "string" }
            })),
            required: Some(vec!["text".into()]),
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
        let input: TextStatsInput = serde_json::from_value(input)
            .map_err(|e| crate::ToolError::InvalidInput(e.to_string()))?;

        let chars = input.text.chars().count();
        let words = input.text.split_whitespace().count();
        let lines = input.text.lines().count();

        Ok(serde_json::json!({
            "characters": chars,
            "words": words,
            "lines": lines
        }))
    }
}
