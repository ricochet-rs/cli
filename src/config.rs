use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: Option<String>,
    pub api_key: Option<String>,
    pub default_format: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Some("http://localhost:3000".to_string()),
            api_key: None,
            default_format: Some("table".to_string()),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path.parent().unwrap();

        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        Ok(config_dir.join("ricochet").join("config.toml"))
    }

    pub fn server_url(&self) -> Result<String> {
        self.server
            .clone()
            .or_else(|| std::env::var("RICOCHET_SERVER").ok())
            .context("No server configured. Use 'ricochet login' or set RICOCHET_SERVER")
    }

    pub fn api_key(&self) -> Result<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("RICOCHET_API_KEY").ok())
            .context("No API key configured. Use 'ricochet login' to authenticate")
    }
}
