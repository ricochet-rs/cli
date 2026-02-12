use mockito::{Matcher, Server};
use serde_json::json;
use url::Url;

#[cfg(test)]
mod invoke_tests {
    use super::*;

    #[tokio::test]
    async fn test_invoke_success() {
        // Create mock server
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        // Mock the invoke endpoint
        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{}/invoke", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Json(json!({})))
            .with_status(200)
            .with_body(
                json!({
                    "invocation_id": "01JSZB123456789ABCDEFGHIJ",
                    "status": "running",
                    "content_id": content_id
                })
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.invoke(content_id, None).await;

        if let Err(ref e) = result {
            eprintln!("Error invoking: {:?}", e);
        }

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.get("invocation_id").is_some());
        assert_eq!(response["content_id"], content_id);
    }

    #[tokio::test]
    async fn test_invoke_unauthorized() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{}/invoke", content_id).as_str(),
            )
            .match_header("authorization", "Key invalid_key")
            .with_status(401)
            .with_body(json!({"error": "Unauthorized"}).to_string())
            .create();

        // Create test config with invalid key
        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("invalid_key".to_string()),
        );

        // Create client and invoke
        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.invoke(content_id, None).await;

        // Debug print the error
        if let Err(ref e) = result {
            eprintln!("Expected error for unauthorized: {:?}", e);
        }

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoke_not_found() {
        // Create mock server
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56NOTFOUND";

        // Mock the invoke endpoint with 404 error
        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{}/invoke", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(404)
            .with_body(json!({"error": "Content not found"}).to_string())
            .create();

        // Create test config
        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        // Create client and invoke
        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.invoke(content_id, None).await;

        // Debug print the error
        if let Err(ref e) = result {
            eprintln!("Expected error for not found: {:?}", e);
        }

        assert!(result.is_err());
    }

    // short of being able to actually test the output
    // we just check tha the output string is json/yaml formatted
    // the way we anticipate
    #[tokio::test]
    async fn test_invoke_response_can_serialize_to_json() {
        // Create mock server
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{}/invoke", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Json(json!({})))
            .with_status(200)
            .with_body(
                json!({
                    "invocation_id": "01JSZB123456789ABCDEFGHIJ",
                    "status": "running",
                    "content_id": content_id
                })
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.invoke(content_id, None).await.unwrap();

        // Test that we can serialize the response to JSON
        let json_output = serde_json::to_string_pretty(&result);
        assert!(json_output.is_ok());

        let json_str = json_output.unwrap();
        println!("JSON Output:\n{}", json_str);

        // Verify the JSON contains expected fields
        assert!(json_str.contains("invocation_id"));
        assert!(json_str.contains("01JSZB123456789ABCDEFGHIJ"));
        assert!(json_str.contains("status"));
        assert!(json_str.contains("running"));
        assert!(json_str.contains("content_id"));
    }

    #[tokio::test]
    async fn test_invoke_response_can_serialize_to_yaml() {
        // Create mock server
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "POST",
                format!("/api/v0/content/{}/invoke", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Json(json!({})))
            .with_status(200)
            .with_body(
                json!({
                    "invocation_id": "01JSZB123456789ABCDEFGHIJ",
                    "status": "running",
                    "content_id": content_id
                })
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.invoke(content_id, None).await.unwrap();

        // Test that we can serialize the response to YAML
        let yaml_output = serde_yaml::to_string(&result);
        assert!(yaml_output.is_ok());

        let yaml_str = yaml_output.unwrap();
        println!("YAML Output:\n{}", yaml_str);

        // Verify the YAML contains expected fields
        assert!(yaml_str.contains("invocation_id"));
        assert!(yaml_str.contains("01JSZB123456789ABCDEFGHIJ"));
        assert!(yaml_str.contains("status"));
        assert!(yaml_str.contains("running"));
        assert!(yaml_str.contains("content_id"));
    }
}
