//! Example 07: Plugin Development
//!
//! This example demonstrates how to create a custom OpenClaw plugin:
//! - Implementing the `Plugin` trait
//! - Registering lifecycle hooks (before/after tool calls, session events)
//! - Sharing state via `HookContext::metadata`
//! - Loading and unloading plugins via `PluginManager`
//!
//! Run with: cargo run --example example_07_plugin_example

use async_trait::async_trait;
use plugins::{
    HookContext, HookPoint, HookResult, Permission, Plugin, PluginManager,
    PluginManifest, PluginResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

// ─── Shared plugin info type ───────────────────────────────────────────────────

/// Plugin metadata shared by both demo plugins.
#[derive(Debug, Clone)]
struct PluginMeta {
    id: &'static str,
    name: &'static str,
    version: &'static str,
}

// ─── Custom Plugin: Metrics Tracker ───────────────────────────────────────────

/// A plugin that tracks all tool calls and session events, collecting metrics.
#[derive(Debug, Clone)]
struct MetricsTrackerPlugin {
    meta: PluginMeta,
    /// Shared metrics (tokio RwLock so guard is Send across await)
    metrics: Arc<RwLock<PluginMetrics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginMetrics {
    pub tool_calls: u64,
    pub messages_in: u64,
    pub messages_out: u64,
    pub sessions_started: u64,
    pub sessions_ended: u64,
}

impl MetricsTrackerPlugin {
    fn new() -> Self {
        Self {
            meta: PluginMeta {
                id: "example.metrics-tracker",
                name: "Metrics Tracker",
                version: "1.0.0",
            },
            metrics: Arc::new(RwLock::new(PluginMetrics::default())),
        }
    }
}

impl Default for MetricsTrackerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MetricsTrackerPlugin {
    fn id(&self) -> &str {
        self.meta.id
    }
    fn name(&self) -> &str {
        self.meta.name
    }
    fn version(&self) -> &str {
        self.meta.version
    }

    async fn init(&self, _config: serde_json::Value) -> PluginResult<()> {
        tracing::info!(plugin = %self.meta.id, "metrics tracker initialized");
        Ok(())
    }

    async fn on_hook(&self, hook: &HookPoint, ctx: &HookContext) -> HookResult {
        // Increment counters first (synchronous, no await)
        {
            let mut m = self.metrics.write().await;
            match hook {
                HookPoint::BeforeToolCall { tool, .. } => {
                    m.tool_calls += 1;
                    tracing::debug!(tool = %tool, session_id = ?ctx.session_id, "tool call started");
                }
                HookPoint::AfterToolCall { tool, .. } => {
                    tracing::debug!(tool = %tool, "tool call completed");
                }
                HookPoint::BeforeMessage { role, content } => {
                    m.messages_in += 1;
                    tracing::debug!(role = %role, content_len = content.len(), "message received");
                }
                HookPoint::AfterMessage { role, content } => {
                    m.messages_out += 1;
                    tracing::debug!(role = %role, content_len = content.len(), "message sent");
                }
                HookPoint::OnSessionStart => {
                    m.sessions_started += 1;
                    tracing::info!(session_id = ?ctx.session_id, "session started");
                }
                HookPoint::OnSessionEnd => {
                    m.sessions_ended += 1;
                    tracing::info!(session_id = ?ctx.session_id, "session ended");
                }
                HookPoint::OnLoad => {
                    tracing::info!(plugin = %self.meta.id, "metrics tracker plugin loaded");
                }
                HookPoint::OnUnload => {
                    tracing::info!(plugin = %self.meta.id, "metrics tracker plugin unloaded");
                }
                _ => {}
            }
        }

        // Now store metadata (await point - guard already dropped)
        if let HookPoint::BeforeToolCall { tool, input } = hook {
            ctx.set_metadata("last_tool", json!({ "name": tool, "input": input })).await;
        }

        HookResult::unchanged()
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::Session]
    }
}

// ─── Custom Plugin: Input Sanitizer ──────────────────────────────────────────

/// A plugin that sanitizes tool input by removing sensitive-looking fields.
/// Demonstrates a modifying "before" hook.
#[derive(Debug, Clone)]
struct SanitizerPlugin {
    meta: PluginMeta,
    /// Fields to redact (simulated — in production load from config)
    sensitive_fields: Vec<&'static str>,
}

impl SanitizerPlugin {
    fn new() -> Self {
        Self {
            meta: PluginMeta {
                id: "example.sanitizer",
                name: "Input Sanitizer",
                version: "1.0.0",
            },
            sensitive_fields: vec!["password", "api_key", "secret", "token"],
        }
    }

    fn sanitize_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut result = serde_json::Map::new();
                for (k, v) in map {
                    if self.sensitive_fields.iter().any(|s| k.to_lowercase().contains(s)) {
                        result.insert(k.clone(), serde_json::Value::String("[REDACTED]".into()));
                    } else {
                        result.insert(k.clone(), self.sanitize_value(v));
                    }
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.sanitize_value(v)).collect())
            }
            _ => value.clone(),
        }
    }
}

impl Default for SanitizerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for SanitizerPlugin {
    fn id(&self) -> &str {
        self.meta.id
    }
    fn name(&self) -> &str {
        self.meta.name
    }
    fn version(&self) -> &str {
        self.meta.version
    }

    async fn init(&self, config: serde_json::Value) -> PluginResult<()> {
        if let Some(fields) = config.get("sensitive_fields").and_then(|v| v.as_array()) {
            tracing::info!(plugin = %self.meta.id, fields_count = fields.len(), "sanitizer configured");
        }
        tracing::info!(plugin = %self.meta.id, "sanitizer plugin initialized");
        Ok(())
    }

    async fn on_hook(&self, hook: &HookPoint, ctx: &HookContext) -> HookResult {
        if let HookPoint::BeforeToolCall { tool: _, input } = hook {
            let sanitized = self.sanitize_value(input);
            let was_modified = sanitized != *input;
            if was_modified {
                ctx.set_metadata("sanitized_input", sanitized).await;
                return HookResult::modified_with("sensitive fields redacted");
            }
        }
        HookResult::unchanged()
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![]
    }
}

// ─── Demo: Build and run the plugin system ────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== OpenClaw Plugin Example ===\n");

    // 1. Create PluginManager with an allowlist
    println!("1. Create PluginManager...");
    let manager = PluginManager::with_allowlist(vec![
        Permission::Session,
        Permission::Network,
        Permission::ReadFiles("/tmp".into()),
    ]);
    println!("   ✓ PluginManager created\n");

    // 2. Create manifests for plugins
    println!("2. Create plugin manifests...");
    let metrics_manifest = PluginManifest {
        id: "example.metrics-tracker".into(),
        name: "Metrics Tracker".into(),
        version: "1.0.0".into(),
        description: "Tracks tool calls and session events".into(),
        author: None,
        hooks: vec![
            HookPoint::BeforeToolCall { tool: String::new(), input: serde_json::Value::Null },
            HookPoint::AfterToolCall { tool: String::new(), output: serde_json::Value::Null },
            HookPoint::BeforeMessage { role: String::new(), content: String::new() },
            HookPoint::AfterMessage { role: String::new(), content: String::new() },
            HookPoint::OnSessionStart,
            HookPoint::OnSessionEnd,
            HookPoint::OnLoad,
            HookPoint::OnUnload,
        ],
        tools: vec![],
        resources: vec![],
        permissions: vec![Permission::Session],
    };

    let sanitizer_manifest = PluginManifest {
        id: "example.sanitizer".into(),
        name: "Input Sanitizer".into(),
        version: "1.0.0".into(),
        description: "Redacts sensitive fields from tool inputs".into(),
        author: None,
        hooks: vec![HookPoint::BeforeToolCall { tool: String::new(), input: serde_json::Value::Null }],
        tools: vec![],
        resources: vec![],
        permissions: vec![],
    };
    println!("   ✓ Manifests created\n");

    // 3. Load the metrics tracker plugin
    println!("3. Load MetricsTracker plugin...");
    manager
        .load(&metrics_manifest, || MetricsTrackerPlugin::new())
        .await?;
    println!("   ✓ MetricsTracker loaded\n");

    // 4. Load the sanitizer plugin
    println!("4. Load Sanitizer plugin...");
    manager
        .load(&sanitizer_manifest, || SanitizerPlugin::new())
        .await?;
    println!("   ✓ Sanitizer loaded\n");

    // 5. List loaded plugins
    println!("5. List loaded plugins...");
    let plugins = manager.list();
    for (id, name, version) in &plugins {
        println!("   - {} v{} ({})", name, version, id);
    }
    println!();

    // 6. Simulate tool call hooks
    println!("6. Simulate BeforeToolCall hook...");
    let hook_before = HookPoint::BeforeToolCall {
        tool: "http_request".into(),
        input: json!({
            "url": "https://api.example.com/data",
            "api_key": "super-secret-key-12345"
        }),
    };
    let results = manager.trigger(&hook_before).await;
    for (plugin_id, result) in &results {
        if result.modified {
            println!("   ✓ {} modified the hook: {:?}", plugin_id, result.log);
        } else if let Some(e) = &result.error {
            println!("   ✗ {} error: {}", plugin_id, e);
        } else {
            println!("   ○ {} observed (no change)", plugin_id);
        }
    }
    println!();

    // 7. Simulate session start
    println!("7. Simulate OnSessionStart hook...");
    let hook_session = HookPoint::OnSessionStart;
    let results = manager.trigger(&hook_session).await;
    for (plugin_id, result) in &results {
        println!("   - {} → {:?}", plugin_id, result.log.as_ref().unwrap_or(&"ok".into()));
    }
    println!();

    // 8. Simulate message hooks
    println!("8. Simulate message hooks...");
    let hook_msg_in = HookPoint::BeforeMessage {
        role: "user".into(),
        content: "Show me the dashboard".into(),
    };
    manager.trigger(&hook_msg_in).await;
    println!("   ✓ BeforeMessage hook fired");

    let hook_msg_out = HookPoint::AfterMessage {
        role: "assistant".into(),
        content: "Here is your dashboard...".into(),
    };
    manager.trigger(&hook_msg_out).await;
    println!("   ✓ AfterMessage hook fired\n");

    // 9. Fire multiple tool calls
    println!("9. Fire 3 more tool calls...");
    for i in 1..=3 {
        let hook = HookPoint::BeforeToolCall {
            tool: format!("tool_{}", i),
            input: serde_json::json!({ "n": i }),
        };
        manager.trigger(&hook).await;
    }
    println!("   ✓ 3 tool calls simulated\n");

    // 10. Hot reload the sanitizer
    println!("10. Hot reload Sanitizer plugin...");
    manager
        .reload(
            "example.sanitizer",
            json!({ "sensitive_fields": ["password", "api_key"] }),
        )
        .await?;
    println!("   ✓ Sanitizer reloaded\n");

    // 11. Unload the sanitizer
    println!("11. Unload Sanitizer plugin...");
    manager.unload("example.sanitizer").await?;
    println!("   ✓ Sanitizer unloaded\n");

    // 12. Final plugin list
    println!("12. Final loaded plugins:");
    for (id, name, version) in &manager.list() {
        println!("   - {} v{} ({})", name, version, id);
    }
    println!();

    // 13. Check manifest
    println!("13. Check plugin manifest:");
    if let Some(manifest) = manager.manifest("example.metrics-tracker") {
        println!("   ✓ ID: {}", manifest.id);
        println!("   ✓ Hooks registered: {}", manifest.hooks.len());
        println!("   ✓ Permissions: {:?}", manifest.permissions);
    }
    println!();

    println!("=== Plugin Example Complete ===");
    println!("Plugins demonstrate: Plugin trait, lifecycle hooks, state sharing,");
    println!("hot reload, permission system, and manifest-based loading.");
    Ok(())
}
