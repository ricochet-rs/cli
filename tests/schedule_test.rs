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
        use ricochet_cli::item::schedule::parse_schedule;

        #[test]
        fn test_valid_cron_expressions() {
            let valid = [
                "0 9 * * 1-5",  // weekdays at 9am
                "*/15 * * * *", // every 15 minutes
                "0 0 * * *",    // daily at midnight
                "0 0 1 * *",    // first of every month
                "30 6 * * 1",   // monday at 6:30am
                "@daily",       // nickname
            ];

            for expr in valid {
                assert!(parse_schedule(expr).is_ok(), "Expected valid cron: {expr}");
            }
        }

        #[test]
        fn test_shortcut_step_expressions_are_rejected_like_the_server() {
            let shortcut = [
                "5/5 * * * *",  // every 5 minutes starting at minute 5
                "0/10 * * * *", // every 10 minutes starting at minute 0
                "/10 * * * *",  // same as */10
            ];

            for expr in shortcut {
                assert!(
                    parse_schedule(expr).is_err(),
                    "Expected the server to reject cron: {expr}"
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
                    parse_schedule(expr).is_err(),
                    "Expected invalid cron: {expr}"
                );
            }
        }

        #[test]
        fn test_next_occurrence_is_in_the_future() {
            use chrono::Utc;

            let cron = parse_schedule("0 9 * * 1-5").unwrap();
            let next = cron.find_next_occurrence(&Utc::now(), false).unwrap();
            assert!(next > Utc::now());
        }

        #[test]
        fn test_next_occurrence_matches_the_scheduled_time() {
            use chrono::{TimeZone, Utc};

            // a Thursday, half a minute past noon
            let from = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 30).unwrap();
            let cases = [
                ("0 9 * * 1-5", Utc.with_ymd_and_hms(2026, 1, 2, 9, 0, 0)),
                ("*/15 * * * *", Utc.with_ymd_and_hms(2026, 1, 1, 12, 15, 0)),
                ("@daily", Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0)),
            ];

            for (expr, expected) in cases {
                let next = parse_schedule(expr)
                    .unwrap()
                    .find_next_occurrence(&from, false)
                    .unwrap();
                assert_eq!(next, expected.unwrap(), "Unexpected next run for {expr}");
            }
        }
    }
}
