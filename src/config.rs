use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Configuration for a single server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // Legacy fields - kept for backward compatibility during migration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    // Multi-server configuration
    #[serde(default)]
    pub servers: HashMap<String, ServerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_server: Option<String>,

    pub default_format: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut servers = HashMap::new();
        servers.insert(
            "default".to_string(),
            ServerConfig {
                url: Url::parse("http://localhost:3000").unwrap(),
                api_key: None,
            },
        );
        Self {
            server: None,
            api_key: None,
            servers,
            default_server: Some("default".to_string()),
            default_format: Some("table".to_string()),
        }
    }
}

impl Config {
    /// Creates a Config for testing purposes with a single "default" server.
    /// This is useful for testing without needing to construct the full config.
    pub fn for_test(server_url: Url, api_key: Option<String>) -> Self {
        let mut servers = HashMap::new();
        servers.insert(
            "default".to_string(),
            ServerConfig {
                url: server_url,
                api_key,
            },
        );
        Self {
            server: None,
            api_key: None,
            servers,
            default_server: Some("default".to_string()),
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
            let mut config: Config =
                toml::from_str(&content).context("Failed to parse config file")?;

            // Auto-migrate legacy single-server config to multi-server format
            if config.servers.is_empty() && config.server.is_some() {
                config.migrate_v1_config();
                config.save()?;
            }

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Migrate v1 single-server config to multi-server format
    fn migrate_v1_config(&mut self) {
        if let Some(url) = self.server.take() {
            let api_key = self.api_key.take();
            self.servers
                .insert("default".to_string(), ServerConfig { url, api_key });
            self.default_server = Some("default".to_string());
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

    /// Resolve server by name or URL
    /// Priority: 1) RICOCHET_SERVER env var 2) Provided server_ref 3) default_server
    pub fn resolve_server(&self, server_ref: Option<&str>) -> Result<ServerConfig> {
        // Check environment variable first, then provided reference
        let server_str = std::env::var("RICOCHET_SERVER").ok();
        let server_str = server_str.as_deref().or(server_ref);

        let Some(server_str) = server_str else {
            return self.default_server_config();
        };

        // Try as named server first
        if let Some(server_config) = self.servers.get(server_str) {
            return Ok(self.apply_env_key_override(server_config.clone()));
        }

        // Try as direct URL
        if server_str.starts_with("http://") || server_str.starts_with("https://") {
            let url = parse_server_url(server_str)?;
            // For direct URLs, check if we have a matching server config
            for server_config in self.servers.values() {
                if server_config.url == url {
                    return Ok(self.apply_env_key_override(server_config.clone()));
                }
            }
            // No match, return URL with no API key (user will need to login)
            return Ok(ServerConfig { url, api_key: None });
        }

        // Not found
        let available: Vec<&String> = self.servers.keys().collect();
        if available.is_empty() {
            anyhow::bail!(
                "Server '{}' not found. No servers configured. Use 'ricochet servers add' to add one.",
                server_str
            )
        } else {
            anyhow::bail!(
                "Server '{}' not found. Available servers: {}",
                server_str,
                available
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Apply RICOCHET_API_KEY environment variable override if set
    fn apply_env_key_override(&self, mut server_config: ServerConfig) -> ServerConfig {
        if let Ok(api_key_env) = std::env::var("RICOCHET_API_KEY") {
            server_config.api_key = Some(api_key_env);
        }
        server_config
    }

    /// Get the default server configuration
    fn default_server_config(&self) -> Result<ServerConfig> {
        if let Some(default_name) = &self.default_server
            && let Some(server_config) = self.servers.get(default_name)
        {
            return Ok(self.apply_env_key_override(server_config.clone()));
        }

        // Fallback to first available server
        if let Some(server_config) = self.servers.values().next() {
            return Ok(self.apply_env_key_override(server_config.clone()));
        }

        anyhow::bail!("No servers configured. Use 'ricochet servers add' to add a server.")
    }

    /// Add or update a server
    pub fn add_server(&mut self, name: impl Into<String>, url: Url, api_key: Option<String>) {
        let name = name.into();
        self.servers
            .insert(name.clone(), ServerConfig { url, api_key });

        // Set as default if it's the first server
        if self.default_server.is_none() {
            self.default_server = Some(name);
        }
    }

    /// Remove a server. Returns true if this was the default server.
    pub fn remove_server(&mut self, name: &str) -> Result<bool> {
        if !self.servers.contains_key(name) {
            anyhow::bail!("Server '{}' not found", name);
        }

        self.servers.remove(name);

        // Clear default if we removed it (don't auto-select another)
        let was_default = self.default_server.as_deref() == Some(name);
        if was_default {
            self.default_server = None;
        }

        Ok(was_default)
    }

    /// Set the default server
    pub fn set_default_server(&mut self, name: &str) -> Result<()> {
        if !self.servers.contains_key(name) {
            anyhow::bail!("Server '{}' not found", name);
        }
        self.default_server = Some(name.to_string());
        Ok(())
    }

    /// List all configured servers
    pub fn list_servers(&self) -> Vec<(&String, &ServerConfig)> {
        self.servers.iter().collect()
    }

    /// Get the default server name
    pub fn default_server(&self) -> Option<&str> {
        self.default_server.as_deref()
    }

    // Backward compatibility methods

    pub fn server_url(&self) -> Result<Url> {
        Ok(self.resolve_server(None)?.url)
    }

    pub fn api_key(&self) -> Result<String> {
        let server_config = self.resolve_server(None)?;
        server_config
            .api_key
            .context("No API key configured. Use 'ricochet login' to authenticate")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn cleanup_env() {
        unsafe {
            env::remove_var("RICOCHET_SERVER");
            env::remove_var("RICOCHET_API_KEY");
        }
    }

    fn create_test_config() -> Config {
        let mut servers = HashMap::new();
        servers.insert(
            "prod".to_string(),
            ServerConfig {
                url: Url::parse("https://prod.ricochet.com").unwrap(),
                api_key: Some("rico_prod_key".to_string()),
            },
        );
        servers.insert(
            "staging".to_string(),
            ServerConfig {
                url: Url::parse("https://staging.ricochet.com").unwrap(),
                api_key: Some("rico_staging_key".to_string()),
            },
        );
        servers.insert(
            "local".to_string(),
            ServerConfig {
                url: Url::parse("http://localhost:3000").unwrap(),
                api_key: None,
            },
        );
        Config {
            server: None,
            api_key: None,
            servers,
            default_server: Some("prod".to_string()),
            default_format: Some("table".to_string()),
        }
    }

    // ==================== parse_server_url tests ====================

    #[test]
    fn test_parse_server_url_valid_https() {
        let result = parse_server_url("https://example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "https://example.com/");
    }

    #[test]
    fn test_parse_server_url_valid_http() {
        let result = parse_server_url("http://localhost:3000");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "http://localhost:3000/");
    }

    #[test]
    fn test_parse_server_url_missing_scheme() {
        let result = parse_server_url("example.com");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("scheme prefix"));
    }

    #[test]
    fn test_parse_server_url_invalid_url() {
        let result = parse_server_url("http://");
        assert!(result.is_err());
    }

    // ==================== add_server tests ====================

    #[test]
    fn test_add_server_new() {
        cleanup_env();
        let mut config = Config::default();
        let url = Url::parse("https://new.server.com").unwrap();

        config.add_server("new".to_string(), url.clone(), Some("rico_key".to_string()));

        assert!(config.servers.contains_key("new"));
        let server = config.servers.get("new").unwrap();
        assert_eq!(server.url, url);
        assert_eq!(server.api_key, Some("rico_key".to_string()));
    }

    #[test]
    fn test_add_server_sets_default_when_none() {
        cleanup_env();
        let mut config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        let url = Url::parse("https://first.server.com").unwrap();
        config.add_server("first".to_string(), url, None);

        assert_eq!(config.default_server, Some("first".to_string()));
    }

    #[test]
    fn test_add_server_preserves_existing_default() {
        cleanup_env();
        let mut config = create_test_config();
        let original_default = config.default_server.clone();

        let url = Url::parse("https://new.server.com").unwrap();
        config.add_server("new".to_string(), url, None);

        assert_eq!(config.default_server, original_default);
    }

    #[test]
    fn test_add_server_overwrites_existing() {
        cleanup_env();
        let mut config = create_test_config();
        let new_url = Url::parse("https://new-prod.ricochet.com").unwrap();

        config.add_server(
            "prod".to_string(),
            new_url.clone(),
            Some("new_key".to_string()),
        );

        let server = config.servers.get("prod").unwrap();
        assert_eq!(server.url, new_url);
        assert_eq!(server.api_key, Some("new_key".to_string()));
    }

    // ==================== remove_server tests ====================

    #[test]
    fn test_remove_server_success() {
        cleanup_env();
        let mut config = create_test_config();

        let result = config.remove_server("staging");

        assert!(result.is_ok());
        assert!(!result.unwrap()); // was not default
        assert!(!config.servers.contains_key("staging"));
    }

    #[test]
    fn test_remove_server_default() {
        cleanup_env();
        let mut config = create_test_config();

        let result = config.remove_server("prod");

        assert!(result.is_ok());
        assert!(result.unwrap()); // was default
        assert!(!config.servers.contains_key("prod"));
        assert_eq!(config.default_server, None);
    }

    #[test]
    fn test_remove_server_not_found() {
        cleanup_env();
        let mut config = create_test_config();

        let result = config.remove_server("nonexistent");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ==================== set_default_server tests ====================

    #[test]
    fn test_set_default_server_success() {
        cleanup_env();
        let mut config = create_test_config();

        let result = config.set_default_server("staging");

        assert!(result.is_ok());
        assert_eq!(config.default_server, Some("staging".to_string()));
    }

    #[test]
    fn test_set_default_server_not_found() {
        cleanup_env();
        let mut config = create_test_config();

        let result = config.set_default_server("nonexistent");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ==================== list_servers tests ====================

    #[test]
    fn test_list_servers() {
        cleanup_env();
        let config = create_test_config();

        let servers = config.list_servers();

        assert_eq!(servers.len(), 3);
        let names: Vec<&str> = servers.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"prod"));
        assert!(names.contains(&"staging"));
        assert!(names.contains(&"local"));
    }

    #[test]
    fn test_list_servers_empty() {
        cleanup_env();
        let config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        let servers = config.list_servers();

        assert!(servers.is_empty());
    }

    // ==================== default server resolution tests ====================

    #[test]
    fn test_resolve_default_server() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(None);

        assert!(result.is_ok());
        let server = result.unwrap();
        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_prod_key".to_string()));
    }

    #[test]
    fn test_resolve_default_server_fallback_to_first() {
        cleanup_env();
        let mut config = create_test_config();
        config.default_server = None;

        let result = config.resolve_server(None);

        // Should fallback to first available server (order not guaranteed in HashMap)
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_default_server_no_servers() {
        cleanup_env();
        let config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        let result = config.resolve_server(None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No servers configured")
        );
    }

    // ==================== resolve_server tests ====================

    #[test]
    fn test_resolve_server_by_name() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(Some("staging"));

        assert!(result.is_ok());
        let server = result.unwrap();
        assert_eq!(server.url.as_str(), "https://staging.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_staging_key".to_string()));
    }

    #[test]
    fn test_resolve_server_by_url_matching() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(Some("https://prod.ricochet.com"));

        assert!(result.is_ok());
        let server = result.unwrap();
        // Should find matching server and return with API key
        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_prod_key".to_string()));
    }

    #[test]
    fn test_resolve_server_by_url_new() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(Some("https://new.server.com"));

        assert!(result.is_ok());
        let server = result.unwrap();
        assert_eq!(server.url.as_str(), "https://new.server.com/");
        assert_eq!(server.api_key, None); // No matching config
    }

    #[test]
    fn test_resolve_server_default() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(None);

        assert!(result.is_ok());
        let server = result.unwrap();
        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");
    }

    #[test]
    fn test_resolve_server_not_found() {
        cleanup_env();
        let config = create_test_config();

        let result = config.resolve_server(Some("nonexistent"));

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
        assert!(err.contains("Available servers"));
    }

    // Note: env var override tests (RICOCHET_SERVER, RICOCHET_API_KEY) are in
    // tests/servers_test.rs to avoid race conditions between unit and integration tests.

    // ==================== legacy migration tests ====================

    #[test]
    fn test_migrate_legacy_config() {
        cleanup_env();
        let mut config = Config {
            server: Some(Url::parse("https://legacy.server.com").unwrap()),
            api_key: Some("rico_legacy_key".to_string()),
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        config.migrate_v1_config();

        assert!(config.server.is_none());
        assert!(config.api_key.is_none());
        assert!(config.servers.contains_key("default"));
        let server = config.servers.get("default").unwrap();
        assert_eq!(server.url.as_str(), "https://legacy.server.com/");
        assert_eq!(server.api_key, Some("rico_legacy_key".to_string()));
        assert_eq!(config.default_server, Some("default".to_string()));
    }

    #[test]
    fn test_migrate_legacy_config_no_api_key() {
        cleanup_env();
        let mut config = Config {
            server: Some(Url::parse("https://legacy.server.com").unwrap()),
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        config.migrate_v1_config();

        let server = config.servers.get("default").unwrap();
        assert_eq!(server.api_key, None);
    }

    // ==================== backward compatibility tests ====================

    #[test]
    fn test_server_url_compat() {
        cleanup_env();
        let config = create_test_config();

        let result = config.server_url();

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "https://prod.ricochet.com/");
    }

    #[test]
    fn test_api_key_compat() {
        cleanup_env();
        let config = create_test_config();

        let result = config.api_key();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "rico_prod_key");
    }

    #[test]
    fn test_api_key_compat_no_key() {
        cleanup_env();
        let mut config = create_test_config();
        config.default_server = Some("local".to_string()); // local has no api_key

        let result = config.api_key();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No API key configured")
        );
    }

    // ==================== Config::for_test tests ====================

    #[test]
    fn test_config_for_test() {
        cleanup_env();
        let url = Url::parse("https://test.example.com").unwrap();
        let config = Config::for_test(url.clone(), Some("rico_test".to_string()));

        assert!(config.servers.contains_key("default"));
        let server = config.servers.get("default").unwrap();
        assert_eq!(server.url, url);
        assert_eq!(server.api_key, Some("rico_test".to_string()));
        assert_eq!(config.default_server, Some("default".to_string()));
    }

    // ==================== default_server tests ====================

    #[test]
    fn test_default_server() {
        cleanup_env();
        let config = create_test_config();

        assert_eq!(config.default_server(), Some("prod"));
    }

    #[test]
    fn test_default_server_none() {
        cleanup_env();
        let config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        assert_eq!(config.default_server(), None);
    }
}
