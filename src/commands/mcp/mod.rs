//! An MCP server over stdio that lets an editor's agent use the Ricochet server's tools.
//!
//! The server's own tools come from its MCP endpoint, connected once at startup with the CLI's
//! stored profile. When that fails, `ricochet mcp` still serves, without them.

mod remote;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};

use crate::commands::mcp::remote::RemoteTools;
use crate::config::Config;

/// Serve MCP over stdio until the client closes stdin.
pub async fn serve_mcp(config: &Config, server_ref: Option<&str>) -> anyhow::Result<()> {
    let remote = match config.resolve_server(server_ref) {
        Ok(server) => match RemoteTools::connect(&server).await {
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
    McpServer { remote }
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

/// The tools `ricochet mcp` offers: the server's, forwarded by name.
struct McpServer {
    remote: Option<RemoteTools>,
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
        let tools = self
            .remote
            .as_ref()
            .map(|remote| remote.tools.clone())
            .unwrap_or_default();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        match &self.remote {
            Some(remote) if remote.offers(&request.name) => remote.call(request).await,
            _ => Err(McpError::invalid_params(
                format!("unknown tool {}", request.name),
                None,
            )),
        }
    }
}
