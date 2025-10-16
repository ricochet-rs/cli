// Standalone binary to generate CLI documentation
// This is much faster than building the full ricochet binary
// because it doesn't need all the runtime dependencies like
// tokio, reqwest, ricochet-core, etc.

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

#[derive(Parser)]
#[command(name = "ricochet")]
#[command(about = "Ricochet CLI", long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Server URL (can also be set with RICOCHET_SERVER environment variable)
    #[arg(global = true, short = 'S', long, env = "RICOCHET_SERVER", help_heading = "Global Options")]
    server: Option<String>,

    /// Output format
    #[arg(global = true, short = 'F', long, default_value = "table", value_enum, help_heading = "Global Options")]
    format: OutputFormat,

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
    /// Show configuration
    Config {
        /// Show full configuration including sensitive values
        #[arg(short = 'A', long)]
        show_all: bool,
    },
}

fn main() {
    let mut markdown = clap_markdown::help_markdown::<Cli>();

    // Remove leading blank lines
    markdown = markdown.trim_start().to_string();

    // Ensure exactly one trailing newline
    markdown = markdown.trim_end().to_string();
    markdown.push('\n');

    print!("{}", markdown);
}
