//! # OpenClaw Plugin System
//!
//! Core plugin system with lifecycle management, hook pipeline, and hot reload.
//!
//! ## Hook execution order
//!
//! ```text
//! before_tool_call ──► tool executes ──► after_tool_call
//! before_message    ──► LLM call      ──► after_message
//! ```
//!
//! Failed hooks do NOT block execution (logged + skipped).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::Instant;

// ─── Plugin metadata ────────────────────────────────────────────────────────

/// Plugin manifest, stored as `plugin.json` alongside the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    #[serde(default)]
    pub hooks: Vec<HookPoint>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub resources: Vec<ResourceSpec>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub uri_pattern: String,
    pub name: String,
    pub description: String,
}

/// Built-in permission types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadFiles(String),
    WriteFiles(String),
    Shell,
    Network,
    Session,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::ReadFiles(p) => write!(f, "read_files:{p}"),
            Permission::WriteFiles(p) => write!(f, "write_files:{p}"),
            Permission::Shell => write!(f, "shell"),
            Permission::Network => write!(f, "network"),
            Permission::Session => write!(f, "session"),
        }
    }
}

// ─── Hook types ──────────────────────────────────────────────────────────────

/// All supported hook points.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookPoint {
    BeforeToolCall {
        tool: String,
        input: serde_json::Value,
    },
    AfterToolCall {
        tool: String,
        output: serde_json::Value,
    },
    BeforeMessage {
        role: String,
        content: String,
    },
    AfterMessage {
        role: String,
        content: String,
    },
    OnSessionStart,
    OnSessionEnd,
    OnCompact,
    OnTick {
        interval_ms: u64,
    },
    BeforeProviderCall {
        provider: String,
        model: String,
    },
    AfterProviderCall {
        provider: String,
        model: String,
    },
    OnLoad,
    OnUnload,
}

impl std::fmt::Display for HookPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookPoint::BeforeToolCall { tool, .. } => write!(f, "before_tool_call:{tool}"),
            HookPoint::AfterToolCall { tool, .. } => write!(f, "after_tool_call:{tool}"),
            HookPoint::BeforeMessage { role, .. } => write!(f, "before_message:{role}"),
            HookPoint::AfterMessage { role, .. } => write!(f, "after_message:{role}"),
            HookPoint::OnSessionStart => write!(f, "on_session_start"),
            HookPoint::OnSessionEnd => write!(f, "on_session_end"),
            HookPoint::OnCompact => write!(f, "on_compact"),
            HookPoint::OnTick { .. } => write!(f, "on_tick"),
            HookPoint::BeforeProviderCall { provider, .. } => {
                write!(f, "before_provider_call:{provider}")
            }
            HookPoint::AfterProviderCall { provider, .. } => {
                write!(f, "after_provider_call:{provider}")
            }
            HookPoint::OnLoad => write!(f, "on_load"),
            HookPoint::OnUnload => write!(f, "on_unload"),
        }
    }
}

/// Result of a hook execution.
#[derive(Debug, Clone)]
pub struct HookResult {
    pub modified: bool,
    pub log: Option<String>,
    pub error: Option<String>,
}

impl HookResult {
    pub fn unchanged() -> Self {
        Self {
            modified: false,
            log: None,
            error: None,
        }
    }
    pub fn modified_with(log: impl Into<String>) -> Self {
        Self {
            modified: true,
            log: Some(log.into()),
            error: None,
        }
    }
    pub fn error(e: impl Into<String>) -> Self {
        Self {
            modified: false,
            log: None,
            error: Some(e.into()),
        }
    }
}

// ─── Plugin trait ─────────────────────────────────────────────────────────────

/// The trait that all plugins must implement.
///
/// All methods use #[async_trait] so the trait is dyn-compatible (Send + Sync).
#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    async fn init(&self, config: serde_json::Value) -> PluginResult<()> {
        let _ = config;
        Ok(())
    }

    async fn on_hook(&self, hook: &HookPoint, ctx: &HookContext) -> HookResult;

    fn required_permissions(&self) -> Vec<Permission> {
        vec![]
    }

    async fn shutdown(&self) {}
    async fn reload(&self, _config: serde_json::Value) -> PluginResult<()> {
        Ok(())
    }
}

/// Context passed to every hook invocation.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub session_id: Option<String>,
    pub metadata: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub timestamp: Instant,
}

impl HookContext {
    pub fn new() -> Self {
        Self {
            session_id: None,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            timestamp: Instant::now(),
        }
    }
    pub fn with_session(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            timestamp: Instant::now(),
        }
    }
    pub async fn set_metadata(&self, key: impl Into<String>, value: serde_json::Value) {
        if let Ok(mut map) = self.metadata.write() {
            map.insert(key.into(), value);
        }
    }
}

impl Default for HookContext {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin `{0}` not found")]
    NotFound(String),
    #[error("plugin `{0}` already loaded")]
    AlreadyLoaded(String),
    #[error("manifest not found at `{0}`")]
    ManifestNotFound(String),
    #[error("manifest parse error: {0}")]
    ManifestParse(#[from] serde_json::Error),
    #[error("init failed for `{0}`: {1}")]
    InitFailed(String, String),
    #[error("permission denied: {0} requires {1}")]
    PermissionDenied(String, Permission),
    #[error("hook execution failed: {0}")]
    HookFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PluginResult<T> = std::result::Result<T, PluginError>;

// ─── PluginManager ────────────────────────────────────────────────────────────

/// Central registry and dispatcher for all plugins.
///
/// Cheap to clone (internally Arc'd).
#[derive(Clone)]
pub struct PluginManager {
    inner: Arc<PluginManagerInner>,
}

struct PluginManagerInner {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    manifests: RwLock<HashMap<String, PluginManifest>>,
    permissions: RwLock<HashMap<String, Vec<Permission>>>,
    allowlist: RwLock<Vec<Permission>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PluginManagerInner {
                plugins: RwLock::new(HashMap::new()),
                manifests: RwLock::new(HashMap::new()),
                permissions: RwLock::new(HashMap::new()),
                allowlist: RwLock::new(vec![]),
            }),
        }
    }

    pub fn with_allowlist(allowlist: Vec<Permission>) -> Self {
        Self {
            inner: Arc::new(PluginManagerInner {
                plugins: RwLock::new(HashMap::new()),
                manifests: RwLock::new(HashMap::new()),
                permissions: RwLock::new(HashMap::new()),
                allowlist: RwLock::new(allowlist),
            }),
        }
    }

    /// Discover plugins from a directory (looks for `plugin.json` subdirs).
    pub async fn discover(&self, dir: &Path) -> PluginResult<Vec<PluginManifest>> {
        let mut discovered = Vec::new();
        if !dir.is_dir() {
            return Ok(discovered);
        }
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    match self.load_manifest(&manifest_path).await {
                        Ok(manifest) => {
                            if let Ok(mut manifests) = self.inner.manifests.write() {
                                manifests.insert(manifest.id.clone(), manifest.clone());
                            }
                            discovered.push(manifest);
                        }
                        Err(e) => {
                            tracing::warn!("failed to load manifest at {:?}: {}", manifest_path, e)
                        }
                    }
                }
            }
        }
        Ok(discovered)
    }

    async fn load_manifest(&self, path: &Path) -> PluginResult<PluginManifest> {
        let content = tokio::fs::read_to_string(path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Load and initialize a plugin from manifest + factory.
    pub async fn load<P: Plugin + 'static>(
        &self,
        manifest: &PluginManifest,
        factory: fn() -> P,
    ) -> PluginResult<()> {
        let id = &manifest.id;
        {
            let plugins = self.inner.plugins.read().unwrap();
            if plugins.contains_key(id) {
                return Err(PluginError::AlreadyLoaded(id.clone()));
            }
        }
        let plugin: Arc<dyn Plugin> = Arc::new(factory());
        let required = plugin.required_permissions();
        self.check_permissions(id, &required)?;
        plugin
            .init(serde_json::Value::Null)
            .await
            .map_err(|e| PluginError::InitFailed(id.clone(), e.to_string()))?;
        {
            let mut plugins = self.inner.plugins.write().unwrap();
            plugins.insert(id.clone(), plugin);
        }
        {
            let mut permissions = self.inner.permissions.write().unwrap();
            permissions.insert(id.clone(), required);
        }
        let _ = self.trigger_hook_internal(id, &HookPoint::OnLoad).await;
        tracing::info!(plugin_id = %id, version = %manifest.version, "plugin loaded");
        Ok(())
    }

    /// Unload a plugin.
    pub async fn unload(&self, id: &str) -> PluginResult<()> {
        let _ = self.trigger_hook_internal(id, &HookPoint::OnUnload).await;
        let plugin = {
            let mut plugins = self.inner.plugins.write().unwrap();
            plugins.remove(id)
        };
        match plugin {
            Some(_) => {
                let mut permissions = self.inner.permissions.write().unwrap();
                permissions.remove(id);
                tracing::info!(plugin_id = %id, "plugin unloaded");
                Ok(())
            }
            None => Err(PluginError::NotFound(id.to_string())),
        }
    }

    /// Hot reload a plugin's configuration.
    pub async fn reload(&self, id: &str, config: serde_json::Value) -> PluginResult<()> {
        let plugin = {
            let plugins = self.inner.plugins.read().unwrap();
            plugins.get(id).cloned()
        };
        if let Some(plugin) = plugin {
            plugin
                .reload(config)
                .await
                .map_err(|e| PluginError::InitFailed(id.to_string(), e.to_string()))?;
            tracing::info!(plugin_id = %id, "plugin reloaded");
            Ok(())
        } else {
            Err(PluginError::NotFound(id.to_string()))
        }
    }

    /// Trigger a hook for all subscribed plugins.
    ///
    /// Errors are logged and skipped; does NOT fail the call.
    pub async fn trigger(&self, hook: &HookPoint) -> Vec<(String, HookResult)> {
        // Collect subscribed plugin IDs first (drops lock before async work)
        let subscribed: Vec<String> = {
            let plugins = self.inner.plugins.read().unwrap();
            let manifests = self.inner.manifests.read().unwrap();
            plugins
                .iter()
                .filter(|(id, _)| {
                    manifests
                        .get(id.as_str())
                        .map(|m| m.hooks.contains(hook))
                        .unwrap_or(false)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        // Now call async hook methods without holding locks
        let ctx = HookContext::new();
        let mut results = Vec::new();
        for id in subscribed {
            let plugin = {
                let plugins = self.inner.plugins.read().unwrap();
                plugins.get(&id).cloned()
            };
            if let Some(plugin) = plugin {
                let result = plugin.on_hook(hook, &ctx).await;
                if let Some(e) = &result.error {
                    tracing::warn!(plugin_id = %id, hook = %hook, error = %e, "hook failed, continuing pipeline");
                }
                results.push((id.clone(), result));
            }
        }
        results
    }

    async fn trigger_hook_internal(&self, plugin_id: &str, hook: &HookPoint) -> HookResult {
        let plugin = {
            let plugins = self.inner.plugins.read().unwrap();
            plugins.get(plugin_id).cloned()
        };
        if let Some(plugin) = plugin {
            let ctx = HookContext::new();
            plugin.on_hook(hook, &ctx).await
        } else {
            HookResult::unchanged()
        }
    }

    fn check_permissions(&self, id: &str, required: &[Permission]) -> PluginResult<()> {
        let allowlist = self.inner.allowlist.read().unwrap();
        for perm in required {
            let granted = allowlist.iter().any(|a| match (a, perm) {
                (Permission::ReadFiles(p), Permission::ReadFiles(r)) => {
                    r.starts_with(p) || p.is_empty()
                }
                (Permission::WriteFiles(p), Permission::WriteFiles(r)) => {
                    r.starts_with(p) || p.is_empty()
                }
                (Permission::Shell, Permission::Shell) => true,
                (Permission::Network, Permission::Network) => true,
                (Permission::Session, Permission::Session) => true,
                _ => false,
            });
            if !granted {
                return Err(PluginError::PermissionDenied(id.to_string(), perm.clone()));
            }
        }
        Ok(())
    }

    pub fn grant_permission(&self, plugin_id: &str, perm: Permission) -> PluginResult<()> {
        let mut permissions = self.inner.permissions.write().unwrap();
        permissions
            .entry(plugin_id.to_string())
            .or_default()
            .push(perm);
        Ok(())
    }

    /// List all loaded plugins (id, name, version).
    pub fn list(&self) -> Vec<(String, String, String)> {
        let plugins = self.inner.plugins.read().unwrap();
        plugins
            .iter()
            .map(|(id, p)| (id.clone(), p.name().to_string(), p.version().to_string()))
            .collect()
    }

    pub fn is_loaded(&self, id: &str) -> bool {
        self.inner.plugins.read().unwrap().contains_key(id)
    }

    pub fn manifest(&self, id: &str) -> Option<PluginManifest> {
        self.inner.manifests.read().unwrap().get(id).cloned()
    }

    /// Create a channel pair for external hook dispatch.
    pub fn hook_channel(&self) -> (mpsc::Sender<HookPoint>, mpsc::Receiver<HookPoint>) {
        mpsc::channel(100)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Built-in plugin: Logger ─────────────────────────────────────────────────

/// Built-in logging plugin — records all hook events without modifying data.
#[derive(Debug, Clone)]
pub struct LoggingPlugin {
    pub info: PluginInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

impl LoggingPlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                id: "openclaw.logging".into(),
                name: "OpenClaw Logger".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Built-in plugin that logs all hook events for debugging.".into(),
            },
        }
    }
}

impl Default for LoggingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    fn id(&self) -> &str {
        &self.info.id
    }
    fn name(&self) -> &str {
        &self.info.name
    }
    fn version(&self) -> &str {
        &self.info.version
    }

    async fn init(&self, _config: serde_json::Value) -> PluginResult<()> {
        tracing::info!(plugin = %self.info.id, "logging plugin initialized");
        Ok(())
    }

    async fn on_hook(&self, hook: &HookPoint, ctx: &HookContext) -> HookResult {
        tracing::debug!(hook = %hook, session_id = ?ctx.session_id, "hook fired");
        HookResult::unchanged()
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![]
    }
}

pub mod hot;
pub mod registry;
pub mod telemetry;

pub use registry::{PluginRegistry, PluginRegistryEntry, PluginSource, RegistryError, CompatibilityIssue};
pub use telemetry::MetricsCollector;
