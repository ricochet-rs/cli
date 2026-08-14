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
async fn test_list_credentials_success() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::Missing)
        .match_header("authorization", "Key test_api_key")
        .with_status(200)
        .with_body(
            json!([
                {
                    "id": "01K66JV2Q123456789ABCDEF",
                    "user_id": "user_1",
                    "name": "deploy-key",
                    "type": "ssh"
                },
                {
                    "id": "01K66JV2Q987654321FEDCBA",
                    "user_id": "user_1",
                    "name": "https-token",
                    "type": "https"
                }
            ])
            .to_string(),
        )
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::user::list_credentials(
        &config,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
}

#[tokio::test]
async fn test_list_credentials_json_format() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::Missing)
        .with_status(200)
        .with_body(
            json!([
                {
                    "id": "01K66JV2Q123456789ABCDEF",
                    "user_id": "user_1",
                    "name": "deploy-key",
                    "type": "ssh"
                }
            ])
            .to_string(),
        )
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::user::list_credentials(
        &config,
        None,
        None,
        None,
        OutputFormat::Json,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
}

#[tokio::test]
async fn test_list_credentials_with_filters() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("user_id".into(), "user_2".into()),
            Matcher::UrlEncoded("type".into(), "ssh".into()),
        ]))
        .with_status(200)
        .with_body(
            json!([
                {
                    "id": "01K66JV2Q123456789ABCDEF",
                    "user_id": "user_2",
                    "name": "deploy-key",
                    "type": "ssh"
                }
            ])
            .to_string(),
        )
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::user::list_credentials(
        &config,
        None,
        Some("user_2"),
        Some(ricochet_core::config::git::GitProtocol::Ssh),
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
    _m.assert_async().await;
}

#[tokio::test]
async fn test_list_credentials_empty_response() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::Missing)
        .with_status(200)
        .with_body(json!([]).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::user::list_credentials(
        &config,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_ok(), "expected success, got {result:?}");
}

#[tokio::test]
async fn test_list_credentials_forbidden() {
    let mut server = Server::new_async().await;
    let _key_mock = mock_valid_key(&mut server);

    let _m = server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::Missing)
        .with_status(403)
        .with_body(json!({"error": "Insufficient privileges"}).to_string())
        .create();

    let config = test_config(&server);
    let result = ricochet_cli::commands::user::list_credentials(
        &config,
        None,
        None,
        None,
        OutputFormat::Table,
    )
    .await;

    assert!(result.is_err());
    _m.assert_async().await;
}
