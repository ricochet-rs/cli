use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct RicochetToml {
    #[serde(default)]
    content: ContentSection,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
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
    let ricochet_toml: RicochetToml = toml::from_str(&toml_content)?;

    let content_id = ricochet_toml.content.id.clone();
    let content_type = ricochet_toml.content.content_type.clone();

    if content_id.is_none() && content_type.is_none() {
        anyhow::bail!("Invalid _ricochet.toml: either 'content.id' or 'content.content_type' must be present");
    }

    if let Some(ref id) = content_id {
        println!("📦 Creating new deployment for content item: {}\n", id.bright_cyan());
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

                // Update _ricochet.toml with the content ID if it's a new deployment
                if content_id.is_none() {
                    // Read the original file content
                    let original_content = std::fs::read_to_string(&toml_path)?;
                    
                    // Find the [content] section and add/update the id field
                    let updated_content = if original_content.contains("id =") {
                        // Replace existing id field
                        use regex::Regex;
                        let re = Regex::new(r#"(?m)^(\s*)id\s*=\s*.*$"#)?;
                        re.replace(&original_content, format!("${{1}}id = \"{}\"", id)).to_string()
                    } else {
                        // Add id field after [content] section
                        use regex::Regex;
                        let re = Regex::new(r#"(?m)^\[content\]$"#)?;
                        re.replace(&original_content, format!("[content]\nid = \"{}\"", id)).to_string()
                    };
                    
                    std::fs::write(&toml_path, updated_content)?;
                }

                // Get server URL and construct links
                let server_url = config.server_url()?;
                let base_url = server_url.trim_end_matches('/');

                println!("\n{}", "Links:".bold());

                // Show deployment link if deployment_id is available
                if let Some(deployment_id) = response.get("deployment_id")
                    .or_else(|| response.get("deploymentId"))
                    .and_then(|v| v.as_str()) {
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
            anyhow::bail!("Deployment failed: {}", e)
        }
    }
}
