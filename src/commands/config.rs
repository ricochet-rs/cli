use crate::{
    OutputFormat,
    commands::server::{ConfiguredServer, configured_servers, mask_api_key},
    config::Config,
};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

/// The environment variables that override the stored configuration.
#[derive(Serialize)]
struct EnvironmentOverrides {
    ricochet_server: Option<String>,
    ricochet_api_key: Option<String>,
}

/// The effective CLI configuration.
#[derive(Serialize)]
struct ConfigView {
    config_file: String,
    default_server: Option<String>,
    default_format: Option<String>,
    servers: Vec<ConfiguredServer>,
    environment: EnvironmentOverrides,
}

/// Reveal the key in full when the caller asked for it, otherwise mask it.
fn shown_api_key(api_key: String, show_all: bool) -> String {
    if show_all {
        api_key
    } else {
        mask_api_key(&api_key)
    }
}

pub fn show(config: &Config, show_all: bool, format: OutputFormat) -> Result<()> {
    let mut servers = configured_servers(config);
    if show_all {
        for server in &mut servers {
            if let Some(api_key) = config
                .servers
                .get(&server.name)
                .and_then(|c| c.api_key.clone())
            {
                server.api_key = Some(api_key);
            }
        }
    }

    let view = ConfigView {
        config_file: Config::config_path()?.display().to_string(),
        default_server: config.default_server().map(str::to_string),
        default_format: config.default_format.clone(),
        servers,
        environment: EnvironmentOverrides {
            ricochet_server: std::env::var("RICOCHET_SERVER").ok(),
            ricochet_api_key: std::env::var("RICOCHET_API_KEY")
                .ok()
                .map(|key| shown_api_key(key, show_all)),
        },
    };

    format.print(&view, || {
        let mut output = format!("⚙️  {}\n", "Ricochet CLI Configuration".bold());
        output.push_str(&format!("\nConfig file: {}\n", view.config_file));

        match &view.default_server {
            Some(name) => output.push_str(&format!("\nDefault server: {}", name.bright_cyan())),
            None => output.push_str(&format!("\nDefault server: {}", "Not set".yellow())),
        }

        if let Some(format) = &view.default_format {
            output.push_str(&format!("\nDefault format: {format}"));
        }

        output.push_str(&format!("\n\n{}", "Configured Servers:".bold()));

        if view.servers.is_empty() {
            output.push_str(&format!("\n  {}", "None".dimmed()));
        } else {
            for server in &view.servers {
                let marker = if server.default { " (default)" } else { "" };
                output.push_str(&format!(
                    "\n\n  {}{}\n    URL: {}\n    API Key: {}",
                    server.name.bright_cyan(),
                    marker.dimmed(),
                    server.url,
                    match &server.api_key {
                        Some(api_key) => api_key.bright_cyan().to_string(),
                        None => "Not configured".yellow().to_string(),
                    }
                ));
            }
        }

        output.push_str(&format!("\n\n{}", "Environment Variables:".bold()));
        match &view.environment.ricochet_server {
            Some(server) => output.push_str(&format!(
                "\n  RICOCHET_SERVER: {} {}",
                server.bright_cyan(),
                "(overrides default)".dimmed()
            )),
            None => output.push_str(&format!("\n  RICOCHET_SERVER: {}", "Not set".dimmed())),
        }
        match &view.environment.ricochet_api_key {
            Some(api_key) if show_all => {
                output.push_str(&format!("\n  RICOCHET_API_KEY: {}", api_key.bright_cyan()))
            }
            Some(_) => output.push_str(&format!(
                "\n  RICOCHET_API_KEY: {} {}",
                "***set***".green(),
                "(overrides all server keys)".dimmed()
            )),
            None => output.push_str(&format!("\n  RICOCHET_API_KEY: {}", "Not set".dimmed())),
        }

        Ok(output)
    })
}
