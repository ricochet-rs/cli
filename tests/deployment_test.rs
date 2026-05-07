use mockito::Server;
use serde_json::json;
use url::Url;

#[cfg(test)]
mod deployment_tests {
    use super::*;

    const CONTENT_ULID: &str = "01KQZMZXE17RV7TCNF3GGG24P4";
    const DEPLOYMENT_ULID: &str = "01KQZPF4Y5SRHES967VZEYY765";

    fn deployment_json() -> serde_json::Value {
        json!({
            "id": DEPLOYMENT_ULID,
            "content_id": CONTENT_ULID,
            "deployed_at": 1778106471,
            "status": "success",
            "deployed_by": "344509059241640593",
            "ip_address": "127.0.0.1",
            "requested_ver": "4.5.1",
            "matched_ver": "4.5.2",
            "git_hash": null
        })
    }

    // --- list_deployments ---

    #[tokio::test]
    async fn test_list_deployments_success() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{}/deployments", CONTENT_ULID).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(json!([deployment_json()]).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_deployments(CONTENT_ULID).await;

        assert!(result.is_ok());
        let deployments = result.unwrap();
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].id, DEPLOYMENT_ULID);
        assert_eq!(deployments[0].content_id, CONTENT_ULID);
    }

    #[tokio::test]
    async fn test_list_deployments_empty() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{}/deployments", CONTENT_ULID).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(json!([]).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_deployments(CONTENT_ULID).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_deployments_unauthorized() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{}/deployments", CONTENT_ULID).as_str(),
            )
            .match_header("authorization", "Key invalid_key")
            .with_status(401)
            .with_body(json!({"error": "Unauthorized"}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("invalid_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_deployments(CONTENT_ULID).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_deployments_can_serialize_to_json() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{}/deployments", CONTENT_ULID).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(json!([deployment_json()]).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_deployments(CONTENT_ULID).await.unwrap();

        let json_output = serde_json::to_string_pretty(&result);
        assert!(json_output.is_ok());

        let json_str = json_output.unwrap();
        assert!(json_str.contains(DEPLOYMENT_ULID));
        assert!(json_str.contains("success"));
        assert!(json_str.contains("4.5.1"));
    }

    // --- get_deployment ---

    #[tokio::test]
    async fn test_get_deployment_success() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/deployments/{}", DEPLOYMENT_ULID).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(deployment_json().to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.get_deployment(DEPLOYMENT_ULID).await;

        assert!(result.is_ok());
        let d = result.unwrap();
        assert_eq!(d.id, DEPLOYMENT_ULID);
        assert_eq!(d.content_id, CONTENT_ULID);
        assert_eq!(d.requested_ver.as_deref(), Some("4.5.1"));
        assert_eq!(d.matched_ver.as_deref(), Some("4.5.2"));
        assert!(d.git_hash.is_none());
    }

    #[tokio::test]
    async fn test_get_deployment_unauthorized() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/deployments/{}", DEPLOYMENT_ULID).as_str(),
            )
            .match_header("authorization", "Key invalid_key")
            .with_status(401)
            .with_body(json!({"error": "Unauthorized"}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("invalid_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.get_deployment(DEPLOYMENT_ULID).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_deployment_can_serialize_to_json() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/deployments/{}", DEPLOYMENT_ULID).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(deployment_json().to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.get_deployment(DEPLOYMENT_ULID).await.unwrap();

        let json_output = serde_json::to_string_pretty(&result);
        assert!(json_output.is_ok());

        let json_str = json_output.unwrap();
        assert!(json_str.contains(DEPLOYMENT_ULID));
        assert!(json_str.contains("success"));
        assert!(json_str.contains("4.5.1"));
    }
}
