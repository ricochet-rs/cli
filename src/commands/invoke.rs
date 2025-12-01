use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;

pub async fn invoke(config: &Config, id: &str, format: OutputFormat) -> Result<()> {
    println!("Invoking task: {}", id.bright_cyan());

    let client = RicochetClient::new(config)?;

    match client.invoke(id, None).await {
        Ok(result) => {
            println!("{} Task invoked successfully!\n", "✓".green().bold());

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&result)?);
                }
                OutputFormat::Table => {
                    // For table format, just print the JSON pretty
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }

            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Failed to invoke task: {}", e)
        }
    }
}
