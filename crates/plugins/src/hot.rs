//! Plugin Hot-Reload Module
//!
//! Uses `libloading` to dynamically load plugin `.so` files at runtime,
//! allowing plugins to be updated without restarting the OpenClaw process.
//!
//! ## Usage
//! ```ignore
//! let loader = PluginLoader::new("/path/to/plugins")?;
//! let plugin = loader.load("my-plugin")?;
//! plugin.call("init", args).await?;
//! // Reload the plugin in-place:
//! let reloaded = loader.reload("my-plugin")?;
//! ```

use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during hot reloading.
#[derive(Error, Debug)]
pub enum HotReloadError {
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("failed to load library: {0}")]
    LoadFailed(String),
    #[error("symbol not found in plugin: {0}")]
    SymbolNotFound(String),
    #[error("plugin manifest invalid: {0}")]
    ManifestInvalid(String),
    #[error("reload failed: {0}")]
    ReloadFailed(String),
}

pub type Result<T> = std::result::Result<T, HotReloadError>;

/// Plugin metadata loaded from a `.so` file's manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPluginInfo {
    pub id: String,
    pub version: String,
    pub path: PathBuf,
}

/// Dynamically loads and hot-reloads plugin `.so` files.
pub struct PluginLoader {
    /// Base directory to search for plugins.
    base_dir: PathBuf,
    /// Currently loaded libraries.
    libraries: HashMap<String, Library>,
    /// Loaded plugin metadata.
    plugins: HashMap<String, LoadedPluginInfo>,
}

impl PluginLoader {
    /// Create a new plugin loader that searches for plugins in `base_dir`.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            libraries: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    /// Set the base directory and return self for chaining.
    pub fn base_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.base_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Load a plugin by its ID (directory name under base_dir).
    ///
    /// Looks for `base_dir/<id>/plugin.so` and `base_dir/<id>/manifest.json`.
    pub fn load(&mut self, id: &str) -> Result<&LoadedPluginInfo> {
        let plugin_dir = self.base_dir.join(id);
        let so_path = plugin_dir.join("plugin.so");
        let manifest_path = plugin_dir.join("manifest.json");

        if !so_path.exists() {
            return Err(HotReloadError::NotFound(format!(
                "plugin.so not found at {}",
                so_path.display()
            )));
        }

        // Load the library.
        let lib = unsafe {
            Library::new(&so_path)
                .map_err(|e| HotReloadError::LoadFailed(format!("{}: {}", so_path.display(), e)))?
        };

        // Load manifest.
        let manifest: LoadedPluginInfo = if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)
                .map_err(|e| HotReloadError::ManifestInvalid(e.to_string()))?;
            serde_json::from_str(&content)
                .map_err(|e| HotReloadError::ManifestInvalid(e.to_string()))?
        } else {
            LoadedPluginInfo {
                id: id.to_string(),
                version: "0.0.0".to_string(),
                path: so_path.clone(),
            }
        };

        // Check for required symbols.
        unsafe {
            let _init: Symbol<unsafe extern "C" fn()> = lib
                .get(b"openclaw_plugin_init")
                .map_err(|_| HotReloadError::SymbolNotFound("openclaw_plugin_init".to_string()))?;
            // Call init if present.
            let init: Symbol<unsafe extern "C" fn()> = lib.get(b"openclaw_plugin_init").unwrap();
            init();
        }

        self.libraries.insert(id.to_string(), lib);
        self.plugins.insert(id.to_string(), manifest.clone());

        Ok(self.plugins.get(id).unwrap())
    }

    /// Reload a previously loaded plugin (hot swap).
    ///
    /// The old library is dropped and a new one is loaded from disk.
    /// This allows updating a plugin without restarting the process.
    pub fn reload(&mut self, id: &str) -> Result<&LoadedPluginInfo> {
        // Remove old library.
        self.libraries.remove(id);
        self.plugins.remove(id);

        // Re-load from disk.
        self.load(id)
    }

    /// Unload a plugin (stop it).
    pub fn unload(&mut self, id: &str) -> Result<()> {
        self.libraries.remove(id);
        self.plugins.remove(id);
        Ok(())
    }

    /// Call a symbol (function) in a loaded plugin by name.
    ///
    /// # Safety
    /// The plugin must export the requested symbol. Undefined behavior if
    /// the symbol has an incompatible ABI.
    pub unsafe fn call<Args, Ret>(&mut self, id: &str, symbol: &str, args: Args) -> Result<Ret>
    where
        Args: Serialize,
        Ret: serde::de::DeserializeOwned,
    {
        let lib = self
            .libraries
            .get_mut(id)
            .ok_or_else(|| HotReloadError::NotFound(id.to_string()))?;

        let sym_name = format!("openclaw_plugin_{}", symbol);
        let sym: Symbol<unsafe extern "C" fn(*const u8, usize) -> *mut u8> = lib
            .get(sym_name.as_bytes())
            .map_err(|_| HotReloadError::SymbolNotFound(sym_name))?;

        // Serialize args, call, deserialize result.
        let args_json = serde_json::to_vec(&args)
            .map_err(|e| HotReloadError::LoadFailed(format!("args serialization: {}", e)))?;

        let result_ptr = sym(args_json.as_ptr(), args_json.len());
        if result_ptr.is_null() {
            return Err(HotReloadError::LoadFailed(
                "plugin call returned null".to_string(),
            ));
        }

        let result_len = *(result_ptr as *const usize).offset(-1) as usize;
        let result_slice = std::slice::from_raw_parts(result_ptr, result_len);
        let ret: Ret = serde_json::from_slice(result_slice)
            .map_err(|e| HotReloadError::LoadFailed(format!("result deserialization: {}", e)))?;

        // Free the result buffer (plugin is responsible for allocating with the same allocator).
        // For simplicity we leak here; in production, plugins should use a shared allocator.
        let _ = result_ptr; // Leaking is intentional; plugin manages the buffer.

        Ok(ret)
    }

    /// List all currently loaded plugin IDs.
    pub fn list_loaded(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Get info about a loaded plugin.
    pub fn info(&self, id: &str) -> Option<&LoadedPluginInfo> {
        self.plugins.get(id)
    }

    /// Discover all plugin directories under base_dir.
    pub fn discover(&self) -> Vec<String> {
        let mut plugins = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("plugin.so").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        plugins.push(name.to_string());
                    }
                }
            }
        }
        plugins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = PluginLoader::new("/tmp/plugins");
        assert_eq!(loader.list_loaded().len(), 0);
    }

    #[test]
    fn test_discover_nonexistent_dir() {
        let loader = PluginLoader::new("/nonexistent/path");
        // Should not panic, just return empty
        let discovered = loader.discover();
        assert!(discovered.is_empty());
    }

    #[test]
    fn test_load_nonexistent_plugin() {
        let mut loader = PluginLoader::new("/tmp");
        let result = loader.load("nonexistent-plugin");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HotReloadError::NotFound(_)));
    }

    #[test]
    fn test_unload_after_failed_load() {
        let mut loader = PluginLoader::new("/tmp");
        // Unloading something not loaded is a no-op (doesn't error).
        assert!(loader.unload("foo").is_ok());
    }

    #[test]
    fn test_plugin_info_none() {
        let loader = PluginLoader::new("/tmp");
        assert!(loader.info("not-loaded").is_none());
    }
}
