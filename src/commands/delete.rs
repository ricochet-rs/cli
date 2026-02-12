use crate::{client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;

pub async fn delete(config: &Config, server_ref: Option<&str>, id: &str, force: bool) -> Result<()> {
    if !force {
        let message = format!("Are you sure you want to delete content item '{}'?", id);
        if !utils::confirm(&message)? {
            println!("{}", "Deletion cancelled".yellow());
            return Ok(());
        }
    }

    println!("🗑  Deleting content item: {}", id.bright_cyan());

    // Resolve server configuration
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;

    match client.delete(id).await {
        Ok(()) => {
            println!("{} Content item deleted successfully!", "✓".green().bold());
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Failed to delete content item: {}", e)
        }
    }
}
