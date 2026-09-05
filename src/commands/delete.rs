use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

/// The content item a `delete` call removed.
#[derive(Serialize)]
struct DeletedItem {
    id: String,
}

pub async fn delete(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    force: bool,
    format: OutputFormat,
) -> Result<()> {
    if !force {
        let message = format!("Are you sure you want to delete content item '{id}'?");
        if !utils::confirm(&message)? {
            eprintln!("{}", "Deletion cancelled".yellow());
            return Ok(());
        }
    }

    eprintln!("🗑  Deleting content item: {}", id.bright_cyan());

    // Resolve server configuration
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;

    match client.delete(id).await {
        Ok(()) => {
            let deleted = DeletedItem { id: id.to_string() };
            format.print(&deleted, || {
                Ok(format!(
                    "{} Content item deleted successfully!",
                    "✓".green().bold()
                ))
            })
        }
        Err(e) => {
            anyhow::bail!("Failed to delete content item: {}", e)
        }
    }
}
