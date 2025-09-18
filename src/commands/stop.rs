use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;

pub async fn stop(config: &Config, id: &str, instance: Option<String>) -> Result<()> {
    let client = RicochetClient::new(config)?;

    if let Some(instance_id) = instance {
        // Check if it's a PID (numeric) or invocation ID
        if instance_id.parse::<u32>().is_ok() {
            println!(
                "⏹  Stopping service instance: {} (PID: {})",
                id.bright_cyan(),
                instance_id
            );
            client.stop_instance(id, &instance_id).await?;
            println!(
                "{} Service instance stopped successfully!",
                "✓".green().bold()
            );
        } else {
            println!(
                "⏹  Stopping invocation: {} (ID: {})",
                id.bright_cyan(),
                instance_id
            );
            client.stop_invocation(id, &instance_id).await?;
            println!("{} Invocation stopped successfully!", "✓".green().bold());
        }
    } else {
        // Try to get active invocations/instances and prompt user
        println!(
            "{}",
            "No instance specified. Please provide --instance flag".yellow()
        );
        println!(
            "\n{}",
            "Tip: Use 'ricochet status' to see active instances".dimmed()
        );
        return Ok(());
    }

    Ok(())
}
