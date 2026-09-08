use mockito::{Matcher, Server};
use ricochet_cli::OutputFormat;
use serde_json::json;
use url::Url;

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

#[tokio::test]
async fn test_deploy_git_success_minimal() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("POST", "/api/v0/deploy/git")
        .match_header("authorization", "Key test_api_key")
        .match_body(Matcher::Regex("name=\"repo\"".to_string()))
        .with_status(200)
        .with_body(json!({"id": "01K66JV2Q123456789ABCDEF"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::deploy::deploy_git(
        &config,
        None,
        "https://github.com/org/repo".to_string(),
        None,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
}

#[tokio::test]
async fn test_deploy_git_success_with_all_fields() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let toml_dir = tempfile::tempdir().unwrap();
    let toml_path = toml_dir.path().join("_ricochet.toml");
    std::fs::write(&toml_path, "[content]\ncontent_type = \"shiny\"\n").unwrap();

    let _m = server
        .mock("POST", "/api/v0/deploy/git")
        .match_header("authorization", "Key test_api_key")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("name=\"repo\"".to_string()),
            Matcher::Regex("name=\"config\"".to_string()),
            Matcher::Regex("name=\"credential_id\"".to_string()),
            Matcher::Regex("\"branch\":\"main\"".to_string()),
            Matcher::Regex("\"path\":\"apps/dashboard\"".to_string()),
        ]))
        .with_status(200)
        .with_body(json!({"id": "01K66JV2Q123456789ABCDEF"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::deploy::deploy_git(
        &config,
        None,
        "https://github.com/org/repo".to_string(),
        Some("main".to_string()),
        Some("apps/dashboard".to_string()),
        Some(toml_path),
        Some("cred_123".to_string()),
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
    _m.assert_async().await;
}

#[tokio::test]
async fn test_deploy_git_bad_request() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("POST", "/api/v0/deploy/git")
        .match_header("authorization", "Key test_api_key")
        .with_status(400)
        .with_body(json!({"error": "Invalid deployment data"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::deploy::deploy_git(
        &config,
        None,
        "https://github.com/org/repo".to_string(),
        None,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _m.assert_async().await;
}

#[tokio::test]
async fn test_deploy_git_forbidden() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("POST", "/api/v0/deploy/git")
        .match_header("authorization", "Key test_api_key")
        .with_status(403)
        .with_body(json!({"error": "Insufficient privileges"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::deploy::deploy_git(
        &config,
        None,
        "https://github.com/org/repo".to_string(),
        None,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _m.assert_async().await;
}
