/*!
 * LMO - LMOxide Command-Line Interface
 *
 * A comprehensive CLI for model management and chat completions.
 */

mod cli;
mod commands;
mod config;
mod error;
mod output;
mod utils;

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

use cli::{Cli, Commands};
use config::CliConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration
    let config = CliConfig::load().unwrap_or_default();

    info!("LMO CLI starting");

    // Handle commands
    match cli.command {
        Commands::Models(cmd) => commands::models::handle(cmd, &config).await,
        Commands::Chat(cmd) => commands::chat::handle(cmd, &config, cli.verbose).await,
        Commands::Load(cmd) => commands::load::handle(cmd, &config).await,
        Commands::Unload(cmd) => commands::unload::handle(cmd, &config).await,
        Commands::Status(cmd) => commands::status::handle(cmd, &config).await,
        Commands::Config(cmd) => commands::config::handle(cmd, &config).await,
        Commands::Health(cmd) => commands::health::handle(cmd, &config).await,
        Commands::Download(cmd) => commands::download::handle(cmd, &config).await,
    }
}
