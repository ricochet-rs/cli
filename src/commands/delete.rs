use crate::{client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;

pub async fn delete(config: &Config, id: &str, force: bool) -> Result<()> {
    if !force {
        let message = format!("Are you sure you want to delete content item '{}'?", id);
        if !utils::confirm(&message)? {
            println!("{}", "Deletion cancelled".yellow());
            return Ok(());
        }
    }

    println!("🗑  Deleting content item: {}", id.bright_cyan());

    let client = RicochetClient::new(config)?;

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
