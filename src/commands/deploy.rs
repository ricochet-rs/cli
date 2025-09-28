use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct RicochetToml {
    content: ContentSection,
}

#[derive(Debug, Deserialize, Serialize)]
struct ContentSection {
    id: Option<String>,
    content_type: Option<String>,
}

pub async fn deploy(
    config: &Config,
    path: PathBuf,
    _name: Option<String>,
    _description: Option<String>,
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

    // Read and parse _ricochet.toml
    let toml_content = std::fs::read_to_string(&toml_path)?;
    let mut ricochet_toml: RicochetToml = toml::from_str(&toml_content)?;
    
    let content_id = ricochet_toml.content.id.clone();
    let content_type = ricochet_toml.content.content_type.clone();

    if content_id.is_none() && content_type.is_none() {
        anyhow::bail!("Invalid _ricochet.toml: either 'content.id' or 'content.content_type' must be present");
    }

    if let Some(ref id) = content_id {
        println!("📦 Creating new deployment for item: {}\n", id.bright_cyan());
    } else if let Some(ref ctype) = content_type {
        println!("📦 Deploying new {} content item\n", ctype.bright_cyan());
    } else {
        println!("📦 Deploying content from: {}\n", path.display());
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Creating bundle...");

    let client = RicochetClient::new(config)?;

    pb.set_message("Uploading to server...");

    match client.deploy(&path, content_id.clone(), &toml_path).await {
        Ok(response) => {
            pb.finish_and_clear();

            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                println!("{} Deployment successful!", "✓".green().bold());
                println!("\nContent ID: {}", id.bright_cyan());

                // Update _ricochet.toml with the content ID if it's a new deployment
                if content_id.is_none() {
                    ricochet_toml.content.id = Some(id.to_string());
                    let updated_toml = toml::to_string_pretty(&ricochet_toml)?;
                    std::fs::write(&toml_path, updated_toml)?;
                    println!("Updated _ricochet.toml with content ID");
                }

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
