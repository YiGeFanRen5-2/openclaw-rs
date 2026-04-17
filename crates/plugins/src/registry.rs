//! Plugin Registry - Discover, validate, and manage plugins
//!
//! Provides a central registry for plugin metadata with version
//! compatibility checking and dependency resolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Plugin registry entry with validation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistryEntry {
    pub manifest: super::PluginManifest,
    pub installed_at: String,
    pub source: PluginSource,
    /// Semantic version of OpenClaw required
    pub requires_openclaw: Option<String>,
}

/// Where the plugin was installed from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSource {
    Local,
    GitHub { repo: String, tag: String },
    Registry { index: usize },
}

/// Plugin Registry - manages plugin metadata and discovery
#[derive(Debug, Default)]
pub struct PluginRegistry {
    entries: HashMap<String, PluginRegistryEntry>,
    sources: Vec<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin from a manifest file
    pub fn register(&mut self, manifest: super::PluginManifest, source: PluginSource) -> Result<(), RegistryError> {
        // Validate manifest
        self.validate_manifest(&manifest)?;

        let entry = PluginRegistryEntry {
            manifest,
            installed_at: chrono::Utc::now().to_rfc3339(),
            source,
            requires_openclaw: None,
        };

        self.entries.insert(entry.manifest.id.clone(), entry);
        Ok(())
    }

    /// Register from a plugin.json file path
    pub fn register_from_file(&mut self, path: &Path) -> Result<(), RegistryError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RegistryError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;
        
        let manifest: super::PluginManifest = serde_json::from_str(&content)
            .map_err(|e| RegistryError::InvalidManifest(format!("Invalid JSON: {}", e)))?;

        let source = PluginSource::Local;

        self.register(manifest, source)
    }

    /// Validate plugin manifest
    fn validate_manifest(&self, manifest: &super::PluginManifest) -> Result<(), RegistryError> {
        // Check required fields
        if manifest.id.is_empty() {
            return Err(RegistryError::InvalidManifest("Plugin ID cannot be empty".into()));
        }
        if manifest.name.is_empty() {
            return Err(RegistryError::InvalidManifest("Plugin name cannot be empty".into()));
        }
        if manifest.version.is_empty() {
            return Err(RegistryError::InvalidManifest("Plugin version cannot be empty".into()));
        }

        // Check semantic version format
        if !is_valid_semver(&manifest.version) {
            return Err(RegistryError::InvalidManifest(format!(
                "Invalid semver: '{}'. Expected format: major.minor.patch", 
                manifest.version
            )));
        }

        // Check for duplicate tool names
        let mut tool_names: HashSet<&str> = HashSet::new();
        for tool in &manifest.tools {
            if !tool_names.insert(&tool.name) {
                return Err(RegistryError::InvalidManifest(format!(
                    "Duplicate tool name: {}", tool.name
                )));
            }
        }

        // Check for duplicate resource names
        let mut resource_names: HashSet<&str> = HashSet::new();
        for resource in &manifest.resources {
            if !resource_names.insert(&resource.name) {
                return Err(RegistryError::InvalidManifest(format!(
                    "Duplicate resource name: {}", resource.name
                )));
            }
        }

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<&PluginRegistryEntry> {
        self.entries.get(id)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<&PluginRegistryEntry> {
        self.entries.values().collect()
    }

    /// Remove a plugin
    pub fn unregister(&mut self, id: &str) -> Option<PluginRegistryEntry> {
        self.entries.remove(id)
    }

    /// Check version compatibility
    pub fn check_compatibility(&self, openclaw_version: &str) -> Vec<CompatibilityIssue> {
        let mut issues = Vec::new();
        
        for entry in self.entries.values() {
            if let Some(required) = &entry.requires_openclaw {
                if !is_compatible(required, openclaw_version) {
                    issues.push(CompatibilityIssue {
                        plugin_id: entry.manifest.id.clone(),
                        required: required.clone(),
                        actual: openclaw_version.to_string(),
                    });
                }
            }
        }
        
        issues
    }

    /// Export registry as JSON
    pub fn export_json(&self) -> Result<String, RegistryError> {
        serde_json::to_string_pretty(&self.entries)
            .map_err(|e| RegistryError::SerializationError(e.to_string()))
    }

    /// Import registry from JSON
    pub fn import_json(&mut self, json: &str) -> Result<(), RegistryError> {
        let entries: HashMap<String, PluginRegistryEntry> = serde_json::from_str(json)
            .map_err(|e| RegistryError::SerializationError(e.to_string()))?;
        self.entries = entries;
        Ok(())
    }

    /// Add a plugin source (registry URL)
    pub fn add_source(&mut self, url: String) {
        self.sources.push(url);
    }

    /// Get all plugin sources
    pub fn sources(&self) -> &[String] {
        &self.sources
    }
}

/// Registry error types
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Version conflict: {0}")]
    VersionConflict(String),
}

/// Compatibility issue between plugin and OpenClaw version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub plugin_id: String,
    pub required: String,
    pub actual: String,
}

// ─── Semantic Version Utilities ─────────────────────────────────────────────

fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

fn is_compatible(required: &str, actual: &str) -> bool {
    let req_parts: Vec<u32> = required.split('.').filter_map(|s| s.parse().ok()).collect();
    let act_parts: Vec<u32> = actual.split('.').filter_map(|s| s.parse().ok()).collect();
    
    if req_parts.is_empty() || act_parts.is_empty() {
        return true; // Can't determine, assume compatible
    }
    
    // Major version must match
    if req_parts[0] != act_parts[0] {
        return false;
    }
    
    // Minor version must be >= required
    if act_parts.get(1).unwrap_or(&0) < req_parts.get(1).unwrap_or(&0) {
        return false;
    }
    
    true
}

use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_validation() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("10.20.30"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.a"));
        assert!(!is_valid_semver("v1.0.0"));
    }

    #[test]
    fn test_semver_compatibility() {
        assert!(is_compatible("0.1.0", "0.1.0"));
        assert!(is_compatible("0.1.0", "0.2.0"));
        assert!(is_compatible("0.1.0", "0.1.5"));
        assert!(!is_compatible("0.1.0", "1.0.0"));
        assert!(!is_compatible("1.0.0", "0.9.0"));
    }

    #[test]
    fn test_registry_operations() {
        let mut registry = PluginRegistry::new();
        
        let manifest = super::super::PluginManifest {
            id: "test-plugin".into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            author: Some("Test Author".into()),
            hooks: vec![],
            tools: vec![],
            resources: vec![],
            permissions: vec![],
        };
        
        registry.register(manifest, PluginSource::Local).unwrap();
        assert!(registry.get("test-plugin").is_some());
        assert!(registry.get("nonexistent").is_none());
        
        registry.unregister("test-plugin").unwrap();
        assert!(registry.get("test-plugin").is_none());
    }
}
