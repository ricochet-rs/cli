use anyhow::Result;
use clap::{Parser, Subcommand};
use ricochet_cli::{OutputFormat, commands, config::Config};

#[derive(Parser)]
#[command(name = "ricochet")]
#[command(about = "Ricochet CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL (can also be set with RICOCHET_SERVER environment variable)
    #[arg(global = true, short = 'S', long, env = "RICOCHET_SERVER", help_heading = "Global Options")]
    server: Option<String>,

    /// Output format
    #[arg(global = true, short = 'F', long, default_value = "table", value_enum, help_heading = "Global Options")]
    format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with the Ricochet server
    Login {
        /// API key (can also be provided interactively)
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },
    /// Remove stored credentials
    Logout,
    /// Deploy content to the server
    Deploy {
        /// Path to the content directory or bundle
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// Name for the deployment
        #[arg(short = 'n', long)]
        name: Option<String>,
        /// Description for the deployment
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    /// List all content items
    List {
        /// Filter by content type
        #[arg(short = 't', long)]
        content_type: Option<String>,
        /// Show only active deployments (status: deployed, running, or success)
        #[arg(short = 'a', long)]
        active_only: bool,
        /// Sort by field(s) - comma-separated for multiple (e.g., "name,updated" or "status,name")
        /// Prefix with '-' for descending order (e.g., "-updated,name")
        #[arg(short = 's', long)]
        sort: Option<String>,
    },
    /// Get status of a content item
    Status {
        /// Content item ID (ULID)
        id: String,
    },
    /// Invoke a content item
    Invoke {
        /// Content item ID (ULID)
        id: String,
        /// Parameters as JSON
        #[arg(short = 'p', long)]
        params: Option<String>,
    },
    /// Stop a running service or invocation
    Stop {
        /// Content item ID (ULID)
        id: String,
        /// Instance PID (for services) or invocation ID
        #[arg(short = 'i', long)]
        instance: Option<String>,
    },
    /// Delete a content item
    Delete {
        /// Content item ID (ULID)
        id: String,
        /// Skip confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Manage content schedules
    Schedule {
        /// Content item ID (ULID)
        id: String,
        /// Cron expression (e.g., "0 0 * * *" for daily at midnight)
        #[arg(short = 'c', long)]
        cron: Option<String>,
        /// Disable the schedule
        #[arg(short = 'D', long)]
        disable: bool,
    },
    /// Update content settings
    Settings {
        /// Content item ID (ULID)
        id: String,
        /// Update settings as JSON
        #[arg(short = 'u', long)]
        update: String,
    },
    /// Show configuration
    Config {
        /// Show full configuration including sensitive values
        #[arg(short = 'A', long)]
        show_all: bool,
    },
    /// Generate markdown documentation (hidden command)
    #[command(hide = true)]
    GenerateDocs,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load or initialize config
    let mut config = Config::load()?;

    // Override server if provided via CLI
    if let Some(server) = cli.server {
        config.server = Some(server);
    }

    // Execute command
    match cli.command {
        Commands::Login { api_key } => {
            commands::auth::login(&mut config, api_key).await?;
        }
        Commands::Logout => {
            commands::auth::logout(&mut config)?;
        }
        Commands::Deploy {
            path,
            name,
            description,
        } => {
            commands::deploy::deploy(&config, path, name, description).await?;
        }
        Commands::List {
            content_type,
            active_only,
            sort,
        } => {
            commands::list::list(&config, content_type, active_only, sort, cli.format).await?;
        }
        Commands::Status { id } => {
            commands::status::status(&config, &id, cli.format).await?;
        }
        Commands::Invoke { id, params } => {
            commands::invoke::invoke(&config, &id, params).await?;
        }
        Commands::Stop { id, instance } => {
            commands::stop::stop(&config, &id, instance).await?;
        }
        Commands::Delete { id, force } => {
            commands::delete::delete(&config, &id, force).await?;
        }
        Commands::Schedule { id, cron, disable } => {
            commands::schedule::update(&config, &id, cron, disable).await?;
        }
        Commands::Settings { id, update } => {
            commands::settings::update(&config, &id, &update).await?;
        }
        Commands::Config { show_all } => {
            commands::config::show(&config, show_all)?;
        }
        Commands::GenerateDocs => {
            let markdown = clap_markdown::help_markdown::<Cli>();
            println!("{}", markdown);
        }
    }

    Ok(())
}
