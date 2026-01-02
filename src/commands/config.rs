use crate::config::Config;
use anyhow::Result;
use colored::Colorize;

pub fn show(config: &Config, show_all: bool) -> Result<()> {
    println!("⚙️  {}\n", "Ricochet CLI Configuration".bold());

    println!("Config file: {}", Config::config_path()?.display());
    println!();

    println!("Server URL: {}", config.server.as_str().bright_cyan());

    if let Some(api_key) = &config.api_key {
        if show_all {
            println!("API Key: {}", api_key.bright_cyan());
        } else {
            // Mask the API key
            let masked = if api_key.starts_with("rico_") && api_key.len() > 10 {
                format!("{}...{}", &api_key[..8], &api_key[api_key.len() - 4..])
            } else {
                "***hidden***".to_string()
            };
            println!(
                "API Key: {} {}",
                masked,
                "(use --show-all to reveal)".dimmed()
            );
        }
    } else {
        println!("API Key: {}", "Not configured".yellow());
    }

    if let Some(format) = &config.default_format {
        println!("Default Format: {}", format);
    }

    println!("\n{}", "Environment Variables:".bold());

    if let Ok(server_env) = std::env::var("RICOCHET_SERVER") {
        println!("  RICOCHET_SERVER: {}", server_env.bright_cyan());
    } else {
        println!("  RICOCHET_SERVER: {}", "Not set".dimmed());
    }

    if std::env::var("RICOCHET_API_KEY").is_ok() {
        if show_all {
            println!(
                "  RICOCHET_API_KEY: {}",
                std::env::var("RICOCHET_API_KEY").unwrap().bright_cyan()
            );
        } else {
            println!(
                "  RICOCHET_API_KEY: {} {}",
                "***set***".green(),
                "(use --show-all to reveal)".dimmed()
            );
        }
    } else {
        println!("  RICOCHET_API_KEY: {}", "Not set".dimmed());
    }

    Ok(())
}
