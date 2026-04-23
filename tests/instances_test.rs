use mockito::Server;
use serde_json::json;
use url::Url;

#[cfg(test)]
mod instances_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_instances_success() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{content_id}/instances").as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!([
                    {
                        "instance_id": "01KPXMKQ8N9XCBQ06ZV33JFXCD",
                        "connections": 1,
                        "created_at": "2026-04-23T17:01:14.033620902Z",
                        "last_connection": 1776963683145i64
                    }
                ])
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_instances(content_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        let arr = response.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["instance_id"], "01KPXMKQ8N9XCBQ06ZV33JFXCD");
        assert_eq!(arr[0]["connections"], 1);
    }

    #[tokio::test]
    async fn test_list_instances_empty() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{content_id}/instances").as_str(),
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
        let result = client.list_instances(content_id).await.unwrap();

        assert!(result.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_instances_unauthorized() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{content_id}/instances").as_str(),
            )
            .match_header("authorization", "Key bad_key")
            .with_status(401)
            .with_body(json!({"error": "Unauthorized"}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("bad_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_instances(content_id).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_instance_success() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";
        let instance_id = "01KPXMKQ8N9XCBQ06ZV33JFXCD";

        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{content_id}/instances/{instance_id}/stop").as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(json!({"success": true}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.stop_instance(content_id, instance_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_instance_not_found() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";
        let instance_id = "01KPXMKQ8N9XCBQ06ZV33NOTFOUND";

        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{content_id}/instances/{instance_id}/stop").as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(404)
            .with_body(json!({"error": "Instance not found"}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.stop_instance(content_id, instance_id).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_instances_response_serializes_to_json() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "GET",
                format!("/api/v0/content/{content_id}/instances").as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!([
                    {
                        "instance_id": "01KPXMKQ8N9XCBQ06ZV33JFXCD",
                        "connections": 2,
                        "created_at": "2026-04-23T17:01:14.033620902Z",
                        "last_connection": 1776963683145i64
                    }
                ])
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.list_instances(content_id).await.unwrap();

        let json_str = serde_json::to_string_pretty(&result).unwrap();
        assert!(json_str.contains("01KPXMKQ8N9XCBQ06ZV33JFXCD"));
        assert!(json_str.contains("instance_id"));
        assert!(json_str.contains("connections"));
    }
}
