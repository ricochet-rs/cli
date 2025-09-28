use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use std::cmp::Ordering;

// Helper function to compare items by a specific field
fn compare_by_field(a: &serde_json::Value, b: &serde_json::Value, field: &str) -> Ordering {
    let a_val = match field {
        "status" => a.get("status")
            .or_else(|| a.get("deployment_status"))
            .or_else(|| a.get("last_deployment_status")),
        _ => a.get(field),
    };
    
    let b_val = match field {
        "status" => b.get("status")
            .or_else(|| b.get("deployment_status"))
            .or_else(|| b.get("last_deployment_status")),
        _ => b.get(field),
    };

    match (a_val, b_val) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater, // None values sort last
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            // Try to compare as strings
            if let (Some(a_str), Some(b_str)) = (a.as_str(), b.as_str()) {
                a_str.to_lowercase().cmp(&b_str.to_lowercase())
            } else if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
                // Try to compare as numbers
                a_num.partial_cmp(&b_num).unwrap_or(Ordering::Equal)
            } else {
                // Fall back to string representation
                a.to_string().cmp(&b.to_string())
            }
        }
    }
}

pub async fn list(
    config: &Config,
    content_type: Option<String>,
    active_only: bool,
    sort_fields: Option<String>,
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
                    .or_else(|| item.get("deployment_status"))
                    .or_else(|| item.get("last_deployment_status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s == "deployed" || s == "running" || s == "success")
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    // Apply sorting if requested
    let mut sorted_items = filtered_items;
    if let Some(sort_str) = sort_fields {
        // Parse sort fields (comma-separated, prefix with - for descending)
        let sort_specs: Vec<(String, bool)> = sort_str
            .split(',')
            .map(|s| {
                let s = s.trim();
                if let Some(field) = s.strip_prefix('-') {
                    (field.to_lowercase(), false) // descending
                } else {
                    (s.to_lowercase(), true) // ascending
                }
            })
            .collect();

        sorted_items.sort_by(|a, b| {
            for (field, ascending) in &sort_specs {
                let cmp = compare_by_field(a, b, field);
                if cmp != Ordering::Equal {
                    return if *ascending { cmp } else { cmp.reverse() };
                }
            }
            Ordering::Equal
        });
    }

    let filtered_items = sorted_items;

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
