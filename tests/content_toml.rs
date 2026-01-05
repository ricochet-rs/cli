use mockito::Server;
use ricochet_cli::{commands::item::toml::get_toml, config::Config};
use std::{env, fs};
use tempfile::TempDir;

const TEST_TOML_RESPONSE: &str = r#"[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "example-app"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"

[serve]
min_instances = 0
max_instances = 5
spawn_threshold = 80
max_connections = 10
"#;

const LOCAL_TOML_CONTENT: &str = r#"[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "local-app"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;

/// Helper to clean up environment variables after tests
fn cleanup_env() {
    unsafe {
        env::remove_var("RICOCHET_API_KEY");
        env::remove_var("RICOCHET_SERVER");
    }
}

/// Helper to create a config with mock server and API key
fn create_test_config(server_url: String) -> Config {
    let mut config = Config::default();
    config.server = url::Url::parse(&server_url).unwrap();
    config.api_key = Some("rico_test_key_123".to_string());
    config
}

/// Mock the check_key endpoint
async fn mock_check_key(server: &mut Server) -> mockito::Mock {
    server
        .mock("GET", "/api/v0/check_key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"valid": true}"#)
        .create_async()
        .await
}

/// Mock the toml endpoint
async fn mock_get_toml(server: &mut Server, id: &str) -> mockito::Mock {
    server
        .mock("GET", format!("/api/v0/content/{}/toml", id).as_str())
        .with_status(200)
        .with_header("content-type", "application/toml")
        .with_body(TEST_TOML_RESPONSE)
        .create_async()
        .await
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_toml_with_provided_id() {
    cleanup_env();
    let mut server = Server::new_async().await;

    let _mock_check = mock_check_key(&mut server).await;
    let _mock_toml = mock_get_toml(&mut server, "01KE52BY41EQ7NE89K7Z5MMZ84").await;

    let config = create_test_config(server.url());

    // Test with provided ID
    let result = get_toml(
        &config,
        Some("01KE52BY41EQ7NE89K7Z5MMZ84".to_string()),
        None,
    )
    .await;

    assert!(result.is_ok(), "get_toml should succeed with provided ID");

    cleanup_env();
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_toml_with_default_path() {
    cleanup_env();
    let mut server = Server::new_async().await;

    let _mock_check = mock_check_key(&mut server).await;
    let _mock_toml = mock_get_toml(&mut server, "01KE52BY41EQ7NE89K7Z5MMZ84").await;

    let config = create_test_config(server.url());

    // Create a temporary directory and write _ricochet.toml
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("_ricochet.toml");
    fs::write(&toml_path, LOCAL_TOML_CONTENT).unwrap();

    // Change to temp directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Test without ID or path (should read from ./_ricochet.toml)
    let result = get_toml(&config, None, None).await;

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(
        result.is_ok(),
        "get_toml should succeed reading from default _ricochet.toml"
    );

    cleanup_env();
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_toml_with_specified_path() {
    cleanup_env();
    let mut server = Server::new_async().await;

    let _mock_check = mock_check_key(&mut server).await;
    let _mock_toml = mock_get_toml(&mut server, "01KE52BY41EQ7NE89K7Z5MMZ84").await;

    let config = create_test_config(server.url());

    // Create a temporary directory structure: ./some/other/path/_ricochet.toml
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("some").join("other").join("path");
    fs::create_dir_all(&nested_path).unwrap();
    let toml_path = nested_path.join("_ricochet.toml");
    fs::write(&toml_path, LOCAL_TOML_CONTENT).unwrap();

    // Test with specified path
    let result = get_toml(&config, None, Some(toml_path)).await;

    assert!(
        result.is_ok(),
        "get_toml should succeed with specified path"
    );

    cleanup_env();
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_toml_missing_id_and_file() {
    cleanup_env();
    let mut server = Server::new_async().await;

    let _mock_check = mock_check_key(&mut server).await;

    let config = create_test_config(server.url());

    // Create a temporary directory without _ricochet.toml
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Test without ID or path and no _ricochet.toml should fail
    let result = get_toml(&config, None, None).await;

    env::set_current_dir(original_dir).unwrap();

    assert!(
        result.is_err(),
        "get_toml should fail when no ID provided and no _ricochet.toml found"
    );

    cleanup_env();
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_toml_file_without_id() {
    cleanup_env();
    let mut server = Server::new_async().await;

    let _mock_check = mock_check_key(&mut server).await;

    let config = create_test_config(server.url());

    // Create _ricochet.toml without an id field
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("_ricochet.toml");
    let toml_without_id = r#"[content]
name = "app-without-id"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;
    fs::write(&toml_path, toml_without_id).unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Test should fail because _ricochet.toml has no id
    let result = get_toml(&config, None, None).await;

    env::set_current_dir(original_dir).unwrap();

    assert!(
        result.is_err(),
        "get_toml should fail when _ricochet.toml has no id field"
    );

    cleanup_env();
}
