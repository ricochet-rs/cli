pub mod client;
pub mod commands;
pub mod config;
pub mod utils;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}
