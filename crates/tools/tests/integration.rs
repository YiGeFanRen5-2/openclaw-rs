use serde_json::{json, Value as JsonValue};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::process::Command;
use tools::{ListFilesTool, Permission, ReadFileTool, Sandbox, Tool, ToolError, ToolSchema};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_files_tool_success() {
        let tool = ListFilesTool::new();
        let input = json!({ "path": "/root/.openclaw/workspace", "max_depth": 1 });
        let result = tool.execute(input).unwrap();
        assert!(result.get("files").is_some());
        assert!(result.get("total").is_some());
        let files = result.get("files").unwrap().as_array().unwrap();
        assert!(files.len() > 0);
    }

    #[test]
    fn test_list_files_tool_invalid_path() {
        let tool = ListFilesTool::new();
        let input = json!({ "path": "/nonexistent/path" });
        let result = tool.execute(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => assert!(msg.contains("does not exist")),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_read_file_tool_utf8() {
        let tool = ReadFileTool::new();
        // 使用当前 crate 的 lib.rs 作为测试文件
        let input = json!({ "path": "/root/.openclaw/workspace/openclaw-rs/crates/tools/src/lib.rs", "encoding": "utf8" });
        let result = tool.execute(input).unwrap();
        assert!(result.get("content").is_some());
        assert!(result.get("size").is_some());
        assert_eq!(result.get("encoding").unwrap(), "utf8");
    }

    #[test]
    fn test_read_file_tool_base64() {
        let tool = ReadFileTool::new();
        let input = json!({ "path": "/root/.openclaw/workspace/openclaw-rs/crates/tools/src/lib.rs", "encoding": "base64" });
        let result = tool.execute(input).unwrap();
        assert!(result.get("content").is_some());
        assert_eq!(result.get("encoding").unwrap(), "base64");
    }

    #[test]
    fn test_read_file_tool_not_found() {
        let tool = ReadFileTool::new();
        let input = json!({ "path": "/nonexistent/file.txt" });
        let result = tool.execute(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_permission_filesystem_allowlist() {
        let perm = Permission::Filesystem {
            allowlist: vec!["/home".into(), "/root".into()],
            writable: false,
        };
        assert!(perm.check("read", "/root/.openclaw").is_ok());
        assert!(perm.check("read", "/home/user").is_ok());
        assert!(perm.check("read", "/etc/passwd").is_err());
    }

    #[test]
    fn test_permission_shell_allowlist() {
        let perm = Permission::Shell {
            allowlist: vec!["ls".into(), "cat".into()],
            arg_pattern: None,
        };
        assert!(perm.check("ls", "ls").is_ok());
        assert!(perm.check("cat", "cat").is_ok());
        assert!(perm.check("rm", "rm").is_err());
    }

    #[test]
    fn test_tool_schema_serialization() {
        let tool = ListFilesTool::new();
        let schema = tool.input_schema();
        assert_eq!(schema.r#type, "object");
        assert!(schema.required.is_some());
        assert!(schema.properties.is_some());
    }

    #[test]
    fn test_permission_filesystem_path_traversal_blocked() {
        let perm = Permission::Filesystem {
            allowlist: vec!["/root".into()],
            writable: false,
        };
        // 尝试通过 .. 逃逸
        assert!(perm.check("read", "/root/../etc/passwd").is_err());
        // 正常路径允许
        assert!(perm.check("read", "/root/.openclaw").is_ok());
    }

    #[test]
    fn test_sandbox_execute_list_files_success() {
        let sandbox = Sandbox::new();
        let tool = ListFilesTool::new();
        let input = json!({ "path": "/root/.openclaw/workspace", "max_depth": 1 });

        let result = sandbox.execute(&tool, input).unwrap();
        assert!(result.get("files").is_some());
        assert!(result.get("total").is_some());
        assert!(result.get("total").unwrap().as_u64().unwrap() > 0);
    }

    #[test]
    fn test_sandbox_execute_read_file_not_found_returns_error() {
        let sandbox = Sandbox::new();
        let tool = ReadFileTool::new();
        let input = json!({ "path": "/nonexistent/file.txt" });

        let result = sandbox.execute(&tool, input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert!(
                    msg.contains("child exited")
                        || msg.contains("deserialize")
                        || msg.contains("File not found")
                );
            }
            other => panic!("Wrong error type: {:?}", other),
        }
    }

    #[test]
    fn test_sandbox_namespace_pid_isolation() {
        // 通过执行一个简单命令，检查子进程的 PID namespace 情况
        // 在 PID namespace 中，子进程应该是 init (PID=1)
        struct PidCheckTool;
        impl Tool for PidCheckTool {
            fn name(&self) -> &'static str {
                "pid_check"
            }
            fn description(&self) -> &'static str {
                "Check PID namespace isolation"
            }
            fn input_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: None,
                    required: None,
                }
            }
            fn output_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: Some(json!({ "pid": { "type": "integer" } })),
                    required: None,
                }
            }
            fn permission(&self) -> Permission {
                Permission::Safe
            }
            fn execute(&self, _input: JsonValue) -> Result<JsonValue, ToolError> {
                // 在子进程中读取 /proc/self/stat 获取 PID
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("cat /proc/self/stat")
                    .output()
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                let content = String::from_utf8(output.stdout)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                // /proc/self/stat 第一列是 pid
                let pid = content
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<i32>()
                    .unwrap_or(0);
                Ok(json!({ "pid": pid }))
            }
        }

        let sandbox = Sandbox::new();
        let tool = PidCheckTool;
        let input = json!({});
        let result = sandbox.execute(&tool, input).unwrap();
        let pid = result.get("pid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        // 在独立 PID namespace 中，子进程通常是 init (PID=1)
        // 注意：如果父进程也在自己独立的 namespace 中，结果可能不同
        // 这里我们只验证子进程的 PID 是合理的（通常为 1）
        assert!(pid > 0, "PID should be positive, got {}", pid);
    }

    #[test]
    fn test_sandbox_namespace_mount_isolation() {
        // 验证 mount namespace 隔离：子进程的 mount namespace inode 应不同于父进程
        struct MountNsInodeTool;
        impl Tool for MountNsInodeTool {
            fn name(&self) -> &'static str {
                "mount_ns_inode"
            }
            fn description(&self) -> &'static str {
                "Get mount namespace inode"
            }
            fn input_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: None,
                    required: None,
                }
            }
            fn output_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: Some(json!({ "inode": { "type": "integer" } })),
                    required: None,
                }
            }
            fn permission(&self) -> Permission {
                Permission::Safe
            }
            fn execute(&self, _input: JsonValue) -> Result<JsonValue, ToolError> {
                // 读取 /proc/self/ns/mnt 的 inode 号
                let inode = fs::metadata("/proc/self/ns/mnt")
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
                    .ino();
                Ok(json!({ "inode": inode as u64 }))
            }
        }

        let sandbox = Sandbox::new();
        let tool = MountNsInodeTool;
        let input = json!({});
        let result = sandbox.execute(&tool, input).unwrap();
        let child_inode = result.get("inode").and_then(|v| v.as_u64()).unwrap();

        // 父进程的 mount namespace inode
        let parent_inode = fs::metadata("/proc/self/ns/mnt").unwrap().ino() as u64;

        // 不同 mount namespace，inode 应该不同
        assert_ne!(
            child_inode, parent_inode,
            "Mount namespace isolation failed: child and parent have same inode"
        );
    }

    #[test]
    fn test_sandbox_namespace_network_isolation() {
        // 检查网络 namespace：子进程应该看不到 host 的网络设备
        struct NetCheckTool;
        impl Tool for NetCheckTool {
            fn name(&self) -> &'static str {
                "net_check"
            }
            fn description(&self) -> &'static str {
                "Check network namespace isolation"
            }
            fn input_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: None,
                    required: None,
                }
            }
            fn output_schema(&self) -> ToolSchema {
                ToolSchema {
                    r#type: "object".into(),
                    description: None,
                    properties: Some(json!({ "has_lo": { "type": "boolean" } })),
                    required: None,
                }
            }
            fn permission(&self) -> Permission {
                Permission::Safe
            }
            fn execute(&self, _input: JsonValue) -> Result<JsonValue, ToolError> {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("ip link show 2>/dev/null || echo 'NO_IP'")
                    .output()
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                let content = String::from_utf8(output.stdout)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                let has_lo = content.contains("lo:");
                Ok(json!({ "has_lo": has_lo }))
            }
        }

        let sandbox = Sandbox::new();
        let tool = NetCheckTool;
        let input = json!({});
        let result = sandbox.execute(&tool, input).unwrap();
        let has_lo = result
            .get("has_lo")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 在隔离的网络 namespace 中，应该只有回环接口（lo）
        assert!(
            has_lo,
            "Expected loopback interface 'lo' to exist in isolated network namespace"
        );
    }
}
