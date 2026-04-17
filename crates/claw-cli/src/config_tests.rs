#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SharedProviderArgs;

    #[test]
    fn test_merge_provider_args_cli_overrides_config() {
        let cli = SharedProviderArgs {
            provider: ProviderMode::Mock,
            api_key: Some("cli-key".to_string()),
            base_url: Some("cli-url".to_string()),
            no_plugin: false,
            plugins: vec!["cli-plugin".to_string()],
            resume: None,
            window_capacity: 5,
            config: None,
        };

        let config = ProviderConfig {
            mode: Some("mock".to_string()),
            api_key: Some("config-key".to_string()),
            base_url: Some("config-url".to_string()),
            no_plugin: Some(true),
            plugins: Some(vec!["config-plugin".to_string()]),
            resume: Some("config-resume".to_string()),
        };

        let merged = merge_provider_args(cli, &config);

        assert_eq!(merged.api_key, Some("cli-key".to_string()));
        assert_eq!(merged.base_url, Some("cli-url".to_string()));
        assert_eq!(merged.no_plugin, false); // CLI false overrides config true
        assert_eq!(merged.plugins, vec!["cli-plugin".to_string()]);
        assert_eq!(merged.resume, None); // CLI none keeps config? Actually CLI none uses config; but our code: if cli.resume.is_none() { config.resume.clone() }
        // So with cli.resume=None, merged.resume should be config.resume
        assert_eq!(merged.resume, Some("config-resume".to_string()));
        assert_eq!(merged.window_capacity, 5);
    }

    #[test]
    fn test_merge_provider_args_config_fills_defaults() {
        let cli = SharedProviderArgs {
            provider: ProviderMode::Mock, // default
            api_key: None,
            base_url: None,
            no_plugin: false,
            plugins: vec!["demo".to_string()], // default
            resume: None,
            window_capacity: 0,
            config: None,
        };

        let config = ProviderConfig {
            mode: Some("openai".to_string()),
            api_key: Some("config-key".to_string()),
            base_url: Some("config-url".to_string()),
            no_plugin: Some(false),
            plugins: Some(vec!["config-plugin".to_string()]),
            resume: Some("config-resume".to_string()),
        };

        let merged = merge_provider_args(cli, &config);

        assert_eq!(merged.provider, ProviderMode::Openai); // config overrides default
        assert_eq!(merged.api_key, Some("config-key".to_string()));
        assert_eq!(merged.base_url, Some("config-url".to_string()));
        assert_eq!(merged.no_plugin, false);
        assert_eq!(merged.plugins, vec!["config-plugin".to_string()]);
        assert_eq!(merged.resume, Some("config-resume".to_string()));
    }

    #[test]
    fn test_config_from_file_invalid_toml() {
        let bad_toml = "this is not toml ]]]";
        let tmp_dir = tempfile::temp_dir().unwrap();
        let path = tmp_dir.join("bad.toml");
        std::fs::write(&path, bad_toml).unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());
        // Could check error kind, but just ensure it fails
    }

    #[test]
    fn test_config_from_file_valid() {
        let good_toml = r#"
[provider]
type = "openai"
api_key = "sk-test"
base_url = "https://api.openai.com/v1"
no_plugin = false
plugins = ["demo"]

[repl]
model = "gpt-4o-mini"
prompt = "hello {{last_model_output}}"
"#;
        let tmp_dir = tempfile::temp_dir().unwrap();
        let path = tmp_dir.join("good.toml");
        std::fs::write(&path, good_toml).unwrap();

        let config = Config::from_file(&path).unwrap();
        assert_eq!(config.provider.mode, Some("openai".to_string()));
        assert_eq!(config.provider.api_key, Some("sk-test".to_string()));
        assert_eq!(config.repl.model, Some("gpt-4o-mini".to_string()));
    }
}
