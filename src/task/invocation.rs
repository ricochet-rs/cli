//! Read a task run, and follow one until it finishes.

use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Consecutive failed status checks after which `wait` gives up.
const MAX_FAILED_CHECKS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStatus {
    /// Queued or running
    Pending,
    Cancelled,
    Success,
    Failure,
}

impl InvocationStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Cancelled => "cancelled",
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }

    fn cell(self) -> Cell {
        let cell = Cell::new(self.label());
        match self {
            Self::Pending => cell.fg(Color::Cyan),
            Self::Cancelled => cell.fg(Color::Yellow),
            Self::Success => cell.fg(Color::Green),
            Self::Failure => cell.fg(Color::Red),
        }
    }
}

/// One run of a task.
#[derive(Debug, Serialize, Deserialize)]
pub struct Invocation {
    pub id: String,
    /// Seconds since the UNIX epoch
    pub invoked_at: i64,
    pub status: InvocationStatus,
    /// Absent while the run is queued
    pub started: Option<i64>,
    /// Absent until the run exits
    pub ended: Option<i64>,
    pub deployment_id: Option<String>,
    pub display_name: Option<String>,
    pub invoked_by: String,
}

impl Invocation {
    fn table(&self) -> Table {
        let format_time = |ts: i64| {
            jiff::Timestamp::from_second(ts)
                .map(|t| t.strftime("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|_| ts.to_string())
        };
        let optional_time = |ts: Option<i64>| ts.map(format_time).unwrap_or_else(|| "-".into());

        let mut table = Table::new();
        table.load_style(UTF8_FULL);
        table.add_row(vec![Cell::new("Invocation ID"), Cell::new(&self.id)]);
        table.add_row(vec![Cell::new("Status"), self.status.cell()]);
        table.add_row(vec![
            Cell::new("Invoked At"),
            Cell::new(format_time(self.invoked_at)),
        ]);
        table.add_row(vec![
            Cell::new("Started"),
            Cell::new(optional_time(self.started)),
        ]);
        table.add_row(vec![
            Cell::new("Ended"),
            Cell::new(optional_time(self.ended)),
        ]);
        table.add_row(vec![
            Cell::new("Invoked By"),
            Cell::new(self.display_name.as_deref().unwrap_or(&self.invoked_by)),
        ]);
        if let Some(deployment_id) = &self.deployment_id {
            table.add_row(vec![Cell::new("Deployment ID"), Cell::new(deployment_id)]);
        }
        table
    }

    pub fn print(&self, format: OutputFormat, server_url: &str) -> Result<()> {
        format.print(self, || {
            Ok(format!(
                "{}\n{}",
                server_url.italic().dimmed(),
                self.table()
            ))
        })
    }

    /// Fail unless the run succeeded, so a calling script stops.
    pub fn require_success(&self) -> Result<()> {
        match self.status {
            InvocationStatus::Success => Ok(()),
            status => anyhow::bail!("Task run {} finished as {}", self.id, status.label()),
        }
    }
}

/// Poll a run until it leaves `pending`.
pub async fn wait(client: &RicochetClient, id: &str, invocation_id: &str) -> Result<Invocation> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}")?);
    let waiting = format!("Waiting for task run {invocation_id} to finish");
    spinner.set_message(waiting.clone());
    spinner.enable_steady_tick(Duration::from_millis(100));

    let mut failed_checks = 0;
    let run = loop {
        match client.get_invocation(id, invocation_id).await {
            Ok(run) if run.status != InvocationStatus::Pending => break run,
            Ok(_) => {
                failed_checks = 0;
                spinner.set_message(waiting.clone());
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(e) if is_transient(&e) && failed_checks + 1 < MAX_FAILED_CHECKS => {
                failed_checks += 1;
                spinner.set_message(format!("{waiting} (retrying after: {e})"));
                tokio::time::sleep(POLL_INTERVAL * 2u32.pow(failed_checks - 1)).await;
            }
            Err(e) => {
                spinner.finish_and_clear();
                return Err(e);
            }
        }
    };
    spinner.finish_and_clear();
    Ok(run)
}

/// Whether a failed status check may succeed when repeated: the server was unreachable or erred.
fn is_transient(error: &anyhow::Error) -> bool {
    error.downcast_ref::<reqwest::Error>().is_some_and(|e| {
        e.is_connect()
            || e.is_timeout()
            || e.is_request()
            || e.status().is_some_and(|s| s.is_server_error())
    })
}

/// Show one run of a task, finished or not.
pub async fn get(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    invocation_id: &str,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;
    client
        .get_invocation(id, invocation_id)
        .await?
        .print(format, server_config.url.as_str())
}
