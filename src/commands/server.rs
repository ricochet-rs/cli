use crate::{
    OutputFormat,
    config::{Config, parse_server_url},
};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use dialoguer::{Confirm, theme::ColorfulTheme};
use serde::Serialize;

/// A configured server, as `server list` and `config` report it.
#[derive(Serialize)]
pub struct ConfiguredServer {
    pub name: String,
    pub url: String,
    /// Masked unless the caller asked for the full value.
    pub api_key: Option<String>,
    pub default: bool,
}

/// Shorten an API key to its recognisable head and tail.
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.starts_with("rico_") && api_key.len() > 10 {
        format!("{}...{}", &api_key[..8], &api_key[api_key.len() - 4..])
    } else {
        "***hidden***".to_string()
    }
}

/// Collect the configured servers, sorted by name, with masked API keys.
pub fn configured_servers(config: &Config) -> Vec<ConfiguredServer> {
    let default_server = config.default_server();
    let mut servers: Vec<_> = config.list_servers();
    servers.sort_by(|a, b| a.0.cmp(b.0));

    servers
        .into_iter()
        .map(|(name, server_config)| ConfiguredServer {
            name: name.clone(),
            url: server_config.url.as_str().to_string(),
            api_key: server_config.api_key.as_deref().map(mask_api_key),
            default: default_server == Some(name.as_str()),
        })
        .collect()
}

/// The outcome of a command that changed the server configuration.
#[derive(Serialize)]
struct ServerChange {
    name: String,
    default_server: Option<String>,
}

/// List all configured servers
pub fn list(config: &Config, format: OutputFormat) -> Result<()> {
    let servers = configured_servers(config);

    format.print(&servers, || {
        if servers.is_empty() {
            return Ok(format!(
                "{}\n\nAdd a server with: {}",
                "No servers configured.".yellow(),
                "ricochet server add <name> <url>".bright_cyan()
            ));
        }

        let mut table = Table::new();
        table.load_style(UTF8_FULL);
        table.set_header(vec!["Name", "URL", "API Key", "Default"]);

        for server in &servers {
            let name_cell = if server.default {
                Cell::new(&server.name).fg(Color::Green)
            } else {
                Cell::new(&server.name)
            };

            let api_key_status = match server.api_key {
                Some(_) => Cell::new("configured").fg(Color::Green),
                None => Cell::new("not set").fg(Color::Red),
            };

            let default_marker = if server.default {
                Cell::new("*").fg(Color::Green)
            } else {
                Cell::new("")
            };

            table.add_row(vec![
                name_cell,
                Cell::new(&server.url),
                api_key_status,
                default_marker,
            ]);
        }

        let mut output = table.to_string();

        // Show config file path (shell-escaped for copy-paste)
        if let Ok(config_path) = Config::config_path() {
            let escaped_path = config_path.display().to_string().replace(' ', "\\ ");
            output.push_str(&format!("\n\n{} {escaped_path}", "Config:".dimmed()));
        }

        Ok(output)
    })
}

/// Add a new server
pub fn add(
    config: &mut Config,
    name: String,
    url: String,
    default: bool,
    format: OutputFormat,
) -> Result<()> {
    let parsed_url = parse_server_url(&url)?;

    // Check if server already exists
    if config.servers.contains_key(&name) {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Server '{name}' already exists. Overwrite?"))
            .default(false)
            .interact()?;

        if !confirmed {
            eprintln!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    config.add_server(name.clone(), parsed_url.clone(), None);

    if default {
        config.set_default_server(&name)?;
    }

    config.save()?;

    let change = ServerChange {
        name,
        default_server: config.default_server().map(str::to_string),
    };

    format.print(&change, || {
        let mut output = format!(
            "{} Server '{}' added: {}",
            "✓".green().bold(),
            change.name.bright_cyan(),
            parsed_url.as_str()
        );

        if change.default_server.as_deref() == Some(change.name.as_str()) {
            output.push_str("\n  Set as default server");
        }

        output.push_str(&format!(
            "\n\nAuthenticate with: {}",
            format!("ricochet login --server {}", change.name).bright_cyan()
        ));
        Ok(output)
    })
}

/// Remove a server
pub fn remove(config: &mut Config, name: String, force: bool, format: OutputFormat) -> Result<()> {
    if !config.servers.contains_key(&name) {
        anyhow::bail!("Server '{}' not found", name);
    }

    let is_default = config.default_server() == Some(&name);

    if !force {
        let prompt = if is_default {
            format!("Remove default server '{name}'?")
        } else {
            format!("Remove server '{name}'?")
        };

        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            eprintln!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    let was_default = config.remove_server(&name)?;
    config.save()?;
    let servers_remain = !config.servers.is_empty();

    let change = ServerChange {
        name,
        default_server: config.default_server().map(str::to_string),
    };

    format.print(&change, || {
        let mut output = format!(
            "{} Server '{}' removed",
            "✓".green().bold(),
            change.name.bright_cyan()
        );

        if was_default && servers_remain {
            output.push_str(&format!(
                "\n  {}",
                "No default server set. Use 'ricochet server set-default <name>' to set one."
                    .yellow()
            ));
        }

        Ok(output)
    })
}

/// Set the default server
pub fn set_default(config: &mut Config, name: String, format: OutputFormat) -> Result<()> {
    config.set_default_server(&name)?;
    config.save()?;

    let change = ServerChange {
        default_server: Some(name.clone()),
        name,
    };

    format.print(&change, || {
        Ok(format!(
            "{} Default server set to '{}'",
            "✓".green().bold(),
            change.name.bright_cyan()
        ))
    })
}
