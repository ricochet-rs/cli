use crate::config::{Config, parse_server_url};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use dialoguer::{Confirm, theme::ColorfulTheme};

/// List all configured servers
pub fn list(config: &Config) -> Result<()> {
    let servers = config.list_servers();

    if servers.is_empty() {
        println!("{}", "No servers configured.".yellow());
        println!(
            "\nAdd a server with: {}",
            "ricochet servers add <name> <url>".bright_cyan()
        );
        return Ok(());
    }

    let default_server = config.default_server();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Name", "URL", "API Key", "Default"]);

    // Collect and sort servers by name
    let mut sorted_servers: Vec<_> = servers.into_iter().collect();
    sorted_servers.sort_by(|a, b| a.0.cmp(b.0));

    for (name, server_config) in sorted_servers {
        let is_default = default_server == Some(name.as_str());

        let name_cell = if is_default {
            Cell::new(name).fg(Color::Green)
        } else {
            Cell::new(name)
        };

        let api_key_status = if server_config.api_key.is_some() {
            Cell::new("configured").fg(Color::Green)
        } else {
            Cell::new("not set").fg(Color::Red)
        };

        let default_marker = if is_default {
            Cell::new("*").fg(Color::Green)
        } else {
            Cell::new("")
        };

        table.add_row(vec![
            name_cell,
            Cell::new(server_config.url.as_str()),
            api_key_status,
            default_marker,
        ]);
    }

    println!("{}", table);

    // Show config file path (shell-escaped for copy-paste)
    if let Ok(config_path) = Config::config_path() {
        let path_str = config_path.display().to_string();
        let escaped_path = path_str.replace(' ', "\\ ");
        println!("\n{} {}", "Config:".dimmed(), escaped_path);
    }

    Ok(())
}

/// Add a new server
pub fn add(config: &mut Config, name: String, url: String, default: bool) -> Result<()> {
    let parsed_url = parse_server_url(&url)?;

    // Check if server already exists
    if config.servers.contains_key(&name) {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Server '{}' already exists. Overwrite?", name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    config.add_server(name.clone(), parsed_url.clone(), None);

    if default {
        config.set_default_server(&name)?;
    }

    config.save()?;

    println!(
        "{} Server '{}' added: {}",
        "✓".green().bold(),
        name.bright_cyan(),
        parsed_url.as_str()
    );

    if config.default_server() == Some(&name) {
        println!("  Set as default server");
    }

    println!(
        "\nAuthenticate with: {}",
        format!("ricochet login --server {}", name).bright_cyan()
    );

    Ok(())
}

/// Remove a server
pub fn remove(config: &mut Config, name: String, force: bool) -> Result<()> {
    if !config.servers.contains_key(&name) {
        anyhow::bail!("Server '{}' not found", name);
    }

    let is_default = config.default_server() == Some(&name);

    if !force {
        let prompt = if is_default {
            format!("Remove default server '{}'?", name)
        } else {
            format!("Remove server '{}'?", name)
        };

        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    let was_default = config.remove_server(&name)?;
    config.save()?;

    println!(
        "{} Server '{}' removed",
        "✓".green().bold(),
        name.bright_cyan()
    );

    if was_default && !config.servers.is_empty() {
        println!(
            "  {}",
            "No default server set. Use 'ricochet servers set-default <name>' to set one.".yellow()
        );
    }

    Ok(())
}

/// Set the default server
pub fn set_default(config: &mut Config, name: String) -> Result<()> {
    config.set_default_server(&name)?;
    config.save()?;

    println!(
        "{} Default server set to '{}'",
        "✓".green().bold(),
        name.bright_cyan()
    );

    Ok(())
}
