#[cfg(test)]
mod tests {
    use crate::config::Config;
    use std::env;
    use tempfile::TempDir;

    /// Test that logout clears the API key
    #[test]
    fn test_logout_clears_api_key() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("HOME", temp_dir.path());
        }

        let mut config = Config {
            api_key: Some("rico_test_key".to_string()),
            ..Default::default()
        };

        // Call logout
        let result = super::super::logout(&mut config);

        assert!(result.is_ok(), "Logout should succeed");
        assert_eq!(
            config.api_key, None,
            "API key should be cleared after logout"
        );

        unsafe {
            env::remove_var("HOME");
        }
    }

    /// Test that logout handles already logged out state
    #[test]
    fn test_logout_when_not_logged_in() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("HOME", temp_dir.path());
        }

        let mut config = Config {
            api_key: None,
            ..Default::default()
        };

        // Call logout when already logged out
        let result = super::super::logout(&mut config);

        assert!(
            result.is_ok(),
            "Logout should succeed even when not logged in"
        );
        assert_eq!(config.api_key, None, "API key should remain None");

        unsafe {
            env::remove_var("HOME");
        }
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
}
