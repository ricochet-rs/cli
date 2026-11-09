//! The connection to the Ricochet server's own MCP endpoint, whose tools `ricochet mcp` forwards.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use reqwest::header::{AUTHORIZATION, HeaderValue};
use rmcp::model::{CallToolRequestParams, CallToolResponse, Tool};
use rmcp::service::{RunningService, ServiceError};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ErrorData as McpError, RoleClient, ServiceExt};

use crate::config::ServerConfig;

/// How long the server may take to accept a connection and answer the handshake and tool list.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// The tools the server's MCP endpoint offers, and the session that calls them.
pub(crate) struct RemoteTools {
    pub(crate) tools: Vec<Tool>,
    peer: RunningService<RoleClient, ()>,
}

impl RemoteTools {
    /// Connect to `{server}/api/v0/mcp` with the server's API key and fetch its tool list.
    pub(crate) async fn connect(server: &ServerConfig) -> anyhow::Result<Self> {
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
            Ok(Self { tools, peer })
        })
        .await
        .with_context(|| format!("{url} did not answer within {CONNECT_TIMEOUT:?}"))?
    }

    /// Whether the server offers a tool with this name.
    pub(crate) fn offers(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool.name == name)
    }

    /// Call a tool on the server and pass its answer through unchanged.
    pub(crate) async fn call(
        &self,
        request: CallToolRequestParams,
    ) -> Result<CallToolResponse, McpError> {
        self.peer
            .call_tool_once(request)
            .await
            .map_err(|error| match error {
                ServiceError::McpError(error) => error,
                error => {
                    McpError::internal_error(format!("the server did not answer: {error}"), None)
                }
            })
    }
}
