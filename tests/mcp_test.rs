//! Drives `ricochet mcp` over stdio the way an editor's agent does, against a fake Ricochet server.

use axum::Router;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use mockito::{Matcher, Mock, ServerGuard};
use rmcp::model::ErrorCode;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RunningService, ServiceError};
use rmcp::transport::TokioChildProcess;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData as McpError, RoleClient, RoleServer, ServerHandler, ServiceExt};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::process::{ChildStderr, Command};
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(20);
const API_KEY: &str = "test_key";
const ITEMS_TEXT: &str = "one item: example";
const CONTENT_ID: &str = "01K66JV2Q123456789ABCDEF";
const DEPLOYMENT_ID: &str = "01KQZPF4Y5SRHES967VZEYY765";
const REJECTION: &str = "entrypoint app.R is not allowed";

/// A stand-in for the server's MCP endpoint with a single read-only tool.
#[derive(Clone)]
struct FakeServer;

impl ServerHandler for FakeServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let schema = serde_json::json!({ "type": "object", "properties": {} });
        let serde_json::Value::Object(schema) = schema else {
            unreachable!("the schema literal is an object")
        };
        Ok(ListToolsResult::with_all_items(vec![Tool::new(
            "list_items",
            "List the content items you have access to.",
            schema,
        )]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if request.name != "list_items" {
            return Err(McpError::invalid_params("unknown tool", None));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(ITEMS_TEXT)]).into())
    }
}

async fn require_key(request: Request, next: Next) -> Response {
    let expected = format!("Key {API_KEY}");
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .is_some_and(|value| value.as_bytes() == expected.as_bytes());
    if authorized {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[derive(Clone, Copy)]
enum Endpoint {
    Mcp,
    Missing,
}

/// Serve a fake Ricochet server on a local port and return its URL.
async fn start_server(endpoint: Endpoint) -> String {
    let router = match endpoint {
        Endpoint::Mcp => {
            let service = StreamableHttpService::new(
                || Ok(FakeServer),
                Arc::new(NeverSessionManager::default()),
                StreamableHttpServerConfig::default()
                    .with_legacy_session_mode(false)
                    .with_json_response(true)
                    .disable_allowed_hosts(),
            );
            Router::new()
                .nest_service("/api/v0/mcp", service)
                .layer(middleware::from_fn(require_key))
        }
        Endpoint::Missing => Router::new(),
    };
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind a local port");
    let address = listener.local_addr().expect("local address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });
    format!("http://{address}")
}

enum Key {
    Configured,
    /// A key the server refuses.
    Refused,
    Absent,
}

struct McpSession {
    client: RunningService<RoleClient, ()>,
    stderr: ChildStderr,
    /// The child's HOME and working directory.
    home: TempDir,
}

impl McpSession {
    async fn start(server_url: &str, key: Key) -> Self {
        let home = TempDir::new().expect("temp home");
        let mut command = Command::new(env!("CARGO_BIN_EXE_ricochet"));
        command
            .arg("mcp")
            .current_dir(home.path())
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("RICOCHET_SERVER", server_url)
            .env("RICOCHET_NO_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .env_remove("RUST_LOG");
        match key {
            Key::Configured => command.env("RICOCHET_API_KEY", API_KEY),
            Key::Refused => command.env("RICOCHET_API_KEY", "refused_key"),
            Key::Absent => command.env_remove("RICOCHET_API_KEY"),
        };
        let (transport, stderr) = TokioChildProcess::builder(command)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn ricochet mcp");
        let client = timeout(TIMEOUT, ().serve(transport))
            .await
            .expect("initialize within the timeout")
            .expect("initialize");
        Self {
            client,
            stderr: stderr.expect("piped stderr"),
            home,
        }
    }

    async fn tool_names(&self) -> Vec<String> {
        timeout(TIMEOUT, self.client.list_all_tools())
            .await
            .expect("list tools within the timeout")
            .expect("list tools")
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect()
    }

    /// Call a tool with `arguments` and return the server's answer.
    async fn call(
        &self,
        name: &'static str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ServiceError> {
        let serde_json::Value::Object(arguments) = arguments else {
            panic!("tool arguments must be a JSON object")
        };
        timeout(
            TIMEOUT,
            self.client
                .call_tool(CallToolRequestParams::new(name).with_arguments(arguments)),
        )
        .await
        .expect("call within the timeout")
    }

    /// Call a tool that must answer with a result, and return it with its text.
    async fn call_text(&self, name: &'static str, path: &str) -> (CallToolResult, String) {
        let result = self
            .call(name, serde_json::json!({ "path": path }))
            .await
            .unwrap_or_else(|error| panic!("call {name}: {error}"));
        let text = result
            .content
            .iter()
            .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        (result, text)
    }

    /// Close the session and return everything the child wrote to stderr.
    async fn finish(mut self) -> String {
        timeout(TIMEOUT, self.client.cancel())
            .await
            .expect("close within the timeout")
            .expect("close");
        let mut stderr = String::new();
        timeout(TIMEOUT, self.stderr.read_to_string(&mut stderr))
            .await
            .expect("stderr closes within the timeout")
            .expect("read stderr");
        stderr
    }
}

#[tokio::test]
async fn mcp_lists_forwarded_tools() {
    let server = start_server(Endpoint::Mcp).await;
    let session = McpSession::start(&server, Key::Configured).await;
    assert!(
        session
            .tool_names()
            .await
            .contains(&"list_items".to_owned())
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_forwards_tool_calls() {
    let server = start_server(Endpoint::Mcp).await;
    let session = McpSession::start(&server, Key::Configured).await;
    let result = timeout(
        TIMEOUT,
        session
            .client
            .call_tool(CallToolRequestParams::new("list_items")),
    )
    .await
    .expect("call within the timeout")
    .expect("call list_items");
    let texts: Vec<&str> = result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
        .collect();
    assert_eq!(texts, [ITEMS_TEXT]);
    session.finish().await;
}

#[tokio::test]
async fn mcp_starts_without_server_mcp() {
    let server = start_server(Endpoint::Missing).await;
    let session = McpSession::start(&server, Key::Configured).await;
    assert!(
        !session
            .tool_names()
            .await
            .contains(&"list_items".to_owned())
    );
    let stderr = session.finish().await;
    assert!(stderr.contains(&server), "stderr: {stderr}");
}

#[tokio::test]
async fn mcp_starts_without_api_key() {
    let server = start_server(Endpoint::Mcp).await;
    let session = McpSession::start(&server, Key::Absent).await;
    assert!(
        !session
            .tool_names()
            .await
            .contains(&"list_items".to_owned())
    );
    let stderr = session.finish().await;
    assert!(stderr.contains(&server), "stderr: {stderr}");
}

/// A deployable project directory: `_ricochet.toml`, package file and entrypoint.
fn write_project(dir: &Path) {
    std::fs::create_dir_all(dir).expect("creating the project directory");
    std::fs::write(
        dir.join("_ricochet.toml"),
        format!(
            "[content]\nid = \"{CONTENT_ID}\"\nname = \"mcp-app\"\nentrypoint = \"app.R\"\naccess_type = \"external\"\ncontent_type = \"shiny\"\n\n[language]\nname = \"r\"\npackages = \"renv.lock\"\n"
        ),
    )
    .expect("writing _ricochet.toml");
    std::fs::write(dir.join("renv.lock"), r#"{"R":{},"Packages":{}}"#).expect("writing renv.lock");
    std::fs::write(dir.join("app.R"), "shiny::shinyApp(ui, server)").expect("writing app.R");
}

/// How the fake server answers an upload.
enum Upload {
    Accepted,
    Rejected,
}

/// The endpoints a local deploy touches, returned so a test can count the requests that reached them.
struct DeployMocks {
    server: ServerGuard,
    check_key: Mock,
    upload: Mock,
}

impl DeployMocks {
    async fn start(upload: Upload) -> Self {
        let mut server = mockito::Server::new_async().await;
        let check_key = server
            .mock("GET", "/api/v0/check_key")
            .match_header("authorization", format!("Key {API_KEY}").as_str())
            .with_status(200)
            .create_async()
            .await;
        // mockito tries mocks in the order they were created, so only a key other than `API_KEY` reaches this one.
        server
            .mock("GET", "/api/v0/check_key")
            .with_status(401)
            .create_async()
            .await;
        let upload = match upload {
            Upload::Accepted => server
                .mock("POST", "/api/v0/content/upload")
                .match_body(Matcher::Any)
                .with_status(200)
                .with_body(
                    serde_json::json!({
                        "id": CONTENT_ID,
                        "deployment_id": DEPLOYMENT_ID,
                        "status": "deployed"
                    })
                    .to_string(),
                ),
            Upload::Rejected => server
                .mock("POST", "/api/v0/content/upload")
                .match_body(Matcher::Any)
                .with_status(400)
                .with_body(REJECTION),
        }
        .create_async()
        .await;
        Self {
            server,
            check_key,
            upload,
        }
    }

    /// Require that no request reached the server.
    async fn assert_untouched(&self) {
        assert!(
            !self.check_key.matched_async().await,
            "check_key was called"
        );
        assert!(!self.upload.matched_async().await, "upload was called");
    }
}

#[tokio::test]
async fn mcp_lists_local_tools() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let names = session.tool_names().await;
    assert!(names.contains(&"preview_bundle".to_owned()), "{names:?}");
    assert!(names.contains(&"deploy".to_owned()), "{names:?}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_preview_bundle_lists_files() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let project = TempDir::new().expect("temp project");
    write_project(project.path());

    let (result, text) = session
        .call_text(
            "preview_bundle",
            project.path().to_str().expect("UTF-8 path"),
        )
        .await;
    assert_ne!(result.is_error, Some(true), "{text}");
    let preview: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{error}: {text}"));
    let paths: Vec<&str> = preview["files"]
        .as_array()
        .expect("a files array")
        .iter()
        .filter_map(|file| file["path"].as_str())
        .collect();
    assert!(paths.contains(&"app.R"), "{paths:?}");
    mocks.assert_untouched().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_preview_bundle_reports_the_whole_toml_error() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let project = TempDir::new().expect("temp project");
    write_project(project.path());
    std::fs::write(
        project.path().join("_ricochet.toml"),
        "[content]\nname = \"mcp-app\nentrypoint = \"app.R\"\n",
    )
    .expect("writing _ricochet.toml");

    let (result, text) = session
        .call_text(
            "preview_bundle",
            project.path().to_str().expect("UTF-8 path"),
        )
        .await;
    assert_eq!(result.is_error, Some(true), "{text}");
    assert!(text.contains("line 2, column 16"), "{text}");
    assert!(text.contains(r#"name = "mcp-app"#), "{text}");
    assert!(!text.contains("Error: "), "{text}");
    mocks.assert_untouched().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_deploy_returns_done_event() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let project = TempDir::new().expect("temp project");
    write_project(project.path());

    let (result, text) = session
        .call_text("deploy", project.path().to_str().expect("UTF-8 path"))
        .await;
    assert_ne!(result.is_error, Some(true), "{text}");
    let done: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(done["event"], "done");
    assert_eq!(done["deployment_id"], DEPLOYMENT_ID);
    mocks.upload.assert_async().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_deploy_reports_rejection_as_error() {
    let mocks = DeployMocks::start(Upload::Rejected).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let project = TempDir::new().expect("temp project");
    write_project(project.path());

    let (result, text) = session
        .call_text("deploy", project.path().to_str().expect("UTF-8 path"))
        .await;
    assert_eq!(result.is_error, Some(true), "{text}");
    assert!(text.contains("rejected"), "{text}");
    assert!(text.contains(REJECTION), "{text}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_deploy_reports_refused_key_as_error() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Refused).await;
    let project = TempDir::new().expect("temp project");
    write_project(project.path());

    let (result, text) = session
        .call_text("deploy", project.path().to_str().expect("UTF-8 path"))
        .await;
    assert_eq!(result.is_error, Some(true), "{text}");
    assert!(text.contains(r#""kind":"auth""#), "{text}");
    assert!(!mocks.upload.matched_async().await, "upload was called");
    session.finish().await;
}

#[tokio::test]
async fn mcp_local_tools_reject_relative_path() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    // A deployable `proj` beside the server's working directory must still not be deployed.
    write_project(&session.home.path().join("proj"));

    for tool in ["preview_bundle", "deploy"] {
        let (result, text) = session.call_text(tool, "proj").await;
        assert_eq!(result.is_error, Some(true), "{tool}: {text}");
        assert!(text.contains("proj"), "{tool}: {text}");
    }
    mocks.assert_untouched().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_local_tools_reject_missing_path() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let missing = session.home.path().join("missing");
    let missing = missing.to_str().expect("UTF-8 path");

    for tool in ["preview_bundle", "deploy"] {
        let (result, text) = session.call_text(tool, missing).await;
        assert_eq!(result.is_error, Some(true), "{tool}: {text}");
        assert!(text.contains(missing), "{tool}: {text}");
    }
    mocks.assert_untouched().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_local_tools_require_a_path_argument() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let error = session
        .call("deploy", serde_json::json!({}))
        .await
        .expect_err("a call without `path` fails");
    let ServiceError::McpError(error) = error else {
        panic!("expected an MCP error, got {error}")
    };
    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    mocks.assert_untouched().await;
    session.finish().await;
}

#[tokio::test]
async fn mcp_rejects_unknown_tool() {
    let mocks = DeployMocks::start(Upload::Accepted).await;
    let session = McpSession::start(&mocks.server.url(), Key::Configured).await;
    let error = session
        .call("no_such_tool", serde_json::json!({}))
        .await
        .expect_err("an unknown tool fails");
    let ServiceError::McpError(error) = error else {
        panic!("expected an MCP error, got {error}")
    };
    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(error.message.contains("no_such_tool"), "{}", error.message);
    session.finish().await;
}
