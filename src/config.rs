use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

/// Parse a server URL with validation that it includes the http:// or https:// scheme
pub fn parse_server_url(url_str: &str) -> Result<Url> {
    if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
        anyhow::bail!(
            "Server URL must include the scheme prefix (http:// or https://). Got: '{}'",
            url_str
        )
    }
    Url::parse(url_str).context("Invalid server URL format")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: Url,
    pub api_key: Option<String>,
    pub default_format: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Url::parse("http://localhost:3000").unwrap(),
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

    pub fn server_url(&self) -> Result<Url> {
        if let Ok(server_env) = std::env::var("RICOCHET_SERVER") {
            parse_server_url(&server_env)
                .context("Invalid URL in RICOCHET_SERVER environment variable")
        } else {
            Ok(self.server.clone())
        }
    }

    pub fn api_key(&self) -> Result<String> {
        std::env::var("RICOCHET_API_KEY")
            .ok()
            .or_else(|| self.api_key.clone())
            .context("No API key configured. Use 'ricochet login' to authenticate")
    }

    pub fn build_authorize_url(&self, callback_url: &str) -> Result<Url> {
        let mut oauth_url = self.server_url()?;
        oauth_url.set_path("/oauth/authorize");
        oauth_url
            .query_pairs_mut()
            .append_pair("redirect_uri", callback_url)
            .append_pair("response_type", "code")
            .append_pair("client_id", "cli");
        Ok(oauth_url)
    }
}
