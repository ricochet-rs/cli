//! An MCP server over stdio that lets an editor's agent use the Ricochet server's tools.
//!
//! The server's own tools come from its MCP endpoint, connected once at startup with the CLI's
//! stored profile. When that fails, `ricochet mcp` still serves, without them.
//! Beside them, `ricochet mcp` offers its own tools for a project on the local disk.

mod local;

use std::{collections::HashMap, process::Stdio, time::Duration};

use anyhow::Context;
use reqwest::header::{AUTHORIZATION, HeaderValue};
use rmcp::{
    ErrorData as McpError, RoleClient, RoleServer, ServerHandler, ServiceExt,
    handler::server::common::schema_for_type,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
        ToolAnnotations,
    },
    service::{RequestContext, RunningService, ServiceError},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

use crate::{
    commands::mcp::local::{DeployTarget, LocalTool, LocalToolArgs, PREVIEW_TIMEOUT},
    config::Config,
};

/// How long the server may take to accept a connection and answer the handshake and tool list.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Serve MCP over stdio until the client closes stdin.
pub async fn serve_mcp(config: &Config, server_ref: Option<&str>) -> anyhow::Result<()> {
    let target = config.resolve_server(server_ref);
    let remote = match &target {
        Ok(server) => {
            // Connect to `{server}/api/v0/mcp` with the server's API key and fetch its tool list.
            let connected = async {
                let api_key = server
                    .api_key
                    .as_deref()
                    .context("no API key configured, run `ricochet login` to add one")?;
                let mut url = server.url.clone();
                url.set_path("/api/v0/mcp");
                let authorization = HeaderValue::from_str(&format!("Key {api_key}"))
                    .context("the API key is not a valid header value")?;
                let client = reqwest::Client::builder()
                    .connect_timeout(CONNECT_TIMEOUT)
                    .build()?;
                let transport = StreamableHttpClientTransport::with_client(
                    client,
                    StreamableHttpClientTransportConfig::with_uri(url.as_str())
                        .custom_headers(HashMap::from([(AUTHORIZATION, authorization)])),
                );
                tokio::time::timeout(CONNECT_TIMEOUT, async {
                    let peer = ().serve(transport).await?;
                    let tools = peer.list_all_tools().await?;
                    anyhow::Ok(RemoteTools { tools, peer })
                })
                .await
                .with_context(|| format!("{url} did not answer within {CONNECT_TIMEOUT:?}"))?
            }
            .await;
            match connected {
                Ok(remote) => Some(remote),
                Err(error) => {
                    tracing::warn!(
                        server = %server.url,
                        error = format!("{error:#}"),
                        "serving without the server's tools, its MCP endpoint is unavailable"
                    );
                    None
                }
            }
        }
        Err(error) => {
            tracing::warn!(
                error = format!("{error:#}"),
                "serving without the server's tools, no server is configured"
            );
            None
        }
    };
    // A deploy targets the server resolved here, so that changing the default profile while the
    // editor runs cannot send it somewhere the remote tools do not read from.
    let target = match target {
        Ok(server) => DeployTarget::Server(server),
        Err(_) => DeployTarget::Unresolved(server_ref.map(str::to_owned)),
    };
    McpServer { remote, target }
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

/// The tools the server's MCP endpoint offers, and the session that calls them.
struct RemoteTools {
    tools: Vec<Tool>,
    peer: RunningService<RoleClient, ()>,
}

/// The tools `ricochet mcp` offers: the server's, forwarded by name, and its own local ones.
struct McpServer {
    remote: Option<RemoteTools>,
    target: DeployTarget,
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ricochet", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // A local tool answers to its name first, so a server tool of the same name is unreachable.
        let tools = self
            .remote
            .iter()
            .flat_map(|remote| remote.tools.iter())
            .filter(|tool| LocalTool::from_name(&tool.name).is_none())
            .cloned()
            .chain(LocalTool::ALL.map(|tool| {
                let (description, annotations) = match tool {
                    LocalTool::PreviewBundle => (
                        "List the files a deploy of a local project directory would bundle and the ones it would leave out, without contacting a server. `path` is the absolute path of the directory holding `_ricochet.toml`.",
                        ToolAnnotations::new().read_only(true),
                    ),
                    LocalTool::Deploy => (
                        "Deploy a local project directory to the ricochet server and wait until the server accepts the bundle. `path` is the absolute path of the directory holding `_ricochet.toml`.",
                        ToolAnnotations::new()
                            .read_only(false)
                            .destructive(true)
                            .open_world(true),
                    ),
                };
                Tool::new(tool.name(), description, schema_for_type::<LocalToolArgs>())
                    .with_annotations(annotations)
            }))
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if let Some(tool) = LocalTool::from_name(&request.name) {
            let arguments = serde_json::Value::Object(request.arguments.unwrap_or_default());
            let args: LocalToolArgs = serde_json::from_value(arguments).map_err(|error| {
                McpError::invalid_params(
                    format!("invalid arguments for {}: {error}", request.name),
                    None,
                )
            })?;
            let path = args.path.as_path();
            let result = async {
                // A relative path would resolve against wherever the editor started this server, not the project.
                if !path.is_absolute() {
                    anyhow::bail!(
                        "`path` must be an absolute path, but {} is relative",
                        path.display()
                    );
                }
                let metadata = tokio::fs::metadata(path)
                    .await
                    .with_context(|| format!("cannot read {}", path.display()))?;
                if !metadata.is_dir() {
                    anyhow::bail!("{} is not a directory", path.display());
                }

                let exe = std::env::current_exe().context("cannot locate the ricochet binary")?;
                let mut command = tokio::process::Command::new(exe);
                match &self.target {
                    DeployTarget::Server(server) => {
                        command.env("RICOCHET_SERVER", server.url.as_str());
                        match &server.api_key {
                            Some(key) => command.env("RICOCHET_API_KEY", key),
                            None => command.env_remove("RICOCHET_API_KEY"),
                        };
                    }
                    DeployTarget::Unresolved(Some(server_ref)) => {
                        command.args(["--server", server_ref]);
                    }
                    DeployTarget::Unresolved(None) => {}
                }
                command.arg("deploy").arg(path).args(["-F", "json"]);
                if let LocalTool::PreviewBundle = tool {
                    command.arg("--dry-run");
                }
                command
                    .stdin(Stdio::null())
                    .env("RICOCHET_NON_INTERACTIVE", "1")
                    .env("RICOCHET_NO_UPDATE_CHECK", "1")
                    .env("NO_COLOR", "1")
                    .kill_on_drop(true);

                let output = match tool {
                    LocalTool::PreviewBundle => {
                        tokio::time::timeout(PREVIEW_TIMEOUT, command.output())
                            .await
                            .with_context(|| {
                                format!(
                                    "listing the bundle of {} took longer than {PREVIEW_TIMEOUT:?}",
                                    path.display()
                                )
                            })?
                    }
                    LocalTool::Deploy => command.output().await,
                }
                .context("cannot run ricochet deploy")?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stderr = stderr.trim();
                // `deploy -F json` ends its stdout with a `done` or `error` event.
                let last_line = stdout.lines().last();

                Ok(match (output.status.success(), tool) {
                    (true, LocalTool::PreviewBundle) => {
                        CallToolResult::success(vec![ContentBlock::text(stdout.into_owned())])
                    }
                    (true, LocalTool::Deploy) => {
                        let done = last_line.unwrap_or_default();
                        // A successful deploy's stderr carries warnings the agent must act on,
                        // such as an ID that could not be recorded in `_ricochet.toml`.
                        let text = if stderr.is_empty() {
                            done.to_owned()
                        } else {
                            format!("{done}\n\nricochet deploy also reported:\n{stderr}")
                        };
                        CallToolResult::success(vec![ContentBlock::text(text)])
                    }
                    (false, _) => {
                        let reason = stderr.strip_prefix("Error: ").unwrap_or(stderr);
                        let text = match last_line {
                            Some(event) => event.to_owned(),
                            None if reason.is_empty() => {
                                format!("ricochet deploy failed: {}", output.status)
                            }
                            None => format!("ricochet deploy failed: {reason}"),
                        };
                        CallToolResult::error(vec![ContentBlock::text(text)])
                    }
                })
            }
            .await;
            // Report any failure as a tool error the agent can read.
            let result = result.unwrap_or_else(|error| {
                CallToolResult::error(vec![ContentBlock::text(format!("{error:#}"))])
            });
            return Ok(result.into());
        }
        match &self.remote {
            Some(remote) if remote.tools.iter().any(|tool| tool.name == request.name) => remote
                .peer
                .call_tool_once(request)
                .await
                .map_err(|error| match error {
                    ServiceError::McpError(error) => error,
                    error => McpError::internal_error(
                        format!("the server did not answer: {error}"),
                        None,
                    ),
                }),
            _ => Err(McpError::invalid_params(
                format!("unknown tool {}", request.name),
                None,
            )),
        }
    }
}
