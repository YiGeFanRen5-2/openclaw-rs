//! OpenClaw Plugin System
//!
//! Plugin lifecycle management and hook pipeline.

pub mod plugin;
pub mod hook;
pub mod loader;
pub mod hot_reload;

pub use hook::{
    HookContext, HookStage, ModelHook, ModelHookPayload, PromptHook, PromptHookPayload, ToolHook,
    ToolHookPayload,
};
pub use plugin::{Plugin, PluginMetadata, RuntimePlugin};
pub use hot_reload::{HotReloadConfig, HotReloadManager};
