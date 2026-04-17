//! # Tools Crate (Phase 2 + Phase 3)
//! Tool definitions, first batch, and sandbox framework

use libc::{rlimit, MS_PRIVATE, MS_REC, RLIMIT_AS, RLIMIT_CPU, RLIMIT_FSIZE, RLIMIT_NOFILE};
use nix::sched::unshare;
use nix::sched::CloneFlags;
use nix::sys::wait::WaitStatus;
use nix::unistd::{fork, pipe, ForkResult};
use seccompiler::{apply_filter, BpfProgram, SeccompAction, SeccompFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use thiserror::Error;

/// Tool error
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Timeout: {0} seconds")]
    Timeout(u64),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
    #[error("IO error: {0}")]
    Io(String),
}

/// Permission types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Safe,
    Filesystem {
        allowlist: Vec<String>,
        writable: bool,
    },
    Shell {
        allowlist: Vec<String>,
        arg_pattern: Option<String>,
    },
    Network {
        destinations: Vec<String>,
        protocols: Vec<String>,
        max_connections: usize,
    },
    Custom {
        checker: String,
        config: JsonValue,
    },
}

impl Permission {
    pub fn check(&self, action: &str, target: &str) -> Result<(), ToolError> {
        match self {
            Permission::Safe => Ok(()),
            Permission::Filesystem {
                allowlist,
                writable: _,
            } => {
                let target_canon = match std::fs::canonicalize(target) {
                    Ok(p) => p,
                    Err(_) => PathBuf::from(target),
                };
                for allowed in allowlist {
                    if let Ok(allowed_canon) = std::fs::canonicalize(allowed) {
                        if target_canon.starts_with(allowed_canon) {
                            return Ok(());
                        }
                    } else if target_canon.starts_with(PathBuf::from(allowed)) {
                        return Ok(());
                    }
                }
                Err(ToolError::PermissionDenied(format!(
                    "Path '{}' not in allowlist",
                    target
                )))
            }
            Permission::Shell {
                allowlist,
                arg_pattern: _,
            } => {
                if allowlist.iter().any(|x| x == action) {
                    Ok(())
                } else {
                    Err(ToolError::PermissionDenied(format!(
                        "Command '{}' not allowed",
                        action
                    )))
                }
            }
            Permission::Network {
                destinations,
                protocols: _,
                max_connections: _,
            } => {
                if destinations.iter().any(|x| x == target) {
                    Ok(())
                } else {
                    Err(ToolError::PermissionDenied(format!(
                        "Destination '{}' not allowed",
                        target
                    )))
                }
            }
            Permission::Custom {
                checker: _,
                config: _,
            } => Ok(()),
        }
    }
}

/// Tool trait
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> ToolSchema;
    fn output_schema(&self) -> ToolSchema;
    fn permission(&self) -> Permission;
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError>;
}

/// Tool schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub r#type: String,
    pub description: Option<String>,
    pub properties: Option<serde_json::Value>,
    pub required: Option<Vec<String>>,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub error: Option<String>,
}

impl ToolResult {
    /// Convert to display string (content or error).
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match &self.error {
            Some(err) => format!("Error: {}", err),
            None => self.content.clone(),
        }
    }
}

// ==================== Phase 3: Sandbox ====================

/// Resource limits (ulimit style)
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_seconds: Option<u64>,
    pub max_file_size_mb: Option<u64>,
    pub max_open_files: Option<u64>,
    pub max_processes: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(512),
            max_cpu_seconds: Some(30),
            max_file_size_mb: Some(10),
            max_open_files: Some(1024),
            max_processes: Some(100),
        }
    }
}

/// Network policy
#[derive(Debug, Clone)]
pub enum NetworkPolicy {
    Isolated,
    LoopbackOnly,
    Allowlist(Vec<String>),
    Unrestricted,
}

/// Sandbox
#[derive(Debug, Clone)]
pub struct Sandbox {
    pub root: Option<PathBuf>,
    pub limits: ResourceLimits,
    pub network_policy: NetworkPolicy,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            root: None,
            limits: ResourceLimits::default(),
            network_policy: NetworkPolicy::Isolated,
        }
    }

    /// Execute tool in sandbox (fork + namespaces + rlimit)
    pub fn execute(&self, tool: &dyn Tool, input: JsonValue) -> Result<JsonValue, ToolError> {
        // Create pipe for parent-child communication
        let (parent_pipe, child_pipe) =
            pipe().map_err(|e| ToolError::ExecutionFailed(format!("pipe: {}", e)))?;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                drop(child_pipe);

                let mut buf = Vec::new();
                let mut temp_buf = [0u8; 4096];
                loop {
                    let n = nix::unistd::read(parent_pipe.as_raw_fd(), &mut temp_buf)
                        .map_err(|e| ToolError::ExecutionFailed(format!("read: {}", e)))?;
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&temp_buf[..n]);
                }

                match nix::sys::wait::waitpid(child, None) {
                    Ok(WaitStatus::Exited(_pid, code)) => {
                        if code == 0 {
                            serde_json::from_slice(&buf).map_err(|e| {
                                ToolError::ExecutionFailed(format!("deserialize: {}", e))
                            })
                        } else {
                            Err(ToolError::ExecutionFailed(format!("child exited {}", code)))
                        }
                    }
                    Ok(WaitStatus::Signaled(_pid, sig, _)) => Err(ToolError::ExecutionFailed(
                        format!("child killed by signal {:?}", sig),
                    )),
                    Ok(_) => Err(ToolError::ExecutionFailed("child stopped".into())),
                    Err(e) => Err(ToolError::ExecutionFailed(format!("waitpid error: {}", e))),
                }
            }
            Ok(ForkResult::Child) => {
                drop(parent_pipe);

                Self::apply_rlimits(&self.limits)?;
                let _ = Self::enter_namespaces();
                let _ = Self::install_seccomp();

                let result = tool.execute(input);
                let output = match &result {
                    Ok(v) => serde_json::to_vec(v),
                    Err(e) => serde_json::to_vec(&serde_json::json!({"error": e.to_string()})),
                };

                if let Ok(bytes) = output {
                    let _ = nix::unistd::write(child_pipe, &bytes)
                        .map_err(|e| ToolError::Io(format!("write to pipe: {}", e)))?;
                }

                std::process::exit(match result {
                    Ok(_) => 0,
                    Err(_) => 1,
                });
            }
            Err(_) => Err(ToolError::ExecutionFailed("fork failed".into())),
        }
    }

    fn apply_rlimits(limits: &ResourceLimits) -> Result<(), ToolError> {
        if let Some(max_mem) = limits.max_memory_mb {
            let lim = rlimit {
                rlim_cur: max_mem * 1024 * 1024,
                rlim_max: max_mem * 1024 * 1024,
            };
            unsafe { libc::setrlimit(RLIMIT_AS, &lim) };
            if (unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) }) == -1 {
                // ignore
            }
        }
        if let Some(max_cpu) = limits.max_cpu_seconds {
            let lim = rlimit {
                rlim_cur: max_cpu,
                rlim_max: max_cpu,
            };
            unsafe { libc::setrlimit(RLIMIT_CPU, &lim) };
        }
        if let Some(max_fsize) = limits.max_file_size_mb {
            let lim = rlimit {
                rlim_cur: max_fsize * 1024 * 1024,
                rlim_max: max_fsize * 1024 * 1024,
            };
            unsafe { libc::setrlimit(RLIMIT_FSIZE, &lim) };
        }
        if let Some(max_nofile) = limits.max_open_files {
            let lim = rlimit {
                rlim_cur: max_nofile,
                rlim_max: max_nofile,
            };
            unsafe { libc::setrlimit(RLIMIT_NOFILE, &lim) };
        }
        Ok(())
    }

    fn enter_namespaces() -> Result<(), ToolError> {
        let flags = CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWNET
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWIPC;
        unshare(flags).map_err(|e| ToolError::ExecutionFailed(format!("unshare: {}", e)))?;

        let target_c = CString::new("/").unwrap();
        let ret = unsafe {
            libc::mount(
                std::ptr::null(),
                target_c.as_ptr(),
                std::ptr::null(),
                MS_PRIVATE | MS_REC,
                std::ptr::null(),
            )
        };
        if ret < 0 {
            #[cfg(debug_assertions)]
            eprintln!("[Sandbox] warning: mount(MS_PRIVATE|MS_REC) failed, mount isolation may be incomplete");
        }
        Ok(())
    }

    fn install_seccomp() -> Result<(), ToolError> {
        let rules: BTreeMap<i64, Vec<seccompiler::SeccompRule>> = vec![
            (libc::SYS_ptrace, vec![]),
            (libc::SYS_kexec_load, vec![]),
            (libc::SYS_mount, vec![]),
            (libc::SYS_umount2, vec![]),
            (libc::SYS_pivot_root, vec![]),
            (libc::SYS_unshare, vec![]),
            (libc::SYS_setns, vec![]),
            (libc::SYS_reboot, vec![]),
            (libc::SYS_swapon, vec![]),
            (libc::SYS_swapoff, vec![]),
        ]
        .into_iter()
        .collect();

        let filter = SeccompFilter::new(
            rules,
            SeccompAction::Allow,
            SeccompAction::Errno(libc::EPERM as u32),
            std::env::consts::ARCH
                .try_into()
                .map_err(|e| ToolError::ExecutionFailed(format!("target arch: {:?}", e)))?,
        )
        .map_err(|e| ToolError::ExecutionFailed(format!("seccomp filter: {}", e)))?;

        let bpf: BpfProgram = filter
            .try_into()
            .map_err(|e| ToolError::ExecutionFailed(format!("seccomp bpf: {}", e)))?;

        apply_filter(&bpf)
            .map_err(|e| ToolError::ExecutionFailed(format!("seccomp apply: {}", e)))?;

        Ok(())
    }
}

// ==================== Tools ====================

/// Tool registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool + Send + Sync>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + Send + Sync + 'static>(&mut self, tool: T) {
        let name = tool.name();
        self.tools.insert(name, Box::new(tool));
    }

    pub fn get_tool(&self, name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.get(name).map(|boxed| boxed.as_ref())
    }

    pub fn list_tools(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    pub fn list_schemas(&self) -> Vec<(&'static str, ToolSchema)> {
        self.tools
            .iter()
            .map(|(name, tool)| (*name, tool.input_schema()))
            .collect()
    }
}

/// Register built-in tools
pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    registry.register(ListFilesTool::new());
    registry.register(ReadFileTool::new());
    registry.register(WriteFileTool::new());
    registry.register(EditFileTool::new());
    registry.register(HttpRequestTool::new());
    registry.register(FileInfoTool::new());
    // Multimodal image tools
    registry.register(ImageInfoTool);
    registry.register(ImageFormatsTool);
    // Health check tools
    registry.register(HealthCheckTool);
    registry.register(BatchHealthCheckTool);
    // Validation tools
    registry.register(ValidateJsonTool);
    registry.register(ValidateToolInputTool);
    // JSON Store tools
    registry.register(JsonStoreSetTool);
    registry.register(JsonStoreGetTool);
    registry.register(JsonStoreListTool);
    // Text processing tools
    registry.register(HashTool);
    registry.register(UuidTool);
    registry.register(RandomStringTool);
    registry.register(TextStatsTool);
}

/// List Files Tool
#[derive(Debug, Clone)]
pub struct ListFilesTool;

impl Default for ListFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "list_files"
    }
    fn description(&self) -> &'static str {
        "List directory contents"
    }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("List files in a directory".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string" },
                "include_hidden": { "type": "boolean" },
                "max_depth": { "type": "integer" }
            })),
            required: Some(vec!["path".into()]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("List of files".into()),
            properties: Some(serde_json::json!({
                "files": { "type": "array" },
                "total": { "type": "integer" }
            })),
            required: Some(vec!["files".into(), "total".into()]),
        }
    }
    fn permission(&self) -> Permission {
        Permission::Filesystem {
            allowlist: vec![
                "/home".into(),
                "/tmp".into(),
                "/workspace".into(),
                "/root".into(),
            ],
            writable: false,
        }
    }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'path' parameter".into()))?;
        let include_hidden = input
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = input
            .get("max_depth")
            .and_then(|v| v.as_i64())
            .map(|d| d as usize)
            .unwrap_or(1);
        let path = PathBuf::from(path_str);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path does not exist: {}",
                path_str
            )));
        }
        if !path.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path is not a directory: {}",
                path_str
            )));
        }

        let files = Self::list_dir_recursive(&path, include_hidden, max_depth)?;
        Ok(serde_json::json!({ "files": files, "total": files.len() }))
    }
}

impl ListFilesTool {
    fn list_dir_recursive(
        dir: &PathBuf,
        include_hidden: bool,
        max_depth: usize,
    ) -> Result<Vec<JsonValue>, ToolError> {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let name_str = entry.file_name().to_string_lossy().into_owned();
                if !include_hidden && name_str.starts_with('.') {
                    continue;
                }
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let file_info = serde_json::json!({
                    "name": name_str,
                    "path": entry.path().to_string_lossy().to_string(),
                    "is_dir": is_dir,
                    "size": size,
                });
                entries.push(file_info);
                if is_dir && max_depth > 0 {
                    let subdir_files =
                        Self::list_dir_recursive(&entry.path(), include_hidden, max_depth - 1)?;
                    entries.extend(subdir_files);
                }
            }
        }
        Ok(entries)
    }
}

/// Read File Tool
#[derive(Debug, Clone)]
pub struct ReadFileTool;

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }
    fn description(&self) -> &'static str {
        "Read file content, support utf8 or base64"
    }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Read a file".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string", "description": "File path" },
                "encoding": { "type": "string", "enum": ["utf8", "base64"], "description": "Output encoding" },
                "max_size": { "type": "integer", "description": "Max bytes to read, default 1MB" }
            })),
            required: Some(vec!["path".into()]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("File content".into()),
            properties: Some(serde_json::json!({
                "content": { "type": "string" },
                "size": { "type": "integer" },
                "encoding": { "type": "string" }
            })),
            required: Some(vec!["content".into(), "size".into(), "encoding".into()]),
        }
    }
    fn permission(&self) -> Permission {
        Permission::Filesystem {
            allowlist: vec![
                "/home".into(),
                "/tmp".into(),
                "/workspace".into(),
                "/root".into(),
            ],
            writable: false,
        }
    }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'path' parameter".into()))?;
        let encoding = input
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf8");
        let max_size = input
            .get("max_size")
            .and_then(|v| v.as_i64())
            .map(|s| s as usize)
            .unwrap_or(1024 * 1024);
        let path = PathBuf::from(path_str);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                path_str
            )));
        }
        let metadata = std::fs::metadata(&path)
            .map_err(|e| ToolError::ExecutionFailed(format!("metadata: {}", e)))?;
        if metadata.len() > max_size as u64 {
            return Err(ToolError::ExecutionFailed(format!(
                "File too large: {} > {}",
                metadata.len(),
                max_size
            )));
        }
        let bytes =
            std::fs::read(&path).map_err(|e| ToolError::ExecutionFailed(format!("read: {}", e)))?;

        let (content, actual_encoding) = match encoding {
            "utf8" => {
                let content = String::from_utf8(bytes.clone())
                    .map_err(|e| ToolError::ExecutionFailed(format!("utf8: {}", e)))?;
                (content, "utf8")
            }
            "base64" => {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                let content = STANDARD.encode(&bytes);
                (content, "base64")
            }
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unsupported encoding: {}",
                    encoding
                )))
            }
        };

        Ok(serde_json::json!({
            "content": content,
            "size": bytes.len(),
            "encoding": actual_encoding
        }))
    }
}

// ==================== Phase 11: Additional Tools ====================

/// Write File Tool
#[derive(Debug, Clone)]
pub struct WriteFileTool;

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }
    fn description(&self) -> &'static str {
        "Write file content (utf8 or base64)"
    }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Write a file".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string", "description": "File path" },
                "content": { "type": "string", "description": "File content" },
                "encoding": { "type": "string", "enum": ["utf8", "base64"], "description": "Input encoding" },
                "create_parents": { "type": "boolean", "description": "Create parent directories if missing" },
                "overwrite": { "type": "boolean", "description": "Overwrite if file exists (default true)" }
            })),
            required: Some(vec!["path".into(), "content".into()]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Write result".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string" },
                "bytes_written": { "type": "integer" }
            })),
            required: Some(vec!["path".into(), "bytes_written".into()]),
        }
    }
    fn permission(&self) -> Permission {
        Permission::Filesystem {
            allowlist: vec![
                "/home".into(),
                "/tmp".into(),
                "/workspace".into(),
                "/root/.openclaw/workspace".into(),
            ],
            writable: true,
        }
    }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'path' parameter".into()))?;
        let content_str = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'content' parameter".into()))?;
        let encoding = input
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf8");
        let create_parents = input
            .get("create_parents")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let overwrite = input
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let path = PathBuf::from(path_str);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if create_parents {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ToolError::Io(format!("create_dir_all: {}", e)))?;
                } else {
                    return Err(ToolError::ExecutionFailed(format!(
                        "Parent directory does not exist: {}",
                        parent.to_string_lossy()
                    )));
                }
            }
        }
        if path.exists() && !overwrite {
            return Err(ToolError::ExecutionFailed(format!(
                "File exists and overwrite=false: {}",
                path_str
            )));
        }

        let bytes: Vec<u8> = match encoding {
            "utf8" => content_str.as_bytes().to_vec(),
            "base64" => {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD
                    .decode(content_str)
                    .map_err(|e| ToolError::InvalidInput(format!("base64 decode: {}", e)))?
            }
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unsupported encoding: {}",
                    encoding
                )))
            }
        };

        std::fs::write(&path, &bytes).map_err(|e| ToolError::Io(format!("write: {}", e)))?;

        Ok(serde_json::json!({
            "path": path.to_string_lossy().to_string(),
            "bytes_written": bytes.len()
        }))
    }
}

/// Edit File Tool (exact replace)
#[derive(Debug, Clone)]
pub struct EditFileTool;

impl Default for EditFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }
    fn description(&self) -> &'static str {
        "Edit a UTF-8 text file by exact string replacement"
    }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Edit a file (exact replace)".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string" },
                "old": { "type": "string" },
                "new": { "type": "string" }
            })),
            required: Some(vec!["path".into(), "old".into(), "new".into()]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Edit result".into()),
            properties: Some(serde_json::json!({
                "path": { "type": "string" },
                "replaced": { "type": "boolean" },
                "occurrences": { "type": "integer" }
            })),
            required: Some(vec!["path".into(), "replaced".into(), "occurrences".into()]),
        }
    }
    fn permission(&self) -> Permission {
        Permission::Filesystem {
            allowlist: vec![
                "/root/.openclaw/workspace".into(),
                "/tmp".into(),
                "/workspace".into(),
            ],
            writable: true,
        }
    }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'path' parameter".into()))?;
        let old_str = input
            .get("old")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'old' parameter".into()))?;
        let new_str = input
            .get("new")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'new' parameter".into()))?;

        let path = PathBuf::from(path_str);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ToolError::Io(format!("read_to_string: {}", e)))?;

        let occurrences = content.matches(old_str).count() as i64;
        if occurrences == 0 {
            return Ok(serde_json::json!({
                "path": path.to_string_lossy().to_string(),
                "replaced": false,
                "occurrences": 0
            }));
        }

        let updated = content.replace(old_str, new_str);
        std::fs::write(&path, updated.as_bytes())
            .map_err(|e| ToolError::Io(format!("write: {}", e)))?;

        Ok(serde_json::json!({
            "path": path.to_string_lossy().to_string(),
            "replaced": true,
            "occurrences": occurrences
        }))
    }
}

/// HTTP Request Tool
#[derive(Debug, Clone)]
pub struct HttpRequestTool;

impl Default for HttpRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRequestTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for HttpRequestTool {
    fn name(&self) -> &'static str {
        "http_request"
    }
    fn description(&self) -> &'static str {
        "Make an outbound HTTP request (GET/POST/etc.)"
    }
    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("HTTP request".into()),
            properties: Some(serde_json::json!({
                "method": { "type": "string" },
                "url": { "type": "string" },
                "headers": { "type": "object", "additionalProperties": {"type":"string"} },
                "body": { "type": ["string", "object", "null"] },
                "timeout_seconds": { "type": "integer" }
            })),
            required: Some(vec!["method".into(), "url".into()]),
        }
    }
    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("HTTP response".into()),
            properties: Some(serde_json::json!({
                "status": { "type": "integer" },
                "headers": { "type": "object" },
                "body": { "type": "string" }
            })),
            required: Some(vec!["status".into(), "headers".into(), "body".into()]),
        }
    }
    fn permission(&self) -> Permission {
        Permission::Network {
            destinations: vec!["*".into()],
            protocols: vec!["https".into(), "http".into()],
            max_connections: 10,
        }
    }
    fn execute(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
        use std::time::Duration;

        let method = input
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'method' parameter".into()))?
            .to_uppercase();
        let url_str = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'url' parameter".into()))?;

        let parsed = url::Url::parse(url_str)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid url: {}", e)))?;
        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(ToolError::InvalidInput(format!(
                "Unsupported url scheme: {}",
                scheme
            )));
        }

        let timeout_seconds = input
            .get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| ToolError::ExecutionFailed(format!("reqwest build: {}", e)))?;

        let mut req = match method.as_str() {
            "GET" => client.get(parsed.clone()),
            "POST" => client.post(parsed.clone()),
            "PUT" => client.put(parsed.clone()),
            "PATCH" => client.patch(parsed.clone()),
            "DELETE" => client.delete(parsed.clone()),
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unsupported method: {}",
                    method
                )))
            }
        };

        if let Some(hdrs) = input.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in hdrs {
                if let Some(vs) = v.as_str() {
                    req = req.header(k, vs);
                }
            }
        }

        if let Some(body) = input.get("body") {
            if body.is_null() {
                // no-op
            } else if let Some(s) = body.as_str() {
                req = req.body(s.to_string());
            } else {
                req = req.json(body);
            }
        }

        let resp = req
            .send()
            .map_err(|e| ToolError::ExecutionFailed(format!("request: {}", e)))?;

        let status = resp.status().as_u16() as i64;
        let mut headers_json = serde_json::Map::new();
        for (k, v) in resp.headers().iter() {
            headers_json.insert(
                k.as_str().to_string(),
                serde_json::Value::String(v.to_str().unwrap_or("").to_string()),
            );
        }
        let body_text = resp
            .text()
            .map_err(|e| ToolError::ExecutionFailed(format!("read body: {}", e)))?;

        Ok(serde_json::json!({
            "status": status,
            "headers": headers_json,
            "body": body_text
        }))
    }
}

// ==================== File Info Tool ====================
mod file_info;
pub use file_info::FileInfoTool;

// ==================== Image Tools (Multimodal) ====================
mod image_tools;
pub use image_tools::{ImageInfoTool, ImageFormatsTool};

// ==================== Health Check Tools ====================
mod health_tools;
pub use health_tools::{HealthCheckTool, BatchHealthCheckTool};

// ==================== Validation Tools ====================
mod validator;
pub use validator::{ValidateJsonTool, ValidateToolInputTool};

// ==================== JSON Store Tools ====================
mod json_store;
pub use json_store::{JsonStoreSetTool, JsonStoreGetTool, JsonStoreListTool};

// ==================== Text Processing Tools ====================
mod text_tools;
pub use text_tools::{HashTool, UuidTool, RandomStringTool, TextStatsTool};

// ==================== Phase 11: Benchmarks & Tests ====================

#[cfg(test)]
mod bench_tools {
    use super::*;

    /// A minimal concrete tool for testing/benchmarking
    struct BenchTool;

    impl Tool for BenchTool {
        fn name(&self) -> &'static str {
            "bench_tool"
        }
        fn description(&self) -> &'static str {
            "A tool for benchmarking"
        }
        fn input_schema(&self) -> ToolSchema {
            ToolSchema {
                r#type: "object".into(),
                description: Some("Bench input".into()),
                properties: Some(serde_json::json!({})),
                required: Some(vec![]),
            }
        }
        fn output_schema(&self) -> ToolSchema {
            ToolSchema {
                r#type: "object".into(),
                description: Some("Bench output".into()),
                properties: Some(serde_json::json!({})),
                required: Some(vec![]),
            }
        }
        fn permission(&self) -> Permission {
            Permission::Safe
        }
        fn execute(&self, _input: JsonValue) -> Result<JsonValue, ToolError> {
            Ok(serde_json::json!({ "ok": true }))
        }
    }

    #[test]
    fn bench_tool_registry_insert() {
        let mut registry = ToolRegistry::new();
        assert_eq!(registry.list_tools().len(), 0);
        registry.register(BenchTool);
        assert_eq!(registry.list_tools().len(), 1);
        assert_eq!(registry.list_tools()[0], "bench_tool");
    }

    #[test]
    fn bench_tool_registry_get() {
        let mut registry = ToolRegistry::new();
        registry.register(BenchTool);
        let tool = registry.get_tool("bench_tool");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "bench_tool");
    }

    #[test]
    fn bench_tool_registry_list_schemas() {
        let mut registry = ToolRegistry::new();
        registry.register(BenchTool);
        let schemas = registry.list_schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].0, "bench_tool");
    }

    #[test]
    fn bench_tool_execution() {
        let tool = BenchTool;
        let result = tool.execute(serde_json::json!({}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({ "ok": true }));
    }

    #[test]
    fn bench_tool_permission_safe() {
        let tool = BenchTool;
        assert!(tool.permission().check("anything", "anywhere").is_ok());
    }
}
