use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};

pub async fn status(config: &Config, id: &str, format: OutputFormat) -> Result<()> {
    let client = RicochetClient::new(config)?;

    let deployments = client.get_status(id).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&deployments)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&deployments)?);
        }
        OutputFormat::Table => {
            println!("📊 Status for content item: {}\n", id.bright_cyan());

            if let Some(deps) = deployments.as_array() {
                if deps.is_empty() {
                    println!("{}", "No deployments found".yellow());
                    return Ok(());
                }

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.set_header(vec![
                    "Deployment ID",
                    "Version",
                    "Status",
                    "Created",
                    "Message",
                ]);

                for dep in deps {
                    let dep_id = dep.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                    let version = dep
                        .get("version")
                        .and_then(|v| v.as_i64())
                        .map(|v| v.to_string())
                        .unwrap_or("-".to_string());
                    let status = dep.get("status").and_then(|v| v.as_str()).unwrap_or("-");
                    let created = dep
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .map(utils::format_timestamp)
                        .unwrap_or("-".to_string());
                    let message = dep.get("message").and_then(|v| v.as_str()).unwrap_or("");

                    let status_colored = match status {
                        "deployed" | "success" => status.green().to_string(),
                        "failed" => status.red().to_string(),
                        "pending" => status.yellow().to_string(),
                        _ => status.to_string(),
                    };

                    table.add_row(vec![
                        utils::truncate_string(dep_id, 12),
                        version,
                        status_colored,
                        created,
                        utils::truncate_string(message, 40),
                    ]);
                }

                println!("{}", table);
            }
        }
    }

    Ok(())
}
