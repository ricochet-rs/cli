use mockito::Server;
use serde_json::json;
use url::Url;

#[cfg(test)]
mod schedule_tests {
    use super::*;

    #[tokio::test]
    async fn test_schedule_success() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";
        let cron = "0 9 * * 1-5";

        let _m = server
            .mock(
                "PATCH",
                format!("/api/v0/content/{}/schedule", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!({
                    "message": "Schedule updated successfully",
                    "schedule": cron
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
        let result = client.schedule(content_id, cron).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["message"], "Schedule updated successfully");
        assert_eq!(response["schedule"], cron);
    }

    #[tokio::test]
    async fn test_schedule_unauthorized() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";

        let _m = server
            .mock(
                "PATCH",
                format!("/api/v0/content/{}/schedule", content_id).as_str(),
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
        let result = client.schedule(content_id, "0 9 * * 1-5").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_schedule_not_found() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56NOTFOUND";

        let _m = server
            .mock(
                "PATCH",
                format!("/api/v0/content/{}/schedule", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(404)
            .with_body(json!({"error": "Content not found"}).to_string())
            .create();

        let config = ricochet_cli::config::Config::for_test(
            Url::parse(&server.url()).unwrap(),
            Some("test_api_key".to_string()),
        );

        let server_config = config.resolve_server(None).unwrap();
        let client = ricochet_cli::client::RicochetClient::new(&server_config).unwrap();
        let result = client.schedule(content_id, "0 9 * * 1-5").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_schedule_response_serializes_to_json() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";
        let cron = "*/15 * * * *";

        let _m = server
            .mock(
                "PATCH",
                format!("/api/v0/content/{}/schedule", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!({
                    "message": "Schedule updated successfully",
                    "schedule": cron
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
        let result = client.schedule(content_id, cron).await.unwrap();

        let json_str = serde_json::to_string_pretty(&result).unwrap();
        assert!(json_str.contains("Schedule updated successfully"));
        assert!(json_str.contains(cron));
    }

    #[tokio::test]
    async fn test_schedule_response_serializes_to_yaml() {
        let mut server = Server::new_async().await;
        let content_id = "01JSZAXZ3TSTAYXP56ARDVFJCJ";
        let cron = "0 0 * * *";

        let _m = server
            .mock(
                "PATCH",
                format!("/api/v0/content/{}/schedule", content_id).as_str(),
            )
            .match_header("authorization", "Key test_api_key")
            .with_status(200)
            .with_body(
                json!({
                    "message": "Schedule updated successfully",
                    "schedule": cron
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
        let result = client.schedule(content_id, cron).await.unwrap();

        let yaml_str = serde_yaml::to_string(&result).unwrap();
        assert!(yaml_str.contains("Schedule updated successfully"));
        assert!(yaml_str.contains(cron));
    }

    // These tests validate the cron parsing logic that runs before the API call.
    // They don't need a mock server since they fail locally.
    mod cron_validation {
        use std::str::FromStr;

        #[test]
        fn test_valid_cron_expressions() {
            let valid = [
                "0 9 * * 1-5",  // weekdays at 9am
                "*/15 * * * *", // every 15 minutes
                "0 0 * * *",    // daily at midnight
                "0 0 1 * *",    // first of every month
                "30 6 * * 1",   // monday at 6:30am
            ];

            for expr in valid {
                assert!(
                    croner::Cron::from_str(expr).is_ok(),
                    "Expected valid cron: {expr}"
                );
            }
        }

        #[test]
        fn test_invalid_cron_expressions() {
            let invalid = [
                "not a cron",
                "99 * * * *", // invalid minute
                "* 25 * * *", // invalid hour
            ];

            for expr in invalid {
                assert!(
                    croner::Cron::from_str(expr).is_err(),
                    "Expected invalid cron: {expr}"
                );
            }
        }

        #[test]
        fn test_next_occurrence_is_in_the_future() {
            use chrono::Utc;

            let cron = croner::Cron::from_str("0 9 * * 1-5").unwrap();
            let next = cron.find_next_occurrence(&Utc::now(), false).unwrap();
            assert!(next > Utc::now());
        }
    }
}
