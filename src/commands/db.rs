//! `ricochet db doctor` — pull a database schema drift report from a
//! Ricochet server and render it.
//!
//! The endpoint (`GET /api/admin/db/doctor`) requires an admin API key. The
//! server compares its live schema against the build-time snapshot baked
//! into the binary and returns a [`DoctorResponse`]. Empty drift means the
//! database matches the migrations the server was built from.
//!
//! The handler defers to `OutputFormat`:
//!
//! * `Json` / `Yaml` — pass the raw response through `serde_*::to_string_pretty`
//!   so other tooling can pipe it.
//! * `Table` — print a human-readable summary: a healthy banner, or sectioned
//!   tables for phantom tables, missing tables, and per-table shape drift.

use crate::{
    OutputFormat,
    client::{DoctorResponse, RicochetClient},
    config::Config,
};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use ricochet_core::drift::{ColumnDrift, DriftReport, ForeignKeyDrift, IndexDrift, TableDrift};

pub async fn doctor(config: &Config, server_ref: Option<&str>, format: OutputFormat) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;

    let response = client.db_doctor().await?;

    match format {
        OutputFormat::Json => {
            // Re-serialize so output mirrors the wire format byte-for-byte.
            let value = serde_json::json!({
                "healthy": response.healthy,
                "drift": response.drift,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        OutputFormat::Yaml => {
            let value = serde_json::json!({
                "healthy": response.healthy,
                "drift": response.drift,
            });
            println!("{}", serde_yaml::to_string(&value)?);
        }
        OutputFormat::Table => print_table(server_config.url.as_ref(), &response),
    }

    Ok(())
}

fn print_table(server_url: &str, response: &DoctorResponse) {
    println!("{}", server_url.italic().dimmed());

    if response.healthy {
        println!(
            "{} Database schema matches the expected schema for this server build.",
            "✓".green().bold()
        );
        return;
    }

    println!("{} Database schema drift detected.", "✗".red().bold());

    let drift = &response.drift;

    if !drift.phantom_tables.is_empty() {
        print_name_section(
            "Phantom tables (in database, not in expected schema)",
            &drift.phantom_tables,
            Color::Red,
        );
    }

    if !drift.missing_tables.is_empty() {
        print_name_section(
            "Missing tables (in expected schema, not in database)",
            &drift.missing_tables,
            Color::Yellow,
        );
    }

    if !drift.table_drift.is_empty() {
        print_table_drift(&drift.table_drift);
    }

    print_summary(drift);
}

fn print_name_section(title: &str, names: &[String], color: Color) {
    println!("\n{}", title.bold());
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Name"]);
    for name in names {
        table.add_row(vec![Cell::new(name).fg(color)]);
    }
    println!("{table}");
}

fn print_table_drift(table_drift: &std::collections::BTreeMap<String, TableDrift>) {
    println!("\n{}", "Table shape drift".bold());

    for (table_name, drift) in table_drift {
        println!("\n  {}", table_name.cyan().bold());

        if !drift.columns.is_empty() {
            print_column_drift(&drift.columns);
        }
        if !drift.foreign_keys.is_empty() {
            print_fk_drift(&drift.foreign_keys);
        }
        if !drift.indexes.is_empty() {
            print_index_drift(&drift.indexes);
        }
    }
}

fn print_column_drift(items: &[ColumnDrift]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Column", "Drift", "Detail"]);
    for item in items {
        let (name, kind, detail) = match item {
            ColumnDrift::Missing(n) => (n.clone(), "missing", String::new()),
            ColumnDrift::Phantom(n) => (n.clone(), "phantom", String::new()),
            ColumnDrift::Mismatch {
                name,
                expected,
                actual,
            } => {
                let detail = format!(
                    "expected: {} (not_null={}, pk={}, default={:?})  vs  actual: {} (not_null={}, pk={}, default={:?})",
                    expected.sql_type,
                    expected.not_null,
                    expected.primary_key,
                    expected.default_value,
                    actual.sql_type,
                    actual.not_null,
                    actual.primary_key,
                    actual.default_value,
                );
                (name.clone(), "mismatch", detail)
            }
        };
        table.add_row(vec![
            Cell::new(name),
            Cell::new(kind).fg(color_for_kind(kind)),
            Cell::new(detail),
        ]);
    }
    println!("{table}");
}

fn print_fk_drift(items: &[ForeignKeyDrift]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Foreign key", "Drift"]);
    for item in items {
        let (label, kind) = match item {
            ForeignKeyDrift::Missing(fk) => (
                format!("{} -> {}.{}", fk.from_column, fk.to_table, fk.to_column),
                "missing",
            ),
            ForeignKeyDrift::Phantom(fk) => (
                format!("{} -> {}.{}", fk.from_column, fk.to_table, fk.to_column),
                "phantom",
            ),
        };
        table.add_row(vec![
            Cell::new(label),
            Cell::new(kind).fg(color_for_kind(kind)),
        ]);
    }
    println!("{table}");
}

fn print_index_drift(items: &[IndexDrift]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Index", "Drift", "Detail"]);
    for item in items {
        let (name, kind, detail) = match item {
            IndexDrift::Missing(n) => (n.clone(), "missing", String::new()),
            IndexDrift::Phantom(n) => (n.clone(), "phantom", String::new()),
            IndexDrift::Mismatch {
                name,
                expected,
                actual,
            } => {
                let detail = format!(
                    "expected: columns={:?} unique={}  vs  actual: columns={:?} unique={}",
                    expected.columns, expected.unique, actual.columns, actual.unique,
                );
                (name.clone(), "mismatch", detail)
            }
        };
        table.add_row(vec![
            Cell::new(name),
            Cell::new(kind).fg(color_for_kind(kind)),
            Cell::new(detail),
        ]);
    }
    println!("{table}");
}

fn color_for_kind(kind: &str) -> Color {
    match kind {
        "phantom" => Color::Red,
        "missing" => Color::Yellow,
        "mismatch" => Color::Magenta,
        _ => Color::Reset,
    }
}

fn print_summary(drift: &DriftReport) {
    println!(
        "\n{} {} phantom table(s), {} missing table(s), {} table(s) with shape drift",
        "Summary:".bold(),
        drift.phantom_tables.len(),
        drift.missing_tables.len(),
        drift.table_drift.len(),
    );
}
