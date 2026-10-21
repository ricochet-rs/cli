use crate::commands::log::{LogEnd, print_log};
use crate::{
    OutputFormat,
    client::{DeployFailure, RicochetClient},
    config::Config,
    task::invocation,
};
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

/// A record bracketing the log records of `invoke --follow` for a program.
#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum RunEvent<'a> {
    /// The run the server started, written before its first log record.
    Invoked(&'a Invoked),
    /// The finished run, written after its last log record.
    Done(&'a invocation::Invocation),
    /// Why the run could not be started or followed to its end.
    Error {
        #[serde(flatten)]
        failure: DeployFailure,
        message: String,
    },
}

impl RunEvent<'_> {
    /// Write the event as a record of the stream, which a person following the log does not see.
    fn emit(&self, format: OutputFormat, term: &Term) -> Result<()> {
        if format != OutputFormat::Table {
            term.write_line(&format.render_record(self, || Ok(String::new()))?)?;
            term.flush()?;
        }
        Ok(())
    }
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
    let term = Term::buffered_stdout();
    match report_run(config, server_ref, id, follow, format, &term).await {
        Ok(Some(run)) => run.require_success(),
        Ok(None) => Ok(()),
        // A followed stream ends with `done` or `error`, and a failed run already ended with `done`.
        Err(e) => {
            if let Follow::Logs = follow {
                RunEvent::Error {
                    failure: DeployFailure::classify(&e),
                    message: console::strip_ansi_codes(&format!("{e:#}")).into_owned(),
                }
                .emit(format, &term)?;
            }
            Err(e)
        }
    }
}

/// Start a run and report it as `follow` asks, returning the finished run when it was awaited.
async fn report_run(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    follow: Follow,
    format: OutputFormat,
    term: &Term,
) -> Result<Option<invocation::Invocation>> {
    eprintln!("Invoking task: {}", id.bright_cyan());

    // Resolve server configuration
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let invoked = client
        .invoke(id, None)
        .await
        .context("Failed to invoke task")?;
    eprintln!("{} Task invoked successfully!", "✓".green().bold());

    match follow {
        Follow::Detach => format
            .print(&invoked, || {
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
            })
            .map(|()| None),
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
            Ok(Some(run))
        }
        Follow::Logs => {
            eprintln!("Following task run {}", invoked.id.bright_cyan());
            RunEvent::Invoked(&invoked).emit(format, term)?;
            let log_end = print_log(&client, server_ref, &invoked.id, None, format, term)
            .await
            .with_context(|| {
                format!(
                    "Could not follow the logs of task run {run_id}. Check it later with `ricochet task invocation get {id} {run_id}`",
                    run_id = invoked.id
                )
            })?;
            match log_end {
                LogEnd::Closed => {}
                LogEnd::ReaderGone => return Ok(None),
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
            RunEvent::Done(&run).emit(format, term)?;
            Ok(Some(run))
        }
    }
}
