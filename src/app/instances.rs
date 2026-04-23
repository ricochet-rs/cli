use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use jiff::Timestamp;

use crate::{OutputFormat, client::RicochetClient, config::Config, utils};

pub async fn list_instances(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let mut instances = client.list_instances(id).await?;

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

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&instances)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&instances)?);
        }
        OutputFormat::Table => {
            println!("{}", server_config.url.as_str().italic().dimmed());

            let Some(arr) = instances.as_array() else {
                println!("{}", "No instances found".yellow());
                return Ok(());
            };

            if arr.is_empty() {
                println!("{}", "No instances found".yellow());
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
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
                    Cell::new(&last_conn),
                ]);
            }

            println!("{}", table);
            println!("\n{} instance(s)", arr.len());
        }
    }

    Ok(())
}

pub async fn stop_instance(
    config: &Config,
    server_ref: Option<&str>,
    id: &str,
    pid: &str,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    client.stop_instance(id, pid).await?;

    println!(
        "{} Instance {} stopped",
        "✓".green().bold(),
        pid.bright_cyan()
    );

    Ok(())
}
