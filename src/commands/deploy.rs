use crate::{client::RicochetClient, config::Config};
use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use ricochet_core::{content::ContentItem, language::Package};
use std::path::PathBuf;

pub async fn deploy(
    config: &Config,
    server_ref: Option<&str>,
    path: PathBuf,
    _name: Option<String>,
    _description: Option<String>,
    debug: bool,
) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    // Resolve server configuration early so we can bail before the init dialog
    // if the user has no API key configured
    let server_config = config.resolve_server(server_ref)?;
    let client = RicochetClient::new(&server_config)?;

    client.preflight_key_check().await?;

    // Check for _ricochet.toml
    let toml_path = if path.is_dir() {
        path.join("_ricochet.toml")
    } else {
        anyhow::bail!("Path must be a directory containing _ricochet.toml");
    };

    if !toml_path.exists() {
        // Check if we're in an interactive terminal (not in tests or CI)
        if !crate::utils::is_non_interactive() {
            let confirmed = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "No _ricochet.toml found. Would you like to create one? (deploying to {})",
                    server_config.url.as_str().trim_end_matches('/')
                ))
                .default(true)
                .interact()?;

            if !confirmed {
                anyhow::bail!("No _ricochet.toml provided. Please create one with `ricochet init`");
            }

            // Create _ricochet.toml using init command
            crate::commands::init::init_rico_toml(&path, false, false)?;
        } else {
            // Non-interactive mode (tests, CI, etc.)
            anyhow::bail!(
                "No _ricochet.toml found in {}. Please create one with `ricochet init`",
                path.display()
            );
        }
    }

    // Read and parse _ricochet.toml
    let toml_content = std::fs::read_to_string(&toml_path)?;
    let ricochet_toml = ContentItem::from_toml(&toml_content)?;

    let content_id = ricochet_toml.content.id.clone();
    let content_type = ricochet_toml.content.content_type;

    // check for existence of packages file
    let pkgs = ricochet_toml.language.packages;
    let pkg_path = path.join(pkgs.to_string());

    // bail if the package file doesnt exist.
    if !pkg_path.exists() {
        match pkgs {
            Package::RenvLock => bail!("Please create an `renv.lock` via `renv::snapshot()`"),
            Package::ManifestToml => {
                bail!("Please create a `Manifest.toml` via `Pkg.instantiate()`")
            }
            Package::UvLock => bail!("Please create a `uv.lock` via `uv init`"),
        }
    }

    // if python and no .python-version bail
    if let Package::UvLock = pkgs
        && !path.join(".python-version").exists() {
            bail!("Please create a `.python-version` via `uv python pin`")
        }

    if let Some(ref id) = content_id {
        println!(
            "📦 Creating new deployment for content item: {}\n",
            id.bright_cyan()
        );
    } else {
        println!(
            "📦 Deploying new {} content item\n",
            content_type.to_string().bright_cyan()
        );
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    match client
        .deploy(&path, content_id.clone(), &toml_path, &pb, debug)
        .await
    {
        Ok(response) => {
            pb.finish_and_clear();

            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                println!("{} Deployment successful!", "✓".green().bold());

                // Update _ricochet.toml with the content ID if it's a new deployment
                if content_id.is_none() {
                    // Read the original file content
                    let original_content = std::fs::read_to_string(&toml_path)?;

                    // Find the [content] section and add/update the id field
                    let updated_content = if original_content.contains("id =") {
                        // Replace existing id field
                        use regex::Regex;
                        let re = Regex::new(r#"(?m)^(\s*)id\s*=\s*.*$"#)?;
                        re.replace(&original_content, format!("${{1}}id = \"{}\"", id))
                            .to_string()
                    } else {
                        // FIXME: use toml-edit here
                        // Add id field after [content] section
                        use regex::Regex;
                        let re = Regex::new(r#"(?m)^\[content\]$"#)?;
                        re.replace(&original_content, format!("[content]\nid = \"{}\"", id))
                            .to_string()
                    };

                    std::fs::write(&toml_path, updated_content)?;
                }

                // Get server URL and construct links
                let base_url = server_config.url.as_str().trim_end_matches('/');

                println!("\n{}", "Links:".bold());

                // Show deployment link if deployment_id is available
                if let Some(deployment_id) = response
                    .get("deployment_id")
                    .or_else(|| response.get("deploymentId"))
                    .and_then(|v| v.as_str())
                {
                    println!("  Deployment: {}/deployments/{}", base_url, deployment_id);
                }

                // Show app overview link
                println!("  App Overview: {}/apps/{}/overview", base_url, id);
            } else {
                println!("{} Deployment successful!", "✓".green().bold());
                println!("\n{}", serde_json::to_string_pretty(&response)?);
            }

            Ok(())
        }
        Err(e) => {
            pb.finish_and_clear();

            // Provide helpful context for 403 errors when updating existing content
            if let Some(id) = content_id.as_ref()
                && e.to_string().contains("403")
            {
                eprintln!("{} Deployment failed: {}\n", "✗".red().bold(), e);
                eprintln!("{}", "Hint:".yellow().bold());
                eprintln!(
                    "  You're trying to update content item: {}",
                    id.bright_cyan()
                );
                eprintln!("  This error usually means:");
                eprintln!("    • The content ID doesn't exist on this server");
                eprintln!("    • Your API key lacks permission to modify this content item");
                eprintln!("    • The content item was created on a different server\n");
                eprintln!("  Try:");
                eprintln!(
                    "    1. Run {} to verify the content item exists",
                    "ricochet list".bright_cyan()
                );
                eprintln!(
                    "    2. Check if you're connected to the correct server: {}",
                    server_config.url.as_str().bright_cyan()
                );
                eprintln!(
                    "    3. Remove the 'id' field from _ricochet.toml to create a new content item instead"
                );
                anyhow::bail!("")
            }
            anyhow::bail!("Deployment failed: {}", e)
        }
    }
}
