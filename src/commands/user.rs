use crate::{OutputFormat, client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use ricochet_core::config::git::GitProtocol;

pub async fn list_credentials(
    config: &Config,
    server_ref: Option<&str>,
    user_id: Option<&str>,
    protocol: Option<GitProtocol>,
    format: OutputFormat,
) -> Result<()> {
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let credentials = client.list_credentials(user_id, protocol).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&credentials)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&credentials)?);
        }
        OutputFormat::Table => {
            println!("{}", server_config.url.as_str().italic().dimmed());

            if credentials.is_empty() {
                println!("{}", "No credentials found.".yellow());
                return Ok(());
            }

            let mut table = Table::new();
            table.load_style(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "Type", "User ID"]);
            for cred in &credentials {
                table.add_row(vec![
                    cred.id.as_str(),
                    cred.name.as_str(),
                    &cred.protocol.to_string(),
                    cred.user_id.as_str(),
                ]);
            }

            println!("{}", table);
            println!("\n{} credential(s)", credentials.len());
        }
    }

    Ok(())
}
