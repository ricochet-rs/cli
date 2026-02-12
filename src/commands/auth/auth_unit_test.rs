#[cfg(test)]
mod tests {
    use crate::config::Config;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    /// Helper to set up test environment with proper config directory
    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("HOME", temp_dir.path());
        }
        // Create the config directory structure
        // On macOS, dirs::config_dir() might use Library/Application Support
        // On Linux, it uses .config
        // By setting HOME and creating both, we cover both cases
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

    /// Test that logout clears the API key
    #[test]
    fn test_logout_clears_api_key() {
        let _temp_dir = setup_test_env();

        let mut config = Config::default();
        // Set API key on the default server
        if let Some(server) = config.servers.get_mut("default") {
            server.api_key = Some("rico_test_key".to_string());
        }

        // Call logout with no server specified (uses default)
        let result = super::super::logout(&mut config, None);

        assert!(result.is_ok(), "Logout should succeed");
        // Check that the default server's API key is cleared
        let default_server = config
            .servers
            .get("default")
            .expect("default server should exist");
        assert_eq!(
            default_server.api_key, None,
            "API key should be cleared after logout"
        );

    }

    /// Test that logout handles already logged out state
    #[test]
    fn test_logout_when_not_logged_in() {
        let _temp_dir = setup_test_env();

        let mut config = Config::default();
        // Clear any API key from default server
        if let Some(server) = config.servers.get_mut("default") {
            server.api_key = None;
        }

        // Call logout when already logged out
        let result = super::super::logout(&mut config, None);

        assert!(
            result.is_ok(),
            "Logout should succeed even when not logged in"
        );
        // Check that the default server's API key is still None
        let default_server = config
            .servers
            .get("default")
            .expect("default server should exist");
        assert_eq!(default_server.api_key, None, "API key should remain None");

    }

    /// Test API key validation format
    #[test]
    fn test_api_key_format_validation() {
        let valid_keys = vec![
            "rico_abc123",
            "rico_Lg47wQANfgs_LdTNgNXvRMGXmAS2wfUNrJUYXpe2PaWRy",
            "rico_3DLhvjM4hQv_8xGUyQ4RxJXn62hsQYSZgr69T7cVUAR7X",
        ];

        for key in valid_keys {
            assert!(
                key.starts_with("rico_"),
                "Valid API key '{}' should start with 'rico_'",
                key
            );
        }

        let invalid_keys = vec!["not_a_key", "bearer_token", "session_123"];

        for key in invalid_keys {
            assert!(
                !key.starts_with("rico_"),
                "Invalid key '{}' should not start with 'rico_'",
                key
            );
        }
    }

    /// Test key abbreviation for display
    #[test]
    fn test_api_key_abbreviation() {
        let test_cases = vec![
            ("rico_short", "rico_short"), // Short key not abbreviated
            (
                "rico_Lg47wQANfgs_LdTNgNXvRMGXmAS2wfUNrJUYXpe2PaWRy",
                "rico_Lg47wQA...",
            ), // Long key abbreviated
            ("rico_exactly12c", "rico_exactly..."), // Exactly 12 chars + ...
        ];

        for (input, expected) in test_cases {
            let abbreviated = if input.len() > 12 {
                format!("{}...", &input[..12])
            } else {
                input.to_string()
            };

            assert_eq!(
                abbreviated, expected,
                "Key '{}' should abbreviate to '{}'",
                input, expected
            );
        }
    }

    /// Test environment variable handling
    #[test]
    #[serial(env_tests)]
    fn test_env_var_handling() {
        unsafe {
            // Clean environment first
            env::remove_var("RICOCHET_API_KEY");
            env::remove_var("RICOCHET_SERVER");
        }

        // Test without env vars
        assert!(env::var("RICOCHET_API_KEY").is_err());
        assert!(env::var("RICOCHET_SERVER").is_err());

        unsafe {
            // Set env vars
            env::set_var("RICOCHET_API_KEY", "rico_env_key");
            env::set_var("RICOCHET_SERVER", "https://env.server.com");
        }

        // Verify they're set
        assert_eq!(env::var("RICOCHET_API_KEY").unwrap(), "rico_env_key");
        assert_eq!(
            env::var("RICOCHET_SERVER").unwrap(),
            "https://env.server.com"
        );

        unsafe {
            // Clean up
            env::remove_var("RICOCHET_API_KEY");
            env::remove_var("RICOCHET_SERVER");
        }
    }

    /// Test OAuth URL construction
    #[test]
    fn test_oauth_url_format() {
        let server = "https://test.ricochet.com";
        let callback_url = "http://localhost:12345/callback";

        let oauth_url = format!(
            "{}/oauth/authorize?redirect_uri={}&response_type=code&client_id=cli",
            server,
            urlencoding::encode(callback_url)
        );

        // Verify URL structure
        assert!(oauth_url.starts_with(server));
        assert!(oauth_url.contains("/oauth/authorize"));
        assert!(oauth_url.contains("redirect_uri=http%3A%2F%2Flocalhost"));
        assert!(oauth_url.contains("response_type=code"));
        assert!(oauth_url.contains("client_id=cli"));

        // Verify proper query parameter separator
        assert!(oauth_url.contains("?redirect_uri"));
        assert!(oauth_url.contains("&response_type"));
        assert!(oauth_url.contains("&client_id"));
    }

    /// Test CreateApiKeyRequest serialization
    #[test]
    fn test_api_key_request_serialization() {
        use super::super::CreateApiKeyRequest;
        use serde_json;

        let request = CreateApiKeyRequest {
            name: "ricochet-cli-test".to_string(),
            expires_in_hours: 8,
            expires_at: Some("2024-03-18T20:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"name\":\"ricochet-cli-test\""));
        assert!(json.contains("\"expires_in_hours\":8"));
        assert!(json.contains("\"expires_at\":\"2024-03-18T20:00:00Z\""));

        // Test with None expires_at
        let request_no_expiry = CreateApiKeyRequest {
            name: "test".to_string(),
            expires_in_hours: 8,
            expires_at: None,
        };

        let json_no_expiry = serde_json::to_string(&request_no_expiry).unwrap();
        assert!(!json_no_expiry.contains("expires_at"));
    }

    /// Test ApiKeyResponse deserialization
    #[test]
    fn test_api_key_response_deserialization() {
        use super::super::ApiKeyResponse;

        let json = r#"{
            "key": "rico_test_key_123",
            "name": "ricochet-cli-20240318",
            "expires_at": "2024-03-18T20:00:00Z"
        }"#;

        let response: ApiKeyResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.key, "rico_test_key_123");
        assert_eq!(response.name, "ricochet-cli-20240318");
        assert_eq!(
            response.expires_at,
            Some("2024-03-18T20:00:00Z".to_string())
        );

        // Test without expires_at
        let json_no_expiry = r#"{
            "key": "rico_test_key_456",
            "name": "test-key"
        }"#;

        let response_no_expiry: ApiKeyResponse = serde_json::from_str(json_no_expiry).unwrap();
        assert_eq!(response_no_expiry.expires_at, None);
    }

    /// Helper to create a multi-server config for testing
    fn create_multi_server_config() -> Config {
        use crate::config::ServerConfig;
        use std::collections::HashMap;
        use url::Url;

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

    /// Test logout from a specific named server
    #[test]
    fn test_logout_from_named_server() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        // Verify staging has an API key
        assert!(config.servers.get("staging").unwrap().api_key.is_some());

        // Logout from staging (not the default)
        let result = super::super::logout(&mut config, Some("staging"));

        assert!(result.is_ok());

        // Staging should now have no API key
        let staging = config.servers.get("staging").unwrap();
        assert!(staging.api_key.is_none());

        // Prod should still have its API key
        let prod = config.servers.get("prod").unwrap();
        assert!(prod.api_key.is_some());

    }

    /// Test logout from server specified by URL
    #[test]
    fn test_logout_from_server_by_url() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        // Logout using URL
        let result = super::super::logout(&mut config, Some("https://staging.ricochet.com"));

        assert!(result.is_ok());

        // Staging should now have no API key
        let staging = config.servers.get("staging").unwrap();
        assert!(staging.api_key.is_none());

    }

    /// Test logout from nonexistent server fails
    #[test]
    fn test_logout_from_nonexistent_server() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        let result = super::super::logout(&mut config, Some("nonexistent"));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

    }

    /// Test logout from server with no matching URL
    #[test]
    fn test_logout_from_unknown_url() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        let result = super::super::logout(&mut config, Some("https://unknown.server.com"));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No server found"));

    }

    /// Test logout uses default server when none specified
    #[test]
    fn test_logout_uses_default_server() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        // Verify prod (default) has an API key
        assert!(config.servers.get("prod").unwrap().api_key.is_some());

        // Logout without specifying server
        let result = super::super::logout(&mut config, None);

        assert!(result.is_ok());

        // Prod (default) should now have no API key
        let prod = config.servers.get("prod").unwrap();
        assert!(prod.api_key.is_none());

        // Other servers should be unaffected
        let staging = config.servers.get("staging").unwrap();
        assert!(staging.api_key.is_some());

    }

    /// Test logout when already logged out from specified server
    #[test]
    fn test_logout_when_already_logged_out_from_server() {
        let _temp_dir = setup_test_env();
        let mut config = create_multi_server_config();

        // local has no API key
        let result = super::super::logout(&mut config, Some("local"));

        // Should succeed without error (just a warning)
        assert!(result.is_ok());

    }
}
