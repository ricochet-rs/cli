use crate::commands::log::{LogEnd, print_log};
use crate::{OutputFormat, client::RicochetClient, config::Config, task::invocation};
use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use console::Term;
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
    Logs,
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
        Follow::Logs => {
            let log_end = print_log(
                &client,
                server_ref,
                &invoked.id,
                None,
                format,
                &Term::buffered_stdout(),
            )
            .await
            .with_context(|| {
                format!(
                    "Could not follow the logs of task run {run_id}. Check it later with `ricochet task invocation get {id} {run_id}`",
                    run_id = invoked.id
                )
            })?;
            match log_end {
                LogEnd::Closed => {}
                LogEnd::ReaderGone => return Ok(()),
                // Ctrl-C is caught only to restore the scrollback, so exit as an uncaught interrupt would.
                LogEnd::Interrupted => std::process::exit(130),
            }

            // A log ends when its process does, which says nothing of whether the run worked.
            let run = tokio::select! {
                run = invocation::wait(&client, id, &invoked.id) => run.with_context(|| {
                    format!(
                        "Lost track of task run {run_id}. Check it later with `ricochet task invocation get {id} {run_id}`",
                        run_id = invoked.id
                    )
                })?,
                // `print_log` caught Ctrl-C, so it no longer ends the process by itself.
                _ = tokio::signal::ctrl_c() => std::process::exit(130),
            };
            run.require_success()
        }
    }
}
