use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;

pub async fn update(config: &Config, id: &str, cron: Option<String>, disable: bool) -> Result<()> {
    let client = RicochetClient::new(config)?;

    if disable {
        println!("⏰ Disabling schedule for: {}", id.bright_cyan());
        client.update_schedule(id, None).await?;
        println!("{} Schedule disabled successfully!", "✓".green().bold());
    } else if let Some(cron_expr) = cron {
        println!("⏰ Updating schedule for: {}", id.bright_cyan());
        println!("   Schedule: {}", cron_expr.yellow());

        // Basic validation of cron expression
        if cron_expr.split_whitespace().count() != 5 {
            anyhow::bail!(
                "Invalid cron expression. Expected 5 fields (minute hour day month weekday)"
            );
        }

        client.update_schedule(id, Some(cron_expr.clone())).await?;
        println!("{} Schedule updated successfully!", "✓".green().bold());

        // Show next run times hint
        println!(
            "\n{}",
            "Tip: The schedule uses standard cron format:".dimmed()
        );
        println!("{}", "  * * * * * = every minute".dimmed());
        println!("{}", "  0 * * * * = every hour".dimmed());
        println!("{}", "  0 0 * * * = daily at midnight".dimmed());
        println!("{}", "  0 0 * * 1 = every Monday at midnight".dimmed());
    } else {
        anyhow::bail!("Please provide either --cron or --disable flag");
    }

    Ok(())
}
