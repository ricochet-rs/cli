//! An MCP server over stdio that lets an editor's agent use the Ricochet server's tools.
//!
//! The server's own tools come from its MCP endpoint, connected once at startup with the CLI's
//! stored profile. When that fails, `ricochet mcp` still serves, without them.
//! Beside them, `ricochet mcp` offers its own tools for a project on the local disk.

mod local;
mod remote;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};

use crate::commands::mcp::local::{DeployTarget, LocalTool, LocalToolArgs};
use crate::commands::mcp::remote::RemoteTools;
use crate::config::Config;

/// Serve MCP over stdio until the client closes stdin.
pub async fn serve_mcp(config: &Config, server_ref: Option<&str>) -> anyhow::Result<()> {
    let target = config.resolve_server(server_ref);
    let remote = match &target {
        Ok(server) => match RemoteTools::connect(server).await {
            Ok(remote) => Some(remote),
            Err(error) => {
                tracing::warn!(
                    server = %server.url,
                    error = format!("{error:#}"),
                    "serving without the server's tools, its MCP endpoint is unavailable"
                );
                None
            }
        },
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
            .chain(LocalTool::ALL.map(LocalTool::definition))
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
            return Ok(tool.call(args, &self.target).await.into());
        }
        match &self.remote {
            Some(remote) if remote.offers(&request.name) => remote.call(request).await,
            _ => Err(McpError::invalid_params(
                format!("unknown tool {}", request.name),
                None,
            )),
        }
    }
}
