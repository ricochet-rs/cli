use mockito::Server;
use serde_json::json;
use url::Url;

fn client_for(server: &mockito::Server, key: &str) -> ricochet_cli::client::RicochetClient {
    let config = ricochet_cli::config::Config::for_test(
        Url::parse(&server.url()).unwrap(),
        Some(key.to_string()),
    );
    let server_config = config.resolve_server(None).unwrap();
    ricochet_cli::client::RicochetClient::new(&server_config).unwrap()
}

#[tokio::test]
async fn test_update_settings_success() {
    let mut server = Server::new_async().await;
    let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

    let _m = server
        .mock(
            "PATCH",
            format!("/api/v0/content/{}/settings", content_id).as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(200)
        .with_body("ok")
        .create();

    let client = client_for(&server, "test_api_key");
    let patch = json!({"serve": {"min_instances": 2}});
    let result = client.update_settings(content_id, &patch).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_settings_unauthorized() {
    let mut server = Server::new_async().await;
    let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

    let _m = server
        .mock(
            "PATCH",
            format!("/api/v0/content/{}/settings", content_id).as_str(),
        )
        .match_header("authorization", "Key bad_key")
        .with_status(401)
        .with_body(json!({"error": "Unauthorized"}).to_string())
        .create();

    let client = client_for(&server, "bad_key");
    let patch = json!({"serve": {"min_instances": 2}});
    let result = client.update_settings(content_id, &patch).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_settings_not_found() {
    let mut server = Server::new_async().await;
    let content_id = "01JSZAXZ3TSTAYXP56NOTFOUND";

    let _m = server
        .mock(
            "PATCH",
            format!("/api/v0/content/{}/settings", content_id).as_str(),
        )
        .match_header("authorization", "Key test_api_key")
        .with_status(404)
        .with_body(json!({"error": "Content not found"}).to_string())
        .create();

    let client = client_for(&server, "test_api_key");
    let patch = json!({"serve": {"min_instances": 2}});
    let result = client.update_settings(content_id, &patch).await;
    assert!(result.is_err());
}
