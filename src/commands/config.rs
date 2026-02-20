use crate::config::Config;
use anyhow::Result;
use colored::Colorize;

pub fn show(config: &Config, show_all: bool) -> Result<()> {
    println!("⚙️  {}\n", "Ricochet CLI Configuration".bold());

    println!("Config file: {}", Config::config_path()?.display());
    println!();

    // Show default server
    if let Some(default_name) = config.default_server() {
        println!("Default server: {}", default_name.bright_cyan());
    } else {
        println!("Default server: {}", "Not set".yellow());
    }

    if let Some(format) = &config.default_format {
        println!("Default format: {}", format);
    }

    // Show configured servers
    println!("\n{}", "Configured Servers:".bold());

    let servers = config.list_servers();
    if servers.is_empty() {
        println!("  {}", "None".dimmed());
    } else {
        let default_name = config.default_server();
        let mut sorted_servers: Vec<_> = servers.into_iter().collect();
        sorted_servers.sort_by(|a, b| a.0.cmp(b.0));

        for (name, server_config) in sorted_servers {
            let is_default = default_name == Some(name.as_str());
            let marker = if is_default { " (default)" } else { "" };

            println!("\n  {}{}", name.bright_cyan(), marker.dimmed());
            println!("    URL: {}", server_config.url.as_str());

            if let Some(api_key) = &server_config.api_key {
                if show_all {
                    println!("    API Key: {}", api_key.bright_cyan());
                } else {
                    let masked = if api_key.starts_with("rico_") && api_key.len() > 10 {
                        format!("{}...{}", &api_key[..8], &api_key[api_key.len() - 4..])
                    } else {
                        "***hidden***".to_string()
                    };
                    println!("    API Key: {}", masked);
                }
            } else {
                println!("    API Key: {}", "Not configured".yellow());
            }
        }
    }

    println!("\n{}", "Environment Variables:".bold());

    if let Ok(server_env) = std::env::var("RICOCHET_SERVER") {
        println!(
            "  RICOCHET_SERVER: {} {}",
            server_env.bright_cyan(),
            "(overrides default)".dimmed()
        );
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
                "(overrides all server keys)".dimmed()
            );
        }
    } else {
        println!("  RICOCHET_API_KEY: {}", "Not set".dimmed());
    }

    Ok(())
}
