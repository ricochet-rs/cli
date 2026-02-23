use anyhow::Result;
use clap::{Parser, Subcommand};
use ricochet_cli::{OutputFormat, commands, config::Config, update};

#[derive(Parser)]
#[command(name = "ricochet")]
#[command(about = "Ricochet CLI", long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Server URL (can also be set with RICOCHET_SERVER environment variable)
    #[arg(
        global = true,
        short = 'S',
        long,
        env = "RICOCHET_SERVER",
        help_heading = "Global Options"
    )]
    server: Option<String>,

    /// Output format
    #[arg(
        global = true,
        short = 'F',
        long,
        default_value = "table",
        value_enum,
        help_heading = "Global Options"
    )]
    format: OutputFormat,

    /// Enable debug output
    #[arg(global = true, long, help_heading = "Global Options")]
    debug: bool,

    /// Print version
    #[arg(short = 'V', long)]
    version: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a Ricochet server
    Login {
        /// API key (can also be provided interactively)
        #[arg(short = 'k', long)]
        api_key: Option<String>,
    },
    /// Remove stored credentials
    Logout,
    /// Deploy content to a Ricochet server
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
    /// Delete a content item
    Delete {
        /// Content item ID (ULID)
        id: String,
        /// Skip confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Invoke a task
    Invoke {
        /// Content item ID (ULID)
        id: String,
    },
    /// Show configuration
    Config {
        /// Show full configuration including sensitive values
        #[arg(short = 'A', long)]
        show_all: bool,
    },
    /// Initialize a new Ricochet deployment
    Init {
        /// Directory to initialize (defaults to current directory)
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// Overwrite existing _ricochet.toml file without confirmation
        #[arg(long)]
        overwrite: bool,
        /// Preview the _ricochet.toml without saving to file
        #[arg(long)]
        dry_run: bool,
    },
    /// Manage configured Ricochet servers
    Servers {
        #[command(subcommand)]
        command: ServersCommands,
    },
    /// Update the ricochet CLI to the latest version
    SelfUpdate {
        /// Force reinstall even if already on the latest version
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Generate markdown documentation (hidden command)
    #[command(hide = true)]
    GenerateDocs,
}

#[derive(Subcommand)]
enum ServersCommands {
    /// List all configured servers
    List,
    /// Add a new server
    Add {
        /// Server name (e.g., 'production', 'staging', 'local')
        name: String,
        /// Server URL (must include http:// or https://)
        url: String,
        /// Set this server as the default
        #[arg(long)]
        default: bool,
    },
    /// Remove a server
    Remove {
        /// Server name to remove
        name: String,
        /// Skip confirmation prompt
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Set the default server
    SetDefault {
        /// Server name to set as default
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle version flag
    if cli.version {
        let version = env!("CARGO_PKG_VERSION");
        let git_hash = env!("GIT_HASH");
        let has_tag = env!("HAS_GIT_TAG");
        let build_date = env!("BUILD_DATE");

        if has_tag == "true" || git_hash.is_empty() {
            // Tagged release
            println!("{}", version);
        } else {
            // Untagged build
            println!("{}-{} ({})", version, git_hash, build_date);
        }
        return Ok(());
    }

    // Load or initialize config
    let mut config = Config::load()?;

    // Execute command
    match cli.command {
        Some(Commands::Login { api_key }) => {
            commands::auth::login(&mut config, cli.server.as_deref(), api_key).await?;
        }
        Some(Commands::Logout) => {
            commands::auth::logout(&mut config, cli.server.as_deref())?;
        }
        Some(Commands::Deploy {
            path,
            name,
            description,
        }) => {
            commands::deploy::deploy(
                &config,
                cli.server.as_deref(),
                path,
                name,
                description,
                cli.debug,
            )
            .await?;
        }
        Some(Commands::List {
            content_type,
            active_only,
            sort,
        }) => {
            commands::list::list(
                &config,
                cli.server.as_deref(),
                content_type,
                active_only,
                sort,
                cli.format,
            )
            .await?;
        }
        Some(Commands::Delete { id, force }) => {
            commands::delete::delete(&config, cli.server.as_deref(), &id, force).await?;
        }
        Some(Commands::Invoke { id }) => {
            commands::invoke::invoke(&config, cli.server.as_deref(), &id, cli.format).await?;
        }
        Some(Commands::Config { show_all }) => {
            commands::config::show(&config, show_all)?;
        }
        Some(Commands::Init {
            path,
            overwrite,
            dry_run,
        }) => {
            commands::init::init_rico_toml(&path, overwrite, dry_run)?;
        }
        Some(Commands::Servers { command }) => match command {
            ServersCommands::List => {
                commands::servers::list(&config)?;
            }
            ServersCommands::Add { name, url, default } => {
                commands::servers::add(&mut config, name, url, default)?;
            }
            ServersCommands::Remove { name, force } => {
                commands::servers::remove(&mut config, name, force)?;
            }
            ServersCommands::SetDefault { name } => {
                commands::servers::set_default(&mut config, name)?;
            }
        },
        Some(Commands::SelfUpdate { force }) => {
            commands::update::self_update(force).await?;
        }
        Some(Commands::GenerateDocs) => {
            let markdown = clap_markdown::help_markdown::<Cli>();
            println!("{}", markdown);
        }
        None => {
            // No command provided, print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    // Background update check and notification.
    // Skipped for Homebrew installs, when RICOCHET_NO_UPDATE_CHECK is set,
    // or when skip_update_check is set in config (auto-set after repeated failures).
    update::maybe_notify_update(&config);
    if let Some(handle) = update::trigger_background_check(&config) {
        // Give the background check a short window to complete before exiting.
        // If it doesn't finish in time, the cache will be written on the next run.
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
    }

    Ok(())
}
