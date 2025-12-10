use mockito::{Matcher, Server};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod deploy_tests {
    use super::*;

    fn create_test_project(dir: &Path, content_id: Option<&str>) -> std::io::Result<()> {
        // Create _ricochet.toml
        let toml_content = if let Some(id) = content_id {
            format!(
                r#"[content]
id = "{}"
name = "test-app"
content_type = "shiny"
entrypoint = "app.R"
access_type = "private"

[language]
name = "r"
packages = "renv.lock"
"#,
                id
            )
        } else {
            r#"[content]
content_type = "shiny"
name = "test-app"
entrypoint = "app.R"
access_type = "private"

[language]
name = "r"
packages = "renv.lock"
"#
            .to_string()
        };
        fs::write(dir.join("_ricochet.toml"), toml_content)?;

        // Create a sample app.R file
        fs::write(
            dir.join("app.R"),
            r#"library(shiny)
ui <- fluidPage(titlePanel("Test"))
server <- function(input, output) {}
shinyApp(ui = ui, server = server)"#,
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_new_content() {
        // Create a temporary directory for test project
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        create_test_project(project_path, None).unwrap();

        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for new content deployment
        let _m = server
            .mock("POST", "/api/v0/content/upload")
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Any) // Multipart form data
            .with_status(200)
            .with_body(
                json!({
                    "id": "01JZA237920RN65T2XHCCV7296",
                    "name": "test-content",
                    "content_type": "shiny",
                    "status": "deployed"
                })
                .to_string(),
            )
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Run deploy command
        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        if let Err(e) = &result {
            dbg!(&e);
        };

        assert!(result.is_ok());

        // Check that _ricochet.toml was updated with the content ID
        let updated_toml = fs::read_to_string(project_path.join("_ricochet.toml")).unwrap();
        assert!(updated_toml.contains("01JZA237920RN65T2XHCCV7296"));
    }

    #[tokio::test]
    async fn test_deploy_existing_content() {
        // Create a temporary directory for test project with existing content ID
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let existing_id = "01JZA237920RN65T2XHCCV7296";
        create_test_project(project_path, Some(existing_id)).unwrap();

        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response for updating existing content
        let _m = server
            .mock("POST", "/api/v0/content/upload")
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Any) // Should contain id field
            .with_status(200)
            .with_body(
                json!({
                    "id": existing_id,
                    "name": "test-content",
                    "content_type": "shiny",
                    "status": "deployed"
                })
                .to_string(),
            )
            .create();

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Run deploy command
        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        if let Err(e) = &result {
            dbg!(&e);
        };

        assert!(result.is_ok());

        // Check that _ricochet.toml still contains the same content ID
        let updated_toml = fs::read_to_string(project_path.join("_ricochet.toml")).unwrap();
        assert!(updated_toml.contains(existing_id));
    }

    #[tokio::test]
    async fn test_deploy_missing_toml() {
        // Create a temporary directory without _ricochet.toml
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create mock server (not used but needed for config)
        let server = Server::new_async().await;

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Run deploy command - should fail
        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No _ricochet.toml found")
        );
    }

    #[tokio::test]
    async fn test_deploy_invalid_toml() {
        // Create a temporary directory with invalid _ricochet.toml
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create invalid TOML (missing content section)
        fs::write(
            project_path.join("_ricochet.toml"),
            r#"[invalid]
key = "value"
"#,
        )
        .unwrap();

        // Create mock server (not used but needed for config)
        let server = Server::new_async().await;

        // Create test config
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Run deploy command - should fail
        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deploy_api_key_error() {
        // Create a temporary directory for test project
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        create_test_project(project_path, None).unwrap();

        // Create mock server
        let mut server = Server::new_async().await;

        // Mock the server response with 403 error
        let _m = server
            .mock("POST", "/api/v0/content/upload")
            .match_header("authorization", "Key invalid_key")
            .with_status(403)
            .with_body(json!({"error": "Invalid API key"}).to_string())
            .create();

        // Create test config with invalid key
        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("invalid_key".to_string()),
            default_format: Some("table".to_string()),
        };

        // Run deploy command - should fail
        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Authentication failed") || error_msg.contains("Invalid API key")
        );
    }

    #[tokio::test]
    async fn test_deploy_multipart_form_structure() {
        // This test verifies the multipart form structure matches the R function
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Test new deployment (should include 'bundle' and 'config' fields)
        create_test_project(project_path, None).unwrap();

        // Create mock server
        let mut server = Server::new_async().await;

        let _m = server
            .mock("POST", "/api/v0/content/upload")
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Regex(
                r#"Content-Disposition: form-data; name="bundle""#.to_string(),
            ))
            .match_body(Matcher::Regex(
                r#"Content-Disposition: form-data; name="config""#.to_string(),
            ))
            .with_status(200)
            .with_body(
                json!({
                    "id": "01JZA237920RN65T2XHCCV7296",
                    "name": "test-content",
                    "content_type": "shiny",
                    "status": "deployed"
                })
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        if let Err(e) = &result {
            dbg!(&e);
        };

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_deploy_update_multipart_form_structure() {
        // This test verifies the multipart form for updates (should include 'bundle' and 'id' fields)
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let existing_id = "01JZA237920RN65T2XHCCV7296";

        create_test_project(project_path, Some(existing_id)).unwrap();

        // Create mock server
        let mut server = Server::new_async().await;

        let _m = server
            .mock("POST", "/api/v0/content/upload")
            .match_header("authorization", "Key test_api_key")
            .match_body(Matcher::Regex(
                r#"Content-Disposition: form-data; name="bundle""#.to_string(),
            ))
            .match_body(Matcher::Regex(
                r#"Content-Disposition: form-data; name="id""#.to_string(),
            ))
            .with_status(200)
            .with_body(
                json!({
                    "id": existing_id,
                    "name": "test-content",
                    "content_type": "shiny",
                    "status": "deployed"
                })
                .to_string(),
            )
            .create();

        let config = ricochet_cli::config::Config {
            server: Some(server.url()),
            api_key: Some("test_api_key".to_string()),
            default_format: Some("table".to_string()),
        };

        let result = ricochet_cli::commands::deploy::deploy(
            &config,
            project_path.to_path_buf(),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_ok());
    }
}
