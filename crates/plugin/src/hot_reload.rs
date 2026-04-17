//! Plugin hot-reload support for OpenClaw
//! 
//! This module provides file watching infrastructure for dynamic plugin development workflows.
//! NOTE: Full hot-reload integration with PluginManager is planned for a future iteration.

use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Watcher configuration
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Directory to watch for plugin changes
    pub watch_dir: String,
    /// Debounce time in milliseconds (default 500ms)
    pub debounce_ms: u64,
    /// Whether to auto-reload on file changes
    pub auto_reload: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            watch_dir: "./plugins".to_string(),
            debounce_ms: 500,
            auto_reload: true,
        }
    }
}

/// Event types watched by hot-reload
#[derive(Debug)]
pub enum WatchEvent {
    FileChanged(String),
    FileCreated(String),
    FileRemoved(String),
}

/// Hot-reload manager that watches plugin files
pub struct HotReloadManager {
    config: HotReloadConfig,
    watcher: Option<RecommendedWatcher>,
    event_tx: Option<Sender<notify::Result<notify::Event>>>,
    event_rx: Option<Receiver<notify::Result<notify::Event>>>,
}

impl HotReloadManager {
    /// Create a new hot-reload manager
    pub fn new(config: HotReloadConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            config,
            watcher: None,
            event_tx: Some(tx),
            event_rx: Some(rx),
        }
    }

    /// Start watching the plugin directory
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.event_tx.take().ok_or_else(|| "event channel already taken")?;

        let config = Config::default().with_poll_interval(Duration::from_millis(self.config.debounce_ms));
        let mut watcher: RecommendedWatcher = Watcher::new(tx, config)?;

        let path = Path::new(&self.config.watch_dir);
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        watcher.watch(path, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);

        // Take the event receiver and spawn event loop
        let rx = self.event_rx.take().ok_or_else(|| "event receiver already taken")?;
        std::thread::spawn(move || {
            Self::event_loop(rx);
        });

        Ok(())
    }

    /// Event loop processes file system events
    fn event_loop(rx: Receiver<notify::Result<notify::Event>>) {
        for event in rx {
            match event {
                Ok(event) => {
                    use notify::event::{EventKind, ModifyKind};
                    match &event.kind {
                        EventKind::Create(_) => {
                            if let Some(path) = event.paths.first() {
                                println!("[hot-reload] file created: {:?}", path);
                            }
                        }
                        EventKind::Modify(ModifyKind::Data(_)) => {
                            if let Some(path) = event.paths.first() {
                                println!("[hot-reload] file modified: {:?}", path);
                            }
                        }
                        EventKind::Remove(_) => {
                            if let Some(path) = event.paths.first() {
                                println!("[hot-reload] file removed: {:?}", path);
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => eprintln!("[hot-reload] watch error: {}", e),
            }
        }
    }

    /// Get a receiver for watch events (for integration with PluginManager later)
    pub fn event_receiver(&self) -> Option<&Receiver<notify::Result<notify::Event>>> {
        self.event_rx.as_ref()
    }

    /// Manually trigger a reload (stub for now)
    pub fn reload_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[hot-reload] manual reload requested (integration pending)");
        Ok(())
    }

    /// Stop the watcher
    pub fn stop(&mut self) {
        self.watcher = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_config_defaults() {
        let config = HotReloadConfig::default();
        assert_eq!(config.watch_dir, "./plugins");
        assert_eq!(config.debounce_ms, 500);
        assert!(config.auto_reload);
    }
}
