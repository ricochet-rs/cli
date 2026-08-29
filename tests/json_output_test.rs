//! `-F json` and `-F yaml` reserve stdout for the payload.
//!
//! Every test drives the real binary so it catches any status line, hint or
//! link that a command writes to stdout instead of stderr.

use mockito::{Matcher, Server, ServerGuard};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

const CONTENT_ID: &str = "01K66JV2Q123456789ABCDEF";
const DEPLOYMENT_ID: &str = "01KQZPF4Y5SRHES967VZEYY765";
const INSTANCE_ID: &str = "01KQZMZXE17RV7TCNF3GGG24P4";

/// Test-only RSA public key, served so the env-var commands can encrypt.
const TEST_PUB_PEM: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAr1XuDE4bFt7TnYqAtiRQ9RvC2sG3s8N8zUsCvhM+mZD7mGTN47bk
vYxKvp5ShVnM/6XZeCfRQA2TKXnf6dWsRgcZcBMufKHfN9VLNxawLMKHddceHlLA
rFTwsPE9rU9p5p5uA6zhUnZk/skzWumqZw9WK7Lztbh6fhX9UMYXvaBzCFF1nfTM
kGl7YkRcwfL4p+1oa7uGFYaRxvBKv6q9/hm7W9Em7H0g4+icc85wkvlzJrghKakp
5wDkaY8XmSGSiOZr0U8/fPBC4SASPuT5Hy17zZwu7SEYW31JYnRvFoo8bF8N3QxT
WigXLNxbQJjhAq7Y6mU8h7yF2zWMbFGMqwIDAQAB
-----END RSA PUBLIC KEY-----
";

const REMOTE_TOML: &str = r#"[content]
id = "01K66JV2Q123456789ABCDEF"
name = "remote-app"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;

const LOCAL_TOML: &str = r#"[content]
id = "01K66JV2Q123456789ABCDEF"
name = "local-app"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"
"#;

/// Register every endpoint the exercised commands touch.
fn mock_api(server: &mut Server) {
    server
        .mock("GET", "/api/v0/check_key")
        .with_status(200)
        .create();

    server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_header("content-type", "application/x-pem-file")
        .with_body(TEST_PUB_PEM)
        .create();

    server
        .mock("GET", "/api/v0/user/items")
        .with_status(200)
        .with_body(
            json!([
                {
                    "id": CONTENT_ID,
                    "name": "Metadata Dashboard",
                    "content_type": "shiny",
                    "language": "R",
                    "visibility": "private",
                    "status": "deployed",
                    "updated_at": "2024-01-15T10:30:00Z"
                },
                {
                    "id": "01K66JV2Q987654321FEDCBA",
                    "name": "Nightly Report",
                    "content_type": "quarto-r",
                    "language": "R",
                    "visibility": "private",
                    "status": "success",
                    "updated_at": "2024-01-16T14:20:00Z"
                }
            ])
            .to_string(),
        )
        .create();

    server
        .mock("GET", "/api/v0/user/credentials")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_body(
            json!([{
                "id": "credential-id",
                "user_id": "user-id",
                "name": "deploy-key",
                "type": "ssh"
            }])
            .to_string(),
        )
        .create();

    server
        .mock("GET", format!("/api/v0/content/{CONTENT_ID}/toml").as_str())
        .with_status(200)
        .with_header("content-type", "application/toml")
        .with_body(REMOTE_TOML)
        .create();

    server
        .mock(
            "GET",
            format!("/api/v0/content/{CONTENT_ID}/instances").as_str(),
        )
        .with_status(200)
        .with_body(
            json!([{
                "instance_id": INSTANCE_ID,
                "connections": 2,
                "created_at": "2024-01-15T10:30:00Z",
                "last_connection": 0
            }])
            .to_string(),
        )
        .create();

    server
        .mock(
            "POST",
            format!("/api/v0/content/{CONTENT_ID}/instances/{INSTANCE_ID}/stop").as_str(),
        )
        .with_status(200)
        .create();

    server
        .mock(
            "GET",
            format!("/api/v0/content/{CONTENT_ID}/deployments").as_str(),
        )
        .with_status(200)
        .with_body(json!([deployment()]).to_string())
        .create();

    server
        .mock(
            "GET",
            format!("/api/v0/content/deployments/{DEPLOYMENT_ID}").as_str(),
        )
        .with_status(200)
        .with_body(deployment().to_string())
        .create();

    for method in ["GET", "PATCH", "PUT"] {
        server
            .mock(
                method,
                format!("/api/v0/content/{CONTENT_ID}/env-vars").as_str(),
            )
            .with_status(200)
            .with_body(json!(["DATABASE_URL"]).to_string())
            .create();
    }

    server
        .mock(
            "DELETE",
            format!("/api/v0/content/{CONTENT_ID}/env-vars/DATABASE_URL").as_str(),
        )
        .with_status(200)
        .with_body(json!([]).to_string())
        .create();

    server
        .mock(
            "POST",
            format!("/api/v0/content/{CONTENT_ID}/invoke").as_str(),
        )
        .with_status(200)
        .with_body(
            json!({
                "invocation_id": "01KQZPF4Y5SRHES967VZEYY765",
                "content_id": CONTENT_ID,
                "status": "pending"
            })
            .to_string(),
        )
        .create();

    server
        .mock(
            "PATCH",
            format!("/api/v0/content/{CONTENT_ID}/schedule").as_str(),
        )
        .with_status(200)
        .with_body(json!({"content_id": CONTENT_ID, "schedule": "0 9 * * 1-5"}).to_string())
        .create();

    server
        .mock(
            "PATCH",
            format!("/api/v0/content/{CONTENT_ID}/settings").as_str(),
        )
        .with_status(200)
        .create();

    server
        .mock("DELETE", format!("/api/v0/content/{CONTENT_ID}").as_str())
        .with_status(200)
        .create();

    server
        .mock("POST", "/api/v0/content/upload")
        .match_body(Matcher::Any)
        .with_status(200)
        .with_body(
            json!({"id": CONTENT_ID, "deployment_id": DEPLOYMENT_ID, "status": "deployed"})
                .to_string(),
        )
        .create();

    server
        .mock("POST", "/api/v0/deploy/git")
        .match_body(Matcher::Any)
        .with_status(200)
        .with_body(json!({"id": CONTENT_ID}).to_string())
        .create();
}

fn deployment() -> serde_json::Value {
    json!({
        "id": DEPLOYMENT_ID,
        "content_id": CONTENT_ID,
        "deployed_at": 1778106471,
        "status": "success",
        "deployed_by": "344509059241640593",
        "ip_address": "127.0.0.1",
        "requested_ver": "4.5.1",
        "matched_ver": "4.5.2",
        "git_hash": null
    })
}

/// A mock server plus the throwaway home directory the CLI stores its config in.
struct Cli {
    /// Held so the mock server outlives the commands under test.
    _server: ServerGuard,
    home: TempDir,
}

impl Cli {
    async fn new() -> Self {
        let mut server = Server::new_async().await;
        mock_api(&mut server);

        // The CLI reads its configuration from `$HOME/.config/ricochet`.
        let home = TempDir::new().expect("creating a temporary home");
        let config_dir = home.path().join(".config").join("ricochet");
        std::fs::create_dir_all(&config_dir).expect("creating the config directory");
        std::fs::write(
            config_dir.join("config.toml"),
            format!(
                "default_server = \"test\"\ndefault_format = \"table\"\n\n[servers.test]\nurl = \"{}\"\napi_key = \"test_api_key\"\n",
                server.url()
            ),
        )
        .expect("writing the test config");

        Self {
            _server: server,
            home,
        }
    }

    async fn run(&self, args: &[&str]) -> std::process::Output {
        tokio::process::Command::new(env!("CARGO_BIN_EXE_ricochet"))
            .args(args)
            .env("HOME", self.home.path())
            .env_remove("RICOCHET_SERVER")
            .env_remove("RICOCHET_API_KEY")
            .env("RICOCHET_NO_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .output()
            .await
            .expect("running the ricochet binary")
    }

    /// Run with `-F json` and return the parsed stdout.
    async fn json(&self, args: &[&str]) -> serde_json::Value {
        let mut with_format = args.to_vec();
        with_format.extend(["-F", "json"]);
        let output = self.run(&with_format).await;
        let stdout = String::from_utf8(output.stdout).expect("stdout is valid UTF-8");

        assert!(
            output.status.success(),
            "`{}` failed: {}",
            with_format.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );

        serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "`{}` did not write JSON alone to stdout ({e}):\n{stdout}",
                with_format.join(" ")
            )
        })
    }

    /// Run with `-F yaml` and return the parsed stdout.
    async fn yaml(&self, args: &[&str]) -> serde_yaml::Value {
        let mut with_format = args.to_vec();
        with_format.extend(["-F", "yaml"]);
        let output = self.run(&with_format).await;
        let stdout = String::from_utf8(output.stdout).expect("stdout is valid UTF-8");

        assert!(
            output.status.success(),
            "`{}` failed: {}",
            with_format.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );

        serde_yaml::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "`{}` did not write YAML alone to stdout ({e}):\n{stdout}",
                with_format.join(" ")
            )
        })
    }
}

/// A deployable project directory: `_ricochet.toml`, package file and entrypoint.
fn write_project(dir: &Path, toml: &str) {
    std::fs::write(dir.join("_ricochet.toml"), toml).expect("writing _ricochet.toml");
    std::fs::write(dir.join("renv.lock"), r#"{"R":{},"Packages":{}}"#).expect("writing renv.lock");
    std::fs::write(dir.join("app.R"), "shiny::shinyApp(ui, server)").expect("writing app.R");
}

#[tokio::test]
async fn app_list_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "list"]).await;
    assert_eq!(payload[0]["id"], CONTENT_ID);
}

#[tokio::test]
async fn app_list_writes_yaml_alone() {
    let cli = Cli::new().await;
    let payload = cli.yaml(&["app", "list"]).await;
    assert_eq!(
        payload[0]["id"],
        serde_yaml::Value::String(CONTENT_ID.into())
    );
}

#[tokio::test]
async fn task_list_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["task", "list"]).await;
    assert_eq!(payload[0]["content_type"], "quarto-r");
}

#[tokio::test]
async fn app_toml_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "toml", CONTENT_ID]).await;
    assert_eq!(payload["content_id"], CONTENT_ID);
    assert!(
        payload["toml"]
            .as_str()
            .is_some_and(|t| t.contains("[content]"))
    );
}

#[tokio::test]
async fn app_instances_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "instances", CONTENT_ID]).await;
    assert_eq!(payload[0]["instance_id"], INSTANCE_ID);
}

#[tokio::test]
async fn app_stop_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "stop", CONTENT_ID, INSTANCE_ID]).await;
    assert_eq!(payload["stopped"][0], INSTANCE_ID);
}

#[tokio::test]
async fn app_deployment_list_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "deployment", "list", CONTENT_ID]).await;
    assert_eq!(payload[0]["id"], DEPLOYMENT_ID);
}

#[tokio::test]
async fn app_deployment_get_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "deployment", "get", DEPLOYMENT_ID]).await;
    assert_eq!(payload["content_id"], CONTENT_ID);
}

#[tokio::test]
async fn app_env_vars_get_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["app", "env-vars", "get", CONTENT_ID]).await;
    assert_eq!(payload[0], "DATABASE_URL");
}

#[tokio::test]
async fn app_env_vars_set_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli
        .json(&[
            "app",
            "env-vars",
            "set",
            "DATABASE_URL=postgres://localhost",
            "-i",
            CONTENT_ID,
        ])
        .await;
    assert_eq!(payload[0], "DATABASE_URL");
}

#[tokio::test]
async fn app_env_vars_replace_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli
        .json(&[
            "app",
            "env-vars",
            "replace",
            "DATABASE_URL=postgres://localhost",
            "-i",
            CONTENT_ID,
            "-f",
        ])
        .await;
    assert_eq!(payload[0], "DATABASE_URL");
}

#[tokio::test]
async fn app_env_vars_delete_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli
        .json(&[
            "app",
            "env-vars",
            "delete",
            "DATABASE_URL",
            "-i",
            CONTENT_ID,
            "-f",
        ])
        .await;
    assert_eq!(payload, json!([]));
}

#[tokio::test]
async fn app_settings_writes_json_alone() {
    let cli = Cli::new().await;
    let project = TempDir::new().unwrap();
    write_project(project.path(), LOCAL_TOML);
    let toml_path = project.path().join("_ricochet.toml");

    let payload = cli
        .json(&["app", "settings", "-p", toml_path.to_str().unwrap()])
        .await;
    assert_eq!(payload["content"]["name"], "local-app");
}

#[tokio::test]
async fn app_settings_update_writes_json_alone() {
    let cli = Cli::new().await;
    let project = TempDir::new().unwrap();
    write_project(project.path(), LOCAL_TOML);
    let toml_path = project.path().join("_ricochet.toml");

    let payload = cli
        .json(&[
            "app",
            "settings",
            "update",
            "-p",
            toml_path.to_str().unwrap(),
            "-f",
        ])
        .await;
    assert_eq!(payload["content"]["name"], "local-app");
}

#[tokio::test]
async fn task_invoke_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["task", "invoke", CONTENT_ID]).await;
    assert_eq!(payload["status"], "pending");
}

#[tokio::test]
async fn task_schedule_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli
        .json(&["task", "schedule", CONTENT_ID, "0 9 * * 1-5"])
        .await;
    assert_eq!(payload["schedule"], "0 9 * * 1-5");
}

#[tokio::test]
async fn deploy_writes_json_alone() {
    let cli = Cli::new().await;
    let project = TempDir::new().unwrap();
    write_project(project.path(), LOCAL_TOML);

    let payload = cli
        .json(&["deploy", project.path().to_str().unwrap()])
        .await;
    assert_eq!(payload["id"], CONTENT_ID);
    assert_eq!(payload["deployment_id"], DEPLOYMENT_ID);
}

#[tokio::test]
async fn deploy_git_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli
        .json(&["deploy", "--git", "https://github.com/org/repo"])
        .await;
    assert_eq!(payload["id"], CONTENT_ID);
}

#[tokio::test]
async fn delete_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["delete", CONTENT_ID, "-f"]).await;
    assert_eq!(payload["id"], CONTENT_ID);
}

#[tokio::test]
async fn user_credentials_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["user", "credentials"]).await;
    assert_eq!(payload[0]["name"], "deploy-key");
}

#[tokio::test]
async fn config_writes_json_alone() {
    let cli = Cli::new().await;
    let payload = cli.json(&["config"]).await;
    assert!(payload["config_file"].as_str().is_some());
}

#[tokio::test]
async fn server_commands_write_json_alone() {
    let cli = Cli::new().await;

    let listed = cli.json(&["server", "list"]).await;
    assert!(listed.is_array());

    let added = cli
        .json(&["server", "add", "staging", "https://staging.example.com"])
        .await;
    assert_eq!(added["name"], "staging");

    let defaulted = cli.json(&["server", "set-default", "staging"]).await;
    assert_eq!(defaulted["default_server"], "staging");

    let removed = cli.json(&["server", "remove", "staging", "-f"]).await;
    assert_eq!(removed["name"], "staging");
}

#[tokio::test]
async fn login_and_logout_write_json_alone() {
    let cli = Cli::new().await;

    let logout = cli.json(&["logout"]).await;
    assert_eq!(logout["logged_out"], true);

    let login = cli.json(&["login", "-k", "rico_testkey123456"]).await;
    assert_eq!(login["server"], "test");
    assert_eq!(login["api_key"], "rico_tes...3456");
}
