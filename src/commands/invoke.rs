use crate::{client::RicochetClient, config::Config};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn invoke(config: &Config, id: &str, params: Option<String>) -> Result<()> {
    println!("🚀 Invoking content item: {}\n", id.bright_cyan());

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Sending invocation request...");

    let client = RicochetClient::new(config)?;

    match client.invoke(id, params).await {
        Ok(response) => {
            pb.finish_and_clear();

            if let Some(invocation_id) = response.get("invocation_id").and_then(|v| v.as_str()) {
                println!("{} Invocation started successfully!", "✓".green().bold());
                println!("\nInvocation ID: {}", invocation_id.bright_cyan());

                if let Some(status) = response.get("status").and_then(|v| v.as_str()) {
                    println!("Status: {}", status.green());
                }

                println!(
                    "\n{}",
                    "Use 'ricochet stop' to stop this invocation if needed".dimmed()
                );
            } else {
                println!("{} Invocation completed!", "✓".green().bold());

                // Check if there's output
                if let Some(output) = response.get("output") {
                    println!("\n{}", "Output:".bold());
                    if let Some(output_str) = output.as_str() {
                        println!("{}", output_str);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    }
                }

                // Check if there's an error
                if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
                    println!("\n{} {}", "Error:".red().bold(), error);
                }
            }

            Ok(())
        }
        Err(e) => {
            pb.finish_and_clear();
            anyhow::bail!("Invocation failed: {}", e)
        }
    }
}
