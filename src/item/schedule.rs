use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::str::FromStr;

use crate::{OutputFormat, client::RicochetClient, config::Config};

pub async fn schedule_task(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    schedule: &str,
    format: OutputFormat,
) -> Result<()> {
    // validate the cron schedule locally before hitting the API
    let cron = croner::Cron::from_str(schedule).context("parsing cron schedule")?;
    let next = cron
        .find_next_occurrence(&Utc::now(), false)
        .context("computing next occurrence")?;

    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let res = client
        .schedule(id, schedule)
        .await
        .context("sending API request")?;

    format.print(&res, || {
        Ok(format!(
            "{} Schedule updated successfully\n\n  {:<12} {}\n  {:<12} {}\n  {:<12} {} UTC",
            "✓".green().bold(),
            "Schedule:".dimmed(),
            schedule,
            "Runs:".dimmed(),
            cron.describe(),
            "Next run:".dimmed(),
            next.format("%Y-%m-%d %H:%M:%S")
        ))
    })
}
