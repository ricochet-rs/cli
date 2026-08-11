use crate::{OutputFormat, client::RicochetClient, config::Config, utils};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use ricochet_core::content::ContentItem;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

fn resolve_id(id: Option<&str>, path: Option<&Path>) -> Result<String> {
    match id {
        Some(id) => Ok(id.to_string()),
        None => {
            let toml_path = path
                .map(|p| p.to_path_buf())
                .unwrap_or(PathBuf::from("_ricochet.toml"));
            if !toml_path.exists() {
                anyhow::bail!(
                    "{} Provide either an item ID or a path to a `_ricochet.toml` file.",
                    "⚠".yellow()
                );
            }
            let toml = read_to_string(toml_path)?;
            let item = ContentItem::from_toml(&toml)?;
            let Some(id) = item.content.id else {
                anyhow::bail!("Provided _ricochet.toml does not have an item ID")
            };
            Ok(id)
        }
    }
}

/// Directory to resolve `.env` / `.Renviron` lookups against for a `--env
/// KEY` (no value) entry: the directory containing `path`, if given,
/// otherwise the current directory.
fn env_dir(path: Option<&Path>) -> PathBuf {
    path.and_then(|p| p.parent())
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn print_names(server_url: &str, names: &[String], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&names)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&names)?);
        }
        OutputFormat::Table => {
            println!("{}", server_url.italic().dimmed());

            if names.is_empty() {
                println!("{}", "No environment variables found.".yellow());
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Name"]);
            for name in names {
                table.add_row(vec![name]);
            }

            println!("{}", table);
            println!("\n{} environment variable(s)", names.len());
        }
    }

    Ok(())
}

pub async fn get_env_vars(
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

    let names = client.get_env_vars(&id).await?;

    print_names(server_config.url.as_str(), &names, format)
}

pub async fn delete_env_var(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<&str>,
    path: Option<&Path>,
    name: &str,
    force: bool,
    format: OutputFormat,
) -> Result<()> {
    let id = resolve_id(id, path)?;

    if !force {
        let message = format!("Are you sure you want to delete environment variable '{name}'?");
        if !utils::confirm(&message)? {
            println!("{}", "Deletion cancelled".yellow());
            return Ok(());
        }
    }

    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let names = client.delete_env_var(&id, name).await?;

    println!(
        "{} Environment variable '{}' deleted",
        "✓".green().bold(),
        name.bright_cyan()
    );

    print_names(server_config.url.as_str(), &names, format)
}

/// Encrypt the resolved `--env` entries with the server's public key.
async fn encrypt_entries(
    client: &RicochetClient,
    env: &[String],
    dir: &Path,
) -> Result<crate::crypto::RsaEncryptedEnvVars> {
    let resolved = crate::env_vars::resolve_env_vars(env, dir)?;
    let pub_key = client.get_public_key().await?;
    crate::crypto::encrypt_env_vars(&pub_key, &resolved)
}

pub async fn set_env_vars(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<&str>,
    path: Option<&Path>,
    env: &[String],
    format: OutputFormat,
) -> Result<()> {
    if env.is_empty() {
        anyhow::bail!("Provide at least one `--env KEY[=VALUE]`");
    }

    let id = resolve_id(id, path)?;
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let encrypted = encrypt_entries(&client, env, &env_dir(path)).await?;
    let names = client.upsert_env_vars(&id, &encrypted).await?;

    println!(
        "{} Environment variable(s) set; instances will restart to pick them up",
        "✓".green().bold(),
    );

    print_names(server_config.url.as_str(), &names, format)
}

pub async fn replace_env_vars(
    config: &Config,
    server_ref: Option<&str>,
    id: Option<&str>,
    path: Option<&Path>,
    env: &[String],
    force: bool,
    format: OutputFormat,
) -> Result<()> {
    let id = resolve_id(id, path)?;

    if !force {
        let message = "This replaces ALL environment variables; anything not listed will be deleted. Continue?";
        if !utils::confirm(message)? {
            println!("{}", "Replace cancelled".yellow());
            return Ok(());
        }
    }

    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;
    client.preflight_key_check().await?;

    let encrypted = encrypt_entries(&client, env, &env_dir(path)).await?;
    let names = client.replace_env_vars(&id, &encrypted).await?;

    println!(
        "{} Environment variables replaced; instances will restart to pick them up",
        "✓".green().bold(),
    );

    print_names(server_config.url.as_str(), &names, format)
}
