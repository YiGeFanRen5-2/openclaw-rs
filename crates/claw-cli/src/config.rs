use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub repl: ReplConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub mode: Option<String>, // "mock" or "openai"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub no_plugin: Option<bool>,
    pub plugins: Option<Vec<String>>,
    pub resume: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplConfig {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub plugins: Option<Vec<String>>,
}

impl Config {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path.into())?;
        let cfg: Config = toml::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(cfg)
    }
}

/// Convert ProviderConfig mode string to ProviderMode enum
fn mode_from_str(s: &str) -> Option<super::cli::ProviderMode> {
    match s.to_ascii_lowercase().as_str() {
        "mock" => Some(super::cli::ProviderMode::Mock),
        "openai" => Some(super::cli::ProviderMode::Openai),
        _ => None,
    }
}

/// Merge CLI args with config file values.
/// Priority: CLI (explicit) > Config > Defaults.
pub fn merge_provider_args(
    cli: super::cli::SharedProviderArgs,
    config: &ProviderConfig,
) -> super::cli::SharedProviderArgs {
    // Determine provider: if CLI uses default (Mock) and config specifies a mode, use config; otherwise keep CLI
    let default_provider = super::cli::ProviderMode::Mock;
    let provider = if cli.provider == default_provider {
        if let Some(mode_str) = &config.mode {
            mode_from_str(mode_str).unwrap_or(default_provider)
        } else {
            cli.provider
        }
    } else {
        cli.provider
    };

    let api_key = if cli.api_key.is_none() { config.api_key.clone() } else { cli.api_key };
    let base_url = if cli.base_url.is_none() { config.base_url.clone() } else { cli.base_url };

    // no_plugin: if CLI is false and config provides Some, use config; otherwise keep CLI
    let no_plugin = if !cli.no_plugin && config.no_plugin.is_some() {
        config.no_plugin.unwrap()
    } else {
        cli.no_plugin
    };

    // plugins: if CLI is default ["demo"] and config provides some, use config; else keep CLI
    let default_plugins = vec!["demo".to_string()];
    let plugins = if cli.plugins == default_plugins {
        if let Some(ref p) = config.plugins {
            p.clone()
        } else {
            default_plugins
        }
    } else {
        cli.plugins.clone()
    };

    let resume = if cli.resume.is_none() { config.resume.clone() } else { cli.resume };

    super::cli::SharedProviderArgs {
        provider,
        api_key,
        base_url,
        no_plugin,
        plugins,
        resume,
        window_capacity: cli.window_capacity,
        config: cli.config,
    }
}

/// Merge ReplArgs with config
pub fn merge_repl_args(
    mut cli: super::cli::ReplArgs,
    config: &ReplConfig,
) -> super::cli::ReplArgs {
    // If model/prompt are still at their CLI defaults, allow config to override
    let default_model = "mock-1";
    let default_prompt = "summarize previous tool output: {{last_tool_output}}";

    if cli.model == default_model && config.model.is_some() {
        cli.model = config.model.clone().unwrap();
    }
    if cli.prompt == default_prompt && config.prompt.is_some() {
        cli.prompt = config.prompt.clone().unwrap();
    }
    // provider.plugins already merged at provider level
    cli
}
