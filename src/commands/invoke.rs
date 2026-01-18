use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};

pub async fn invoke(config: &Config, server_ref: Option<&str>, id: &str, format: OutputFormat) -> Result<()> {
    println!("Invoking task: {}", id.bright_cyan());

    // Resolve server configuration
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new_with_server_config(&server_config)?;

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
                    // Display server URL above the table
                    println!("{}", server_config.url.as_str().italic().dimmed());

                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL);

                    if let Some(invocation_id) =
                        result.get("invocation_id").and_then(|v| v.as_str())
                    {
                        table.add_row(vec![Cell::new("Invocation ID"), Cell::new(invocation_id)]);
                    }

                    if let Some(content_id) = result.get("content_id").and_then(|v| v.as_str()) {
                        table.add_row(vec![Cell::new("Content ID"), Cell::new(content_id)]);
                    }

                    // Add status with color coding
                    if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                        let status_cell = match status {
                            "running" | "success" | "completed" => {
                                Cell::new(status).fg(Color::Green)
                            }
                            "failed" | "error" => Cell::new(status).fg(Color::Red),
                            "pending" | "queued" => Cell::new(status).fg(Color::Yellow),
                            _ => Cell::new(status),
                        };
                        table.add_row(vec![Cell::new("Status"), status_cell]);
                    }

                    println!("{}", table);
                }
            }

            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Failed to invoke task: {e}")
        }
    }
}
