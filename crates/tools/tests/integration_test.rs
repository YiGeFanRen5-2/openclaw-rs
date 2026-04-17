#[cfg(test)]
mod tests {
    use tools::{register_builtin_tools, Tool, ToolRegistry};

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);

        let tools = registry.list_tools();
        println!("Registered tools: {:?}", tools);
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));

        // Test execute list_files
        let tool = registry.get_tool("list_files").unwrap();
        let args = serde_json::json!({
            "path": "/root/.openclaw/workspace",
            "max_depth": 1
        });
        let result = tool.execute(args).unwrap();
        let files = result.get("files").unwrap().as_array().unwrap();
        println!("Found {} files", files.len());
        assert!(files.len() > 0);
    }
}
