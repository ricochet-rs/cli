use crate::{OutputFormat, client::RicochetClient, config::Config, task::invocation};
use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};

/// The run the server started.
#[derive(Debug, Serialize, Deserialize)]
pub struct Invoked {
    /// Invocation ID
    pub id: String,
    pub content_id: String,
}

/// Whether `invoke` returns once the run starts or once it finishes.
#[derive(Debug, Clone, Copy)]
pub enum Follow {
    Detach,
    UntilFinished,
}

pub async fn invoke(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    follow: Follow,
    format: OutputFormat,
) -> Result<()> {
    eprintln!("Invoking task: {}", id.bright_cyan());

    // Resolve server configuration
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let invoked = client
        .invoke(id, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to invoke task: {e}"))?;
    eprintln!("{} Task invoked successfully!", "✓".green().bold());

    match follow {
        Follow::Detach => format.print(&invoked, || {
            let mut table = Table::new();
            table.load_style(UTF8_FULL);
            table.add_row(vec![Cell::new("Invocation ID"), Cell::new(&invoked.id)]);
            table.add_row(vec![
                Cell::new("Content ID"),
                Cell::new(&invoked.content_id),
            ]);

            Ok(format!(
                "{}\n{table}",
                server_config.url.as_str().italic().dimmed()
            ))
        }),
        Follow::UntilFinished => {
            let run = invocation::wait(&client, id, &invoked.id)
                .await
                .with_context(|| {
                    format!(
                        "Lost track of task run {run_id}. Check it later with `ricochet task invocation get {id} {run_id}`",
                        run_id = invoked.id
                    )
                })?;
            run.print(format, server_config.url.as_str())?;
            run.require_success()
        }
    }
}
