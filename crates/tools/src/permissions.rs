//! Permission types and checks for tool execution.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ReadFile,
    WriteFile,
    ExecuteCommand,
    NetworkAccess,
    SpawnProcess,
    FileSystemAccess(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("permission denied: {0}")]
    Denied(String),
    #[error("permission refused: {0}")]
    Refused(String),
}

pub type PermissionResult<T> = Result<T, PermissionError>;

/// Simple permission set used during tool execution.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    allowed: HashSet<Permission>,
    denied: HashSet<Permission>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow(&mut self, perm: Permission) {
        self.allowed.insert(perm.clone());
        self.denied.remove(&perm);
    }

    pub fn deny(&mut self, perm: Permission) {
        self.denied.insert(perm.clone());
        self.allowed.remove(&perm);
    }

    pub fn is_allowed(&self, perm: &Permission) -> bool {
        self.allowed.contains(perm) && !self.denied.contains(perm)
    }

    pub fn check(&self, perm: &Permission) -> PermissionResult<()> {
        if self.denied.contains(perm) {
            Err(PermissionError::Denied(format!("{:?} is denied", perm)))
        } else if !self.allowed.contains(perm) {
            Err(PermissionError::Refused(format!("{:?} is not explicitly allowed", perm)))
        } else {
            Ok(())
        }
    }
}
