use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub deepseek_api_key: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub deepseek_api_key: String,
}

impl Config {
    /// Load config with priority: env var > config file
    pub fn load() -> Result<Self> {
        // 1. Try environment variable first
        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            if !key.is_empty() {
                return Ok(Config {
                    deepseek_api_key: key,
                });
            }
        }

        // 2. Try config file
        if let Some(path) = Self::config_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config: {}", path.display()))?;
                let file_config: ConfigFile =
                    toml::from_str(&content).with_context(|| "Failed to parse config file")?;

                if let Some(key) = file_config.deepseek_api_key {
                    if !key.is_empty() {
                        return Ok(Config {
                            deepseek_api_key: key,
                        });
                    }
                }
            }
        }

        bail!(
            "DEEPSEEK_API_KEY not found.\n\
             Set it via:\n  \
               export DEEPSEEK_API_KEY=sk-...\n  \
             Or create config file at: {}",
            Self::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/tweet2wx/config.toml".into())
        )
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("tweet2wx").join("config.toml"))
    }
}
