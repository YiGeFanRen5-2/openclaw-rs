use serde_json::json;
use tools::{register_builtin_tools, ListFilesTool, Permission, ReadFileTool, Tool, ToolRegistry};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_tool_utf8() {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);
        let tool = registry.get_tool("read_file").unwrap();

        let args = json!({
            "path": "/root/.openclaw/workspace/PHASE6-NODE-BRIDGE-PROGRESS.md",
            "encoding": "utf8",
            "max_size": 10000
        });
        let result = tool.execute(args).unwrap();
        let content = result.get("content").unwrap().as_str().unwrap();
        let size = result.get("size").unwrap().as_u64().unwrap();
        let encoding = result.get("encoding").unwrap().as_str().unwrap();

        assert!(content.len() > 0);
        assert_eq!(encoding, "utf8");
        assert!(size as usize <= 10000);
    }

    #[test]
    fn test_read_file_tool_base64() {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);
        let tool = registry.get_tool("read_file").unwrap();

        let args = json!({
            "path": "/root/.openclaw/workspace/PHASE6-NODE-BRIDGE-PROGRESS.md",
            "encoding": "base64",
            "max_size": 10000
        });
        let result = tool.execute(args).unwrap();
        let content = result.get("content").unwrap().as_str().unwrap();
        let encoding = result.get("encoding").unwrap().as_str().unwrap();

        assert!(content.len() > 0);
        assert_eq!(encoding, "base64");
        // Verify it's valid base64
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let decoded = STANDARD.decode(content).unwrap();
        assert!(decoded.len() > 0);
    }

    #[test]
    fn test_permission_filesystem() {
        let tool = ListFilesTool::new();
        let perm = tool.permission();

        // Test allowlist check for workspace path
        let allowed_path = "/root/.openclaw/workspace";
        assert!(perm.check("read", allowed_path).is_ok());

        // Test deny for path outside allowlist
        let disallowed_path = "/etc/passwd";
        assert!(perm.check("read", disallowed_path).is_err());
    }

    #[test]
    fn test_list_files_tool_basic() {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);
        let tool = registry.get_tool("list_files").unwrap();

        let args = json!({
            "path": "/root/.openclaw/workspace",
            "max_depth": 1,
            "include_hidden": false
        });
        let result = tool.execute(args).unwrap();
        let files = result.get("files").unwrap().as_array().unwrap();
        let total = result.get("total").unwrap().as_i64().unwrap();

        assert_eq!(files.len() as i64, total);
        assert!(files.len() > 0);

        // Verify structure of file entries
        let first = &files[0];
        assert!(first.get("name").is_some());
        assert!(first.get("path").is_some());
        assert!(first.get("is_dir").is_some());
        assert!(first.get("size").is_some());
    }
}
