use crate::{client::RicochetClient, config::Config};
use colored::Colorize;
use ricochet_core::content::ContentItem;
use std::{fs::read_to_string, path::PathBuf};

pub async fn get_toml(
    config: &Config,
    id: Option<String>,
    path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let client = RicochetClient::new(config)?;
    client.preflight_key_check().await?;

    let id = match id {
        Some(id) => id,
        None => {
            let toml_path = path.unwrap_or(PathBuf::from("_ricochet.toml"));
            if !toml_path.exists() {
                anyhow::bail!(
                    "{} Provide either an item ID or a path to a `_ricochet.toml` file.",
                    "⚠".yellow()
                );
            }
            let toml = read_to_string(toml_path)?;
            let item = ContentItem::from_toml(&toml)?;
            let Some(id) = item.content.id else {
                anyhow::bail!("Provided _ricochet.toml does not have an item ID")
            };

            id
        }
    };

    println!("{}", client.get_ricochet_toml(&id).await?);
    Ok(())
}
