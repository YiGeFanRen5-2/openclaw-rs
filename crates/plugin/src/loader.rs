use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct PluginLoader;

impl PluginLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load_path(&self, path: impl AsRef<Path>) -> PluginHandle {
        PluginHandle {
            path: path.as_ref().to_path_buf(),
        }
    }
}
