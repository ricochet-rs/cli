use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use ricochet_core::events::DeploymentStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentRow {
    pub id: String,
    pub content_id: String,
    pub deployed_at: i64,
    pub status: DeploymentStatus,
    pub deployed_by: String,
    pub ip_address: String,
    pub requested_ver: Option<String>,
    pub matched_ver: Option<String>,
    pub git_hash: Option<String>,
}

const DEFAULT_FIELDS: &[&str] = &["id", "status", "deployed_at"];

const ALL_FIELDS: &[&str] = &[
    "id",
    "content_id",
    "status",
    "deployed_at",
    "deployed_by",
    "requested_ver",
    "matched_ver",
    "git_hash",
];

fn format_deployed_at(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn status_cell(status: &DeploymentStatus) -> Cell {
    let label = status.to_string();
    match status {
        DeploymentStatus::Success => Cell::new(label).fg(Color::Green),
        DeploymentStatus::Failure | DeploymentStatus::Timeout => Cell::new(label).fg(Color::Red),
        DeploymentStatus::Cancelled | DeploymentStatus::Invalidated => {
            Cell::new(label).fg(Color::Yellow)
        }
        DeploymentStatus::Pending => Cell::new(label).fg(Color::Cyan),
    }
}

fn resolve_fields(fields: Option<Vec<String>>) -> Vec<&'static str> {
    match fields {
        None => DEFAULT_FIELDS.to_vec(),
        Some(f) if f.len() == 1 && f[0] == "all" => ALL_FIELDS.to_vec(),
        Some(f) => ALL_FIELDS
            .iter()
            .copied()
            .filter(|col| f.iter().any(|s| s == col))
            .collect(),
    }
}

fn field_header(field: &str) -> &'static str {
    match field {
        "id" => "ID",
        "content_id" => "Content ID",
        "status" => "Status",
        "deployed_at" => "Deployed At",
        "deployed_by" => "Deployed By",
        "requested_ver" => "Requested Ver",
        "matched_ver" => "Matched Ver",
        "git_hash" => "Git Hash",
        _ => "Unknown",
    }
}

fn field_cell(field: &str, d: &DeploymentRow) -> Cell {
    match field {
        "id" => Cell::new(&d.id),
        "content_id" => Cell::new(&d.content_id),
        "status" => status_cell(&d.status),
        "deployed_at" => Cell::new(format_deployed_at(d.deployed_at)),
        "deployed_by" => Cell::new(&d.deployed_by),
        "requested_ver" => Cell::new(d.requested_ver.as_deref().unwrap_or("-")),
        "matched_ver" => Cell::new(d.matched_ver.as_deref().unwrap_or("-")),
        "git_hash" => Cell::new(d.git_hash.as_deref().unwrap_or("-")),
        _ => Cell::new("-"),
    }
}

pub async fn list_deployments(
    config: &Config,
    server_ref: Option<&str>,
    content_ulid: &str,
    fields: Option<Vec<String>>,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let deployments = client.list_deployments(content_ulid).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&deployments)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&deployments)?);
        }
        OutputFormat::Table => {
            println!("{}", server_config.url.as_str().italic().dimmed());

            if deployments.is_empty() {
                println!("{}", "No deployments found.".yellow());
                return Ok(());
            }

            let cols = resolve_fields(fields);

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(cols.iter().map(|c| field_header(c)));

            for d in &deployments {
                table.add_row(cols.iter().map(|c| field_cell(c, d)));
            }

            println!("{}", table);
            println!("\n{} deployments", deployments.len());
        }
    }

    Ok(())
}

pub async fn get_deployment(
    config: &Config,
    server_ref: Option<&str>,
    deployment_ulid: &str,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let d = client.get_deployment(deployment_ulid).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&d)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&d)?);
        }
        OutputFormat::Table => {
            println!("{}", server_config.url.as_str().italic().dimmed());

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);

            for col in ALL_FIELDS {
                table.add_row(vec![Cell::new(field_header(col)), field_cell(col, &d)]);
            }

            println!("{}", table);
        }
    }

    Ok(())
}
