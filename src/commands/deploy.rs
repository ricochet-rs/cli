use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

pub async fn deploy(
    config: &Config,
    path: PathBuf,
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    // Check for _ricochet.toml
    let toml_path = if path.is_dir() {
        path.join("_ricochet.toml")
    } else {
        anyhow::bail!("Path must be a directory containing _ricochet.toml");
    };

    if !toml_path.exists() {
        anyhow::bail!("No _ricochet.toml found in {}", path.display());
    }

    println!("📦 Deploying content from: {}\n", path.display());

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Creating bundle...");

    let client = RicochetClient::new(config)?;

    pb.set_message("Uploading to server...");

    match client.deploy(&path, name, description).await {
        Ok(response) => {
            pb.finish_and_clear();

            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                println!("{} Deployment successful!", "✓".green().bold());
                println!("\nContent ID: {}", id.bright_cyan());

                if let Some(name) = response.get("name").and_then(|v| v.as_str()) {
                    println!("Name: {}", name);
                }

                if let Some(content_type) = response.get("content_type").and_then(|v| v.as_str()) {
                    println!("Type: {}", content_type);
                }

                if let Some(status) = response.get("status").and_then(|v| v.as_str()) {
                    let status_colored = match status {
                        "deployed" => status.green(),
                        "failed" => status.red(),
                        _ => status.yellow(),
                    };
                    println!("Status: {}", status_colored);
                }
            } else {
                println!("{} Deployment successful!", "✓".green().bold());
                println!("\n{}", serde_json::to_string_pretty(&response)?);
            }

            Ok(())
        }
        Err(e) => {
            pb.finish_and_clear();
            anyhow::bail!("Deployment failed: {}", e)
        }
    }
}
