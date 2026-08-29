use crate::{OutputFormat, client::RicochetClient, config::Config, item::resolve_id};
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

/// The remote `_ricochet.toml` of a content item.
#[derive(Serialize)]
struct RemoteToml {
    content_id: String,
    toml: String,
}

pub async fn get_toml(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<String>,
    path: Option<PathBuf>,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let id = resolve_id(id.as_deref(), path.as_deref())?;
    let remote = RemoteToml {
        toml: client.get_ricochet_toml(&id).await?,
        content_id: id,
    };

    format.print(&remote, || Ok(remote.toml.clone()))
}
