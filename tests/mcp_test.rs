//! Drives `ricochet mcp` over stdio the way an editor's agent does, against a fake Ricochet server.

use axum::Router;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RunningService};
use rmcp::transport::TokioChildProcess;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData as McpError, RoleClient, RoleServer, ServerHandler, ServiceExt};
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
    Absent,
}

struct McpSession {
    client: RunningService<RoleClient, ()>,
    stderr: ChildStderr,
    _home: TempDir,
}

impl McpSession {
    async fn start(server_url: &str, key: Key) -> Self {
        let home = TempDir::new().expect("temp home");
        let mut command = Command::new(env!("CARGO_BIN_EXE_ricochet"));
        command
            .arg("mcp")
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("RICOCHET_SERVER", server_url)
            .env("RICOCHET_NO_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .env_remove("RUST_LOG");
        match key {
            Key::Configured => command.env("RICOCHET_API_KEY", API_KEY),
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
            _home: home,
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
