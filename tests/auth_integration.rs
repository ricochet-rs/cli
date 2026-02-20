use mockito::Server;
use ricochet_cli::config::Config;
use std::env;
use tempfile::TempDir;
use url::Url;

/// Helper to create a test config with a temporary directory
fn create_test_config() -> (Config, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::default();
    (config, temp_dir)
}

/// Helper to clean up environment variables after tests
fn cleanup_env() {
    unsafe {
        env::remove_var("RICOCHET_API_KEY");
        env::remove_var("RICOCHET_SERVER");
    }
}

#[tokio::test]
async fn test_client_creation_with_valid_key() {
    cleanup_env();
    let mut server = Server::new_async().await;
    let mock_url = server.url();

    // Mock the validation endpoint
    let _mock = server
        .mock("GET", "/api/v0/validate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"valid": true}"#)
        .create_async()
        .await;

    // Create client with valid key
    let client = ricochet_cli::client::RicochetClient::new_with_key(
        mock_url.clone(),
        "rico_test_key_123".to_string(),
    );

    assert!(client.is_ok(), "Client creation should succeed");

    let client = client.unwrap();
    let result = client.validate_key().await;
    assert!(result.is_ok(), "Validation request should succeed");

    cleanup_env();
}

#[tokio::test]
async fn test_client_with_invalid_server() -> anyhow::Result<()> {
    cleanup_env();

    // Client creation doesn't validate URL format upfront
    // It will fail when trying to make requests
    let client = ricochet_cli::client::RicochetClient::new_with_key(
        "not-a-valid-url".to_string(),
        "rico_test_key_123".to_string(),
    );

    // this should cause an error because the url is invalid
    assert!(
        client.is_err(),
        "Client creation succeeds even with invalid URL"
    );

    // But validation should fail when trying to use a valid client
    // with an invalid key
    let mut server = Server::new_async().await;

    // Mock the check_key endpoint to return 401 Unauthorized (invalid key)
    let _mock = server
        .mock("GET", "/api/v0/check_key")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Invalid API key"}"#)
        .create_async()
        .await;

    // this tests with a valid url from the mock server
    let client = ricochet_cli::client::RicochetClient::new_with_key(
        server.url(),
        "invalid_api_key".to_string(),
    )?;

    let validation = client.validate_key().await?;

    assert!(
        !validation,
        "Validation should return false with invalid API key"
    );

    cleanup_env();

    Ok(())
}

#[tokio::test]
async fn test_api_key_format() {
    // Test that API keys are properly formatted
    let valid_keys = vec![
        "rico_test123",
        "rico_Lg47wQANfgs_LdTNgNXvRMGXmAS2wfUNrJUYXpe2PaWRy",
        "rico_3DLhvjM4hQv_8xGUyQ4RxJXn62hsQYSZgr69T7cVUAR7X",
    ];

    for key in valid_keys {
        assert!(key.starts_with("rico_"), "API key should start with rico_");
    }

    // Test key abbreviation logic
    let long_key = "rico_Lg47wQANfgs_LdTNgNXvRMGXmAS2wfUNrJUYXpe2PaWRy";
    let abbreviated = if long_key.len() > 12 {
        format!("{}...", &long_key[..12])
    } else {
        long_key.to_string()
    };
    assert_eq!(
        abbreviated, "rico_Lg47wQA...",
        "Key should be properly abbreviated"
    );
}

#[test]
fn test_config_persistence() {
    let (_config, temp_dir) = create_test_config();

    // Create a config and save it
    let config1 = Config::for_test(
        Url::parse("https://test.server.com").unwrap(),
        Some("rico_test_key".to_string()),
    );

    // Mock the config path to use temp directory
    unsafe {
        env::set_var("HOME", temp_dir.path());
    }
    let save_result = config1.save();
    assert!(save_result.is_ok(), "Config save should succeed");

    // Load the config back
    let config2 = Config::load();
    assert!(config2.is_ok(), "Config load should succeed");

    let config2 = config2.unwrap();
    // With multi-server config, server info is in the servers map under "default"
    let default_server = config2
        .servers
        .get("default")
        .expect("default server should exist");
    assert_eq!(default_server.api_key, Some("rico_test_key".to_string()));
    assert_eq!(default_server.url.as_str(), "https://test.server.com/");

    cleanup_env();
}

#[tokio::test]
async fn test_oauth_url_construction() {
    let server = "https://test.ricochet.com";
    let callback_url = "http://localhost:12345/callback";

    // Test OAuth URL construction
    let oauth_url = format!(
        "{}/oauth/authorize?redirect_uri={}&response_type=code&client_id=cli",
        server,
        urlencoding::encode(callback_url)
    );

    assert!(oauth_url.contains("/oauth/authorize"));
    assert!(oauth_url.contains("redirect_uri="));
    assert!(oauth_url.contains("response_type=code"));
    assert!(oauth_url.contains("client_id=cli"));
    assert!(
        !oauth_url.contains("&redirect_uri"),
        "Should use ? not & for first param"
    );
}
