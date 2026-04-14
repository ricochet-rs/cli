pub mod app;
pub mod client;
pub mod commands;
pub mod config;
pub mod item;
pub mod task;
pub mod update;
pub mod utils;

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}
