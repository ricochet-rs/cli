use mockito::Server;
use serde_json::json;

#[cfg(test)]
mod delete_tests {
    use super::*;

    #[tokio::test]
    async fn test_delete_with_force() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for delete
        let content_id = "01K66JV2Q123456789ABCDEF";
        let _m = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test delete with force flag (no confirmation)
        let result =
            ricochet_cli::commands::delete::delete(&config, content_id, true).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for non-existent content
        let content_id = "01K66JV2QNONEXISTENT";
        let _m = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(404)
            .with_body(json!({"error": "Content not found"}).to_string())
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test delete with non-existent content
        let result =
            ricochet_cli::commands::delete::delete(&config, content_id, true).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to delete"));
    }

    #[tokio::test]
    async fn test_delete_unauthorized() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for unauthorized access
        let content_id = "01K66JV2Q123456789ABCDEF";
        let _m = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id).as_str())
            .match_header("authorization", "Key invalid_key")
            .with_status(403)
            .with_body(json!({"error": "Unauthorized"}).to_string())
            .create();

        // Create test config with invalid key
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("invalid_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test delete with invalid API key
        let result =
            ricochet_cli::commands::delete::delete(&config, content_id, true).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_server_error() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for internal server error
        let content_id = "01K66JV2Q123456789ABCDEF";
        let _m = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(500)
            .with_body(json!({"error": "Internal server error"}).to_string())
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test delete with server error
        let result =
            ricochet_cli::commands::delete::delete(&config, content_id, true).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to delete"));
    }

    #[tokio::test]
    async fn test_delete_correct_endpoint() {
        // This test verifies the correct endpoint is being called
        let mut server = Server::new_async().await;

        let content_id = "01K66JV2Q123456789ABCDEF";

        // Mock should match the exact endpoint pattern
        let _m = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .create();

        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        let result =
            ricochet_cli::commands::delete::delete(&config, content_id, true).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_multiple_items() {
        // Test deleting multiple items sequentially
        let mut server = Server::new_async().await;

        let content_id1 = "01K66JV2Q111111111111111";
        let content_id2 = "01K66JV2Q222222222222222";

        let _m1 = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id1).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .create();

        let _m2 = server
            .mock("DELETE", format!("/api/v0/content/{}", content_id2).as_str())
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .create();

        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Delete first item
        let result1 =
            ricochet_cli::commands::delete::delete(&config, content_id1, true).await;
        assert!(result1.is_ok());

        // Delete second item
        let result2 =
            ricochet_cli::commands::delete::delete(&config, content_id2, true).await;
        assert!(result2.is_ok());
    }
}
