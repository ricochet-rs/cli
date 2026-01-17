use ricochet_cli::config::{Config, ServerConfig};
use std::collections::HashMap;
use std::env;
use tempfile::TempDir;
use url::Url;

/// Helper to clean up environment variables after tests
fn cleanup_env() {
    unsafe {
        env::remove_var("RICOCHET_API_KEY");
        env::remove_var("RICOCHET_SERVER");
        env::remove_var("HOME");
    }
}

/// Helper to set up test environment with proper config directory
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("HOME", temp_dir.path());
    }
    // Create the config directory structure for both macOS and Linux
    let _ = std::fs::create_dir_all(temp_dir.path().join(".config").join("ricochet"));
    let _ = std::fs::create_dir_all(
        temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("ricochet"),
    );
    temp_dir
}

/// Create a multi-server test config
fn create_multi_server_config() -> Config {
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

#[cfg(test)]
mod servers_tests {
    use super::*;

    // ==================== Config add_server tests ====================

    #[test]
    fn test_add_server_to_empty_config() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        let url = Url::parse("https://new.server.com").unwrap();
        config.add_server("production".to_string(), url.clone(), None);

        // Should be added
        assert!(config.servers.contains_key("production"));
        let server = config.servers.get("production").unwrap();
        assert_eq!(server.url, url);
        assert_eq!(server.api_key, None);

        // Should be set as default since it's the first server
        assert_eq!(config.default_server, Some("production".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_add_server_with_api_key() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = Config::default();
        let url = Url::parse("https://api.server.com").unwrap();

        config.add_server(
            "api".to_string(),
            url.clone(),
            Some("rico_api_key".to_string()),
        );

        let server = config.servers.get("api").unwrap();
        assert_eq!(server.url, url);
        assert_eq!(server.api_key, Some("rico_api_key".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_add_multiple_servers() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        // Add first server
        config.add_server(
            "first".to_string(),
            Url::parse("https://first.com").unwrap(),
            None,
        );

        // Add second server
        config.add_server(
            "second".to_string(),
            Url::parse("https://second.com").unwrap(),
            None,
        );

        // Add third server
        config.add_server(
            "third".to_string(),
            Url::parse("https://third.com").unwrap(),
            None,
        );

        assert_eq!(config.servers.len(), 3);
        // First server should still be default
        assert_eq!(config.default_server, Some("first".to_string()));

        cleanup_env();
    }

    // ==================== Config remove_server tests ====================

    #[test]
    fn test_remove_non_default_server() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();

        let was_default = config.remove_server("staging").unwrap();

        assert!(!was_default);
        assert!(!config.servers.contains_key("staging"));
        assert_eq!(config.servers.len(), 2);
        // Default should still be prod
        assert_eq!(config.default_server, Some("prod".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_remove_default_server_clears_default() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();

        let was_default = config.remove_server("prod").unwrap();

        assert!(was_default);
        assert!(!config.servers.contains_key("prod"));
        // Default should be cleared, not auto-selected
        assert_eq!(config.default_server, None);

        cleanup_env();
    }

    #[test]
    fn test_remove_nonexistent_server_fails() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();

        let result = config.remove_server("nonexistent");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        cleanup_env();
    }

    // ==================== Config set_default_server tests ====================

    #[test]
    fn test_set_default_server() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();
        assert_eq!(config.default_server, Some("prod".to_string()));

        config.set_default_server("staging").unwrap();

        assert_eq!(config.default_server, Some("staging".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_set_default_server_nonexistent_fails() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();

        let result = config.set_default_server("nonexistent");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        cleanup_env();
    }

    // ==================== Config list_servers tests ====================

    #[test]
    fn test_list_servers_returns_all() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let servers = config.list_servers();

        assert_eq!(servers.len(), 3);

        let names: Vec<&str> = servers.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"prod"));
        assert!(names.contains(&"staging"));
        assert!(names.contains(&"local"));

        cleanup_env();
    }

    #[test]
    fn test_list_servers_includes_api_key_status() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let servers = config.list_servers();

        for (name, server_config) in servers {
            match name.as_str() {
                "prod" => assert!(server_config.api_key.is_some()),
                "staging" => assert!(server_config.api_key.is_some()),
                "local" => assert!(server_config.api_key.is_none()),
                _ => panic!("Unexpected server: {}", name),
            }
        }

        cleanup_env();
    }

    // ==================== Config resolve_server tests ====================

    #[test]
    fn test_resolve_server_by_name() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let server = config.resolve_server(Some("staging")).unwrap();

        assert_eq!(server.url.as_str(), "https://staging.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_staging_key".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_resolve_server_by_url() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        // Should find matching server config and return its API key
        let server = config
            .resolve_server(Some("https://prod.ricochet.com"))
            .unwrap();

        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_prod_key".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_resolve_server_by_url_unknown() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        // Unknown URL should return a config with no API key
        let server = config
            .resolve_server(Some("https://unknown.server.com"))
            .unwrap();

        assert_eq!(server.url.as_str(), "https://unknown.server.com/");
        assert_eq!(server.api_key, None);

        cleanup_env();
    }

    #[test]
    fn test_resolve_server_uses_default() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let server = config.resolve_server(None).unwrap();

        // Should use default server (prod)
        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");
        assert_eq!(server.api_key, Some("rico_prod_key".to_string()));

        cleanup_env();
    }

    #[test]
    fn test_resolve_server_env_var_overrides_arg() {
        let config = create_multi_server_config();

        unsafe {
            env::set_var("RICOCHET_SERVER", "local");
        }

        // Even though we pass "prod", env var should win
        let server = config.resolve_server(Some("prod")).unwrap();

        assert_eq!(server.url.as_str(), "http://localhost:3000/");

        cleanup_env();
    }

    #[test]
    fn test_resolve_server_api_key_env_override() {
        let config = create_multi_server_config();

        unsafe {
            env::set_var("RICOCHET_API_KEY", "rico_env_override_key");
        }

        let server = config.resolve_server(Some("prod")).unwrap();

        // Should use env var API key instead of config
        assert_eq!(server.api_key, Some("rico_env_override_key".to_string()));

        cleanup_env();
    }

    // ==================== Config persistence tests ====================

    #[test]
    fn test_config_save_and_load_multi_server() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        // Create and save a multi-server config
        let mut config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        config.add_server(
            "server1".to_string(),
            Url::parse("https://server1.com").unwrap(),
            Some("rico_key1".to_string()),
        );
        config.add_server(
            "server2".to_string(),
            Url::parse("https://server2.com").unwrap(),
            None,
        );
        config.set_default_server("server2").unwrap();

        config.save().unwrap();

        // Load and verify
        let loaded_config = Config::load().unwrap();

        assert_eq!(loaded_config.servers.len(), 2);
        assert!(loaded_config.servers.contains_key("server1"));
        assert!(loaded_config.servers.contains_key("server2"));

        let s1 = loaded_config.servers.get("server1").unwrap();
        assert_eq!(s1.url.as_str(), "https://server1.com/");
        assert_eq!(s1.api_key, Some("rico_key1".to_string()));

        let s2 = loaded_config.servers.get("server2").unwrap();
        assert_eq!(s2.url.as_str(), "https://server2.com/");
        assert_eq!(s2.api_key, None);

        assert_eq!(loaded_config.default_server, Some("server2".to_string()));

        cleanup_env();
    }

    // ==================== get_default_server tests ====================

    #[test]
    fn test_get_default_server() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let server = config.get_default_server().unwrap();

        assert_eq!(server.url.as_str(), "https://prod.ricochet.com/");

        cleanup_env();
    }

    #[test]
    fn test_get_default_server_fallback() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();
        config.default_server = None;

        // Should fallback to first available server
        let result = config.get_default_server();
        assert!(result.is_ok());

        cleanup_env();
    }

    #[test]
    fn test_get_default_server_no_servers() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = Config {
            server: None,
            api_key: None,
            servers: HashMap::new(),
            default_server: None,
            default_format: Some("table".to_string()),
        };

        let result = config.get_default_server();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No servers configured"));

        cleanup_env();
    }

    // ==================== Backward compatibility tests ====================

    #[test]
    fn test_server_url_uses_default() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let url = config.server_url().unwrap();

        assert_eq!(url.as_str(), "https://prod.ricochet.com/");

        cleanup_env();
    }

    #[test]
    fn test_api_key_uses_default() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let key = config.api_key().unwrap();

        assert_eq!(key, "rico_prod_key");

        cleanup_env();
    }

    #[test]
    fn test_api_key_error_when_no_key() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let mut config = create_multi_server_config();
        config.default_server = Some("local".to_string()); // local has no API key

        let result = config.api_key();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No API key configured"));

        cleanup_env();
    }

    #[test]
    fn test_build_authorize_url_for_server() {
        cleanup_env();
        let _temp_dir = setup_test_env();

        let config = create_multi_server_config();

        let url = config
            .build_authorize_url_for_server("http://localhost:12345/callback", Some("staging"))
            .unwrap();

        assert!(url
            .as_str()
            .starts_with("https://staging.ricochet.com/oauth/authorize"));
        assert!(url.as_str().contains("redirect_uri="));
        assert!(url.as_str().contains("response_type=code"));
        assert!(url.as_str().contains("client_id=cli"));

        cleanup_env();
    }
}
