use mockito::{Matcher, Server};
use ricochet_cli::OutputFormat;
use serde_json::json;
use url::Url;

const CONTENT_ID: &str = "01K66JV2Q123456789ABCDEF";

// Test-only RSA public key, used to serve /api/v0/public-key so `set`/`replace`
// can complete their encrypt-then-send flow. See src/crypto.rs for the tests
// that verify encryption/decryption actually round-trips.
const TEST_PUB_PEM: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAr1XuDE4bFt7TnYqAtiRQ9RvC2sG3s8N8zUsCvhM+mZD7mGTN47bk
vYxKvp5ShVnM/6XZeCfRQA2TKXnf6dWsRgcZcBMufKHfN9VLNxawLMKHddceHlLA
rFTwsPE9rU9p5p5uA6zhUnZk/skzWumqZw9WK7Lztbh6fhX9UMYXvaBzCFF1nfTM
kGl7YkRcwfL4p+1oa7uGFYaRxvBKv6q9/hm7W9Em7H0g4+icc85wkvlzJrghKakp
5wDkaY8XmSGSiOZr0U8/fPBC4SASPuT5Hy17zZwu7SEYW31JYnRvFoo8bF8N3QxT
WigXLNxbQJjhAq7Y6mU8h7yF2zWMbFGMqwIDAQAB
-----END RSA PUBLIC KEY-----
";

fn test_config(server: &Server) -> ricochet_cli::config::Config {
    ricochet_cli::config::Config::for_test(
        Url::parse(&server.url()).unwrap(),
        Some("test_api_key".to_string()),
    )
}

// Every command runs a preflight check against this endpoint before doing
// anything else.
fn mock_valid_key(server: &mut Server) -> mockito::Mock {
    server
        .mock("GET", "/api/v0/check_key")
        .with_status(200)
        .create()
}

// --- get ---

#[tokio::test]
async fn test_get_env_vars_success() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock(
            "GET",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(200)
        .with_body(json!(["DATABASE_URL", "API_KEY"]).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::get_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_env_vars_unauthorized() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock(
            "GET",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(401)
        .with_body(json!({"error": "Unauthorized"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::get_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _m.assert_async().await;
}

// --- delete ---

#[tokio::test]
async fn test_delete_env_var_success() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock(
            "DELETE",
            format!("/api/v0/content/{CONTENT_ID}/env-vars/API_KEY").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(200)
        .with_body(json!(["DATABASE_URL"]).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::delete_env_var(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        "API_KEY",
        true,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_env_var_not_found() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock(
            "DELETE",
            format!("/api/v0/content/{CONTENT_ID}/env-vars/MISSING").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(404)
        .with_body(json!({"error": "Variable not found"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::delete_env_var(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        "MISSING",
        true,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _m.assert_async().await;
}

// --- set ---

#[tokio::test]
async fn test_set_env_vars_success() {
    let mut server = Server::new_async().await;
    let _check_key_mock = mock_valid_key(&mut server);

    let _key_mock = server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_body(TEST_PUB_PEM)
        .create();

    let _set_mock = server
        .mock(
            "PATCH",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .match_body(Matcher::Any)
        .with_status(200)
        .with_body(json!(["DATABASE_URL", "API_KEY"]).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::set_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        &["API_KEY=secret".to_string()],
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
    _key_mock.assert_async().await;
    _set_mock.assert_async().await;
}

#[tokio::test]
async fn test_set_env_vars_rejects_server_error() {
    let mut server = Server::new_async().await;
    let _check_key_mock = mock_valid_key(&mut server);

    let _key_mock = server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_body(TEST_PUB_PEM)
        .create();

    let _set_mock = server
        .mock(
            "PATCH",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(400)
        .with_body(json!({"error": "name is unusable"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::set_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        &["BAD KEY=secret".to_string()],
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _set_mock.assert_async().await;
}

// --- replace ---

#[tokio::test]
async fn test_replace_env_vars_success() {
    let mut server = Server::new_async().await;
    let _check_key_mock = mock_valid_key(&mut server);

    let _key_mock = server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_body(TEST_PUB_PEM)
        .create();

    let _replace_mock = server
        .mock(
            "PUT",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .match_body(Matcher::Any)
        .with_status(200)
        .with_body(json!(["DATABASE_URL"]).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::replace_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        &["DATABASE_URL=postgres://localhost".to_string()],
        true,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
    _key_mock.assert_async().await;
    _replace_mock.assert_async().await;
}

#[tokio::test]
async fn test_replace_env_vars_not_found() {
    let mut server = Server::new_async().await;
    let _check_key_mock = mock_valid_key(&mut server);

    let _key_mock = server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_body(TEST_PUB_PEM)
        .create();

    let _replace_mock = server
        .mock(
            "PUT",
            format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(404)
        .with_body(json!({"error": "Content not found"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::item::env_vars::replace_env_vars(
        &config,
        None,
        Some(CONTENT_ID),
        None,
        &["DATABASE_URL=postgres://localhost".to_string()],
        true,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _replace_mock.assert_async().await;
}
