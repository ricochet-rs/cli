use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};

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
            table.set_header(vec!["ID", "Name", "Type", "Language", "Visibility", "Status", "Updated"]);

            for item in &filtered_items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                let content_type = item
                    .get("content_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let language = item
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let visibility = item
                    .get("visibility")
                    .and_then(|v| v.as_str())
                    .unwrap_or("private");
                // Try multiple possible status field names
                let status = item.get("status")
                    .or_else(|| item.get("deployment_status"))
                    .or_else(|| item.get("last_deployment_status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let updated = item
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .map(utils::format_timestamp)
                    .unwrap_or("-".to_string());

                // Create cells with proper coloring using comfy-table's Cell type
                let status_cell = match status {
                    "deployed" | "running" | "success" => Cell::new(status).fg(Color::Green),
                    "failed" | "failure" | "error" => Cell::new(status).fg(Color::Red),
                    "stopped" | "stopping" => Cell::new(status).fg(Color::Yellow),
                    _ => Cell::new(status),
                };

                let visibility_cell = match visibility {
                    "public" => Cell::new(visibility).fg(Color::Green),
                    "private" => Cell::new(visibility).fg(Color::Blue),
                    _ => Cell::new(visibility),
                };

                table.add_row(vec![
                    Cell::new(id),
                    Cell::new(name),
                    Cell::new(content_type),
                    Cell::new(language),
                    visibility_cell,
                    status_cell,
                    Cell::new(updated),
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
