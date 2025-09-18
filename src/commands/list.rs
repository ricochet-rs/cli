use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};

pub async fn list(
    config: &Config,
    content_type: Option<String>,
    active_only: bool,
    format: OutputFormat,
) -> Result<()> {
    let client = RicochetClient::new(config)?;

    let items = client.list_items().await?;

    // Filter items if needed
    let filtered_items: Vec<_> = items
        .iter()
        .filter(|item| {
            if let Some(ref ct) = content_type {
                item.get("content_type")
                    .and_then(|v| v.as_str())
                    .map(|t| t == ct)
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .filter(|item| {
            if active_only {
                item.get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "deployed" || s == "running")
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&filtered_items)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&filtered_items)?);
        }
        OutputFormat::Table => {
            if filtered_items.is_empty() {
                println!("{}", "No content items found".yellow());
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "Type", "Status", "Updated"]);

            for item in &filtered_items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                let content_type = item
                    .get("content_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("-");
                let updated = item
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .map(utils::format_timestamp)
                    .unwrap_or("-".to_string());

                let status_colored = match status {
                    "deployed" | "running" => status.green().to_string(),
                    "failed" => status.red().to_string(),
                    "stopped" => status.yellow().to_string(),
                    _ => status.to_string(),
                };

                table.add_row(vec![
                    utils::truncate_string(id, 12),
                    utils::truncate_string(name, 30),
                    content_type.to_string(),
                    status_colored,
                    updated,
                ]);
            }

            println!("{}", table);
            println!(
                "\n{} {} items",
                filtered_items.len(),
                if active_only { "active" } else { "total" }
            );
        }
    }

    Ok(())
}
