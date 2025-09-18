use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;

pub async fn update(config: &Config, id: &str, settings_json: &str) -> Result<()> {
    println!("⚙️  Updating settings for: {}", id.bright_cyan());

    // Validate JSON
    let _: serde_json::Value =
        serde_json::from_str(settings_json).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let client = RicochetClient::new(config)?;

    match client.update_settings(id, settings_json).await {
        Ok(()) => {
            println!("{} Settings updated successfully!", "✓".green().bold());
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Failed to update settings: {}", e)
        }
    }
}
