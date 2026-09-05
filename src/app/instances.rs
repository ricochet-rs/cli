use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use jiff::Timestamp;
use serde::Serialize;
use std::path::Path;

use crate::{OutputFormat, client::RicochetClient, config::Config, item::resolve_id, utils};

pub async fn list_instances(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<&str>,
    path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let id = resolve_id(id, path)?;
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let mut instances = client.list_instances(&id).await?;

    if let Some(arr) = instances.as_array_mut() {
        for instance in arr.iter_mut() {
            if let Some(ts) = instance.get("last_connection").and_then(|v| v.as_i64()) {
                let formatted = if ts == 0 {
                    "never".to_string()
                } else {
                    Timestamp::from_millisecond(ts)
                        .map(|t| t.to_string())
                        .unwrap_or_else(|_| ts.to_string())
                };
                instance["last_connection"] = serde_json::Value::String(formatted);
            }
        }
    }

    format.print(&instances, || {
        let mut output = server_config.url.as_str().italic().dimmed().to_string();

        let Some(arr) = instances.as_array().filter(|arr| !arr.is_empty()) else {
            output.push_str(&format!("\n{}", "No instances found".yellow()));
            return Ok(output);
        };

        let mut table = Table::new();
        table.load_style(UTF8_FULL);
        table.set_header(vec![
            "Instance ID",
            "Connections",
            "Started",
            "Last Connection",
        ]);

        for instance in arr {
            let pid = instance
                .get("instance_id")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let connections = instance
                .get("connections")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_string());
            let started = instance
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(utils::format_timestamp)
                .unwrap_or_else(|| "-".to_string());
            let last_conn = instance
                .get("last_connection")
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            let conn_cell = if connections == "0" {
                Cell::new(&connections).fg(Color::DarkGrey)
            } else {
                Cell::new(&connections).fg(Color::Green)
            };

            table.add_row(vec![
                Cell::new(pid),
                conn_cell,
                Cell::new(started),
                Cell::new(last_conn),
            ]);
        }

        output.push_str(&format!("\n{table}\n\n{} instance(s)", arr.len()));
        Ok(output)
    })
}

/// The instances a `stop` call brought down.
#[derive(Serialize)]
struct StoppedInstances {
    content_id: String,
    stopped: Vec<String>,
}

pub async fn stop_instance(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<&str>,
    pid: Option<&str>,
    path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let id = resolve_id(id, path)?;
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let targets = match pid {
        Some(pid) => vec![pid.to_string()],
        None => {
            let instances = client.list_instances(&id).await?;
            let arr = instances
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Unexpected response format"))?;
            arr.iter()
                .filter_map(|instance| instance.get("instance_id").and_then(|v| v.as_str()))
                .map(str::to_string)
                .collect()
        }
    };

    // Report what came down before surfacing a failure, so a partial stop
    // leaves a record of which instances the caller no longer has to stop.
    let mut succeeded = Vec::new();
    let mut failure = None;
    for target in targets {
        if let Err(e) = client.stop_instance(&id, &target).await {
            failure = Some((target, e));
            break;
        }
        succeeded.push(target);
    }

    let stopped = StoppedInstances {
        content_id: id,
        stopped: succeeded,
    };

    format.print(&stopped, || {
        if stopped.stopped.is_empty() {
            let message = match failure {
                Some(_) => "No instances stopped",
                None => "No instances to stop",
            };
            return Ok(message.yellow().to_string());
        }
        Ok(stopped
            .stopped
            .iter()
            .map(|pid| {
                format!(
                    "{} Instance {} stopped",
                    "✓".green().bold(),
                    pid.bright_cyan()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    })?;

    match failure {
        Some((target, e)) => Err(e).with_context(|| format!("stopping instance {target}")),
        None => Ok(()),
    }
}
