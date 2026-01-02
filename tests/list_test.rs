use mockito::Server;
use serde_json::json;
use url::Url;

#[cfg(test)]
mod list_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_json_format() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for list items
        let _m = server
            .mock("GET", "/api/v0/user/items")
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!([
                    {
                        "id": "01K66JV2Q123456789ABCDEF",
                        "name": "Metadata Dashboard",
                        "content_type": "shiny",
                        "language": "R",
                        "visibility": "private",
                        "status": "deployed",
                        "updated_at": "2024-01-15T10:30:00Z"
                    },
                    {
                        "id": "01K66JV2Q987654321FEDCBA",
                        "name": "API Service",
                        "content_type": "api",
                        "language": "Python",
                        "visibility": "public",
                        "status": "running",
                        "updated_at": "2024-01-16T14:20:00Z"
                    }
                ])
                .to_string(),
            )
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Url::parse(&server.url()).unwrap(),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test JSON output format
        let result = ricochet_cli::commands::list::list(
            &config,
            None,
            false,
            None, // no sorting
            ricochet_cli::OutputFormat::Json,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_table_format_no_truncation() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response with long names and IDs
        let _m = server.mock("GET", "/api/v0/user/items")
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!([
                    {
                        "id": "01K66JV2Q123456789ABCDEFGHIJKLMNOP",
                        "name": "This is a very long content item name that should not be truncated in the output",
                        "content_type": "shiny",
                        "language": "R",
                        "visibility": "private",
                        "status": "deployed",
                        "updated_at": "2024-01-15T10:30:00Z"
                    }
                ])
                .to_string(),
            )
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Url::parse(&server.url()).unwrap(),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test table output format - should not truncate
        let result = ricochet_cli::commands::list::list(
            &config,
            None,
            false,
            None, // no sorting
            ricochet_cli::OutputFormat::Table,
        )
        .await;

        assert!(result.is_ok());
        // The actual output verification would require capturing stdout,
        // but the test ensures no panic occurs with long values
    }

    #[tokio::test]
    async fn test_list_with_filters() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response
        let _m = server
            .mock("GET", "/api/v0/user/items")
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!([
                    {
                        "id": "01K66JV2Q123",
                        "name": "Shiny App",
                        "content_type": "shiny",
                        "language": "R",
                        "visibility": "private",
                        "status": "deployed",
                        "updated_at": "2024-01-15T10:30:00Z"
                    },
                    {
                        "id": "01K66JV2Q456",
                        "name": "API",
                        "content_type": "api",
                        "language": "Python",
                        "visibility": "public",
                        "status": "failed",
                        "updated_at": "2024-01-16T14:20:00Z"
                    }
                ])
                .to_string(),
            )
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Url::parse(&server.url()).unwrap(),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test filtering by content type
        let result = ricochet_cli::commands::list::list(
            &config,
            Some("shiny".to_string()),
            false,
            None, // no sorting
            ricochet_cli::OutputFormat::Json,
        )
        .await;

        assert!(result.is_ok());

        // Test filtering by active only
        let result2 = ricochet_cli::commands::list::list(
            &config,
            None,
            true,
            None, // no sorting
            ricochet_cli::OutputFormat::Json,
        )
        .await;

        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_list_empty_response() {
        // Create mock server
        let mut server = Server::new_async().await;

        // Mock empty response
        let _m = server
            .mock("GET", "/api/v0/user/items")
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(json!([]).to_string())
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Url::parse(&server.url()).unwrap(),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Test empty list
        let result = ricochet_cli::commands::list::list(
            &config,
            None,
            false,
            None, // no sorting
            ricochet_cli::OutputFormat::Table,
        )
        .await;

        assert!(result.is_ok());
    }
}
