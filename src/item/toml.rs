use crate::{client::RicochetClient, config::Config, item::resolve_id};
use std::path::PathBuf;

pub async fn get_toml(
    config: &Config,
    id: Option<String>,
    path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let server_config = config.resolve_server(None)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let id = resolve_id(id.as_deref(), path.as_deref())?;

    println!("{}", client.get_ricochet_toml(&id).await?);
    Ok(())
}
