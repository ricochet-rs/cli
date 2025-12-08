use anyhow::Result;
use clap::{Parser, Subcommand};
use ricochet_cli::{OutputFormat, commands, config::Config};

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
    /// Generate markdown documentation (hidden command)
    #[command(hide = true)]
    GenerateDocs,
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
            // Tagged release or not in git repo - just show version
            println!("{}", version);
        } else {
            // Untagged build - append git hash and build date
            println!("{}-{} ({})", version, git_hash, build_date);
        }
        return Ok(());
    }

    // Load or initialize config
    let mut config = Config::load()?;

    // Override server if provided via CLI
    if let Some(server) = cli.server {
        config.server = Some(server);
    }

    // Execute command
    match cli.command {
        Some(Commands::Login { api_key }) => {
            commands::auth::login(&mut config, api_key).await?;
        }
        Some(Commands::Logout) => {
            commands::auth::logout(&mut config)?;
        }
        Some(Commands::Deploy {
            path,
            name,
            description,
        }) => {
            commands::deploy::deploy(&config, path, name, description, cli.debug).await?;
        }
        Some(Commands::List {
            content_type,
            active_only,
            sort,
        }) => {
            commands::list::list(&config, content_type, active_only, sort, cli.format).await?;
        }
        Some(Commands::Delete { id, force }) => {
            commands::delete::delete(&config, &id, force).await?;
        }
        Some(Commands::Invoke { id }) => {
            commands::invoke::invoke(&config, &id, cli.format).await?;
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

    Ok(())
}
