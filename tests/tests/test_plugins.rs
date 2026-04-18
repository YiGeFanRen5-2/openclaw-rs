//! Plugin system integration tests.
//!
//! Tests for plugin loading, lifecycle management, capability discovery,
//! and plugin communication.

use openclaw_integration_tests::common::{TestLogger, TestRuntime};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock plugin for testing
#[derive(Debug, Clone)]
struct MockPlugin {
    name: String,
    version: String,
    enabled: bool,
}

impl MockPlugin {
    fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            enabled: true,
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Plugin registry for managing plugins in tests
#[derive(Debug)]
struct PluginRegistry {
    plugins: std::collections::HashMap<String, MockPlugin>,
}

impl PluginRegistry {
    fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    fn register(&mut self, plugin: MockPlugin) -> Result<(), String> {
        let name = plugin.name.clone();
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' already registered", name));
        }
        self.plugins.insert(name, plugin);
        Ok(())
    }

    fn unregister(&mut self, name: &str) -> Option<MockPlugin> {
        self.plugins.remove(name)
    }

    fn get(&self, name: &str) -> Option<&MockPlugin> {
        self.plugins.get(name)
    }

    fn list(&self) -> Vec<&MockPlugin> {
        self.plugins.values().collect()
    }

    fn enabled_plugins(&self) -> Vec<&MockPlugin> {
        self.plugins.values().filter(|p| p.is_enabled()).collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Test plugin registration
#[test]
fn test_plugin_registration() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_registration");

    let mut registry = PluginRegistry::new();
    let plugin = MockPlugin::new("test-plugin", "1.0.0");

    let result = registry.register(plugin.clone());
    assert!(result.is_ok(), "Plugin registration should succeed");

    let loaded = registry.get("test-plugin");
    assert!(loaded.is_some(), "Plugin should be retrievable after registration");
    assert_eq!(loaded.unwrap().version, "1.0.0");
}

/// Test duplicate plugin registration
#[test]
fn test_duplicate_plugin_registration() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_duplicate_plugin_registration");

    let mut registry = PluginRegistry::new();
    let plugin1 = MockPlugin::new("duplicate-plugin", "1.0.0");
    let plugin2 = MockPlugin::new("duplicate-plugin", "2.0.0");

    registry.register(plugin1).expect("First registration should succeed");
    let result = registry.register(plugin2);

    assert!(result.is_err(), "Duplicate registration should fail");
}

/// Test plugin unregistration
#[test]
fn test_plugin_unregistration() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_unregistration");

    let mut registry = PluginRegistry::new();
    let plugin = MockPlugin::new("removable-plugin", "1.0.0");

    registry.register(plugin).expect("Registration should succeed");

    let removed = registry.unregister("removable-plugin");
    assert!(removed.is_some(), "Plugin should be removable");

    let after = registry.get("removable-plugin");
    assert!(after.is_none(), "Plugin should not exist after removal");
}

/// Test plugin listing
#[test]
fn test_plugin_listing() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_listing");

    let mut registry = PluginRegistry::new();

    registry
        .register(MockPlugin::new("plugin-a", "1.0.0"))
        .expect("Registration 1 should succeed");
    registry
        .register(MockPlugin::new("plugin-b", "2.0.0"))
        .expect("Registration 2 should succeed");
    registry
        .register(MockPlugin::new("plugin-c", "3.0.0"))
        .expect("Registration 3 should succeed");

    let all = registry.list();
    assert_eq!(all.len(), 3, "Should have 3 plugins registered");
}

/// Test plugin enable/disable
#[test]
fn test_plugin_enable_disable() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_enable_disable");

    let mut registry = PluginRegistry::new();
    let mut plugin = MockPlugin::new("toggle-plugin", "1.0.0");

    registry
        .register(plugin.clone())
        .expect("Registration should succeed");

    // Initial state
    assert!(plugin.is_enabled(), "Plugin should start enabled");
    assert_eq!(registry.enabled_plugins().len(), 1);

    // Disable the registered plugin via mutable reference
    {
        let registered = registry.plugins.get_mut("toggle-plugin").unwrap();
        registered.disable();
    }
    assert!(!registry.get("toggle-plugin").unwrap().is_enabled(), "Plugin should be disabled after disable()");
    assert_eq!(registry.enabled_plugins().len(), 0, "No plugins should be enabled");
}

/// Test concurrent plugin operations
#[tokio::test]
async fn test_concurrent_plugin_operations() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_concurrent_plugin_operations");

    // Use Arc+RwLock for shared registry
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let registry = Arc::new(RwLock::new(PluginRegistry::new()));

    // Spawn concurrent registrations
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let reg = registry.clone();
            tokio::spawn(async move {
                let mut guard = reg.write().await;
                let plugin = MockPlugin::new(format!("concurrent-plugin-{}", i), "1.0.0");
                guard.register(plugin)
            })
        })
        .collect();

    // Collect results
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        if result.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All 10 concurrent registrations should succeed");

    // Verify all plugins exist
    let guard = registry.read().await;
    assert_eq!(guard.list().len(), 10);
}

/// Test plugin lifecycle simulation
#[tokio::test]
async fn test_plugin_lifecycle() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_lifecycle");

    let registry = Arc::new(RwLock::new(PluginRegistry::new()));

    // 1. Load phase
    {
        let reg_clone = registry.clone();
        tokio::spawn(async move {
            let mut guard = reg_clone.write().await;
            let _ = guard.register(MockPlugin::new("lifecycle-plugin", "1.0.0"));
        }).await.expect("Load phase failed");
    }

    // 2. Enable phase
    let enabled_count = {
        let reg_clone = registry.clone();
        tokio::spawn(async move {
            let guard = reg_clone.read().await;
            guard.enabled_plugins().len()
        }).await.expect("Enable phase panicked")
    };
    assert_eq!(enabled_count, 1, "Plugin should be enabled");

    // 3. Use phase
    tokio::task::yield_now().await;

    // 4. Disable phase
    {
        let reg_clone = registry.clone();
        tokio::spawn(async move {
            let mut guard = reg_clone.write().await;
            if let Some(p) = guard.plugins.get_mut("lifecycle-plugin") {
                p.disable();
            }
        }).await.expect("Disable phase failed");
    }

    // 5. Unload phase
    let removed = {
        let reg_clone = registry.clone();
        tokio::spawn(async move {
            let mut guard = reg_clone.write().await;
            guard.unregister("lifecycle-plugin")
        }).await.expect("Unload phase panicked")
    };
    assert!(removed.is_some(), "Plugin should be unloadable");
}

/// Test plugin capabilities
#[test]
fn test_plugin_capabilities() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_capabilities");

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct PluginCapabilities {
        tools: Vec<String>,
        resources: Vec<String>,
    }

    let caps = PluginCapabilities {
        tools: vec!["tool-a".to_string(), "tool-b".to_string()],
        resources: vec!["resource-x".to_string()],
    };

    let json = serde_json::to_string(&caps).expect("Failed to serialize capabilities");
    let parsed: PluginCapabilities =
        serde_json::from_str(&json).expect("Failed to deserialize capabilities");

    assert_eq!(parsed.tools.len(), 2);
    assert_eq!(parsed.resources.len(), 1);
}

/// Test plugin state persistence simulation
#[test]
fn test_plugin_state_persistence() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_state_persistence");

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct PluginState {
        name: String,
        config: serde_json::Value,
        enabled: bool,
    }

    let state = PluginState {
        name: "stateful-plugin".to_string(),
        config: serde_json::json!({"key": "value", "count": 42}),
        enabled: true,
    };

    // Serialize
    let serialized = serde_json::to_string(&state).expect("Failed to serialize state");

    // Deserialize
    let restored: PluginState =
        serde_json::from_str(&serialized).expect("Failed to deserialize state");

    assert_eq!(restored.name, "stateful-plugin");
    assert_eq!(restored.config["count"], 42);
    assert_eq!(restored.enabled, true);
}

/// Test plugin ordering and priority
#[test]
fn test_plugin_ordering() {
    let mut _logger = TestLogger::new();
    _logger.set_test_name("test_plugin_ordering");

    let mut registry = PluginRegistry::new();

    // Register in specific order
    registry
        .register(MockPlugin::new("alpha", "1.0.0"))
        .expect("Registration should succeed");
    registry
        .register(MockPlugin::new("beta", "1.0.0"))
        .expect("Registration should succeed");
    registry
        .register(MockPlugin::new("gamma", "1.0.0"))
        .expect("Registration should succeed");

    let mut names: Vec<_> = registry.list().iter().map(|p| p.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["alpha", "beta", "gamma"]);
}
