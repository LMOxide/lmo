/*!
 * CLI Argument Parsing
 * 
 * Command-line interface definitions using clap.
 */

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "lmo")]
#[command(about = "LMOxide CLI - Model management and chat completions")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "LMOxide Team")]
pub struct Cli {
    /// Server URL
    #[arg(short = 's', long, env = "LMO_SERVER_URL")]
    pub server_url: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format (json, table, yaml)
    #[arg(short = 'o', long, global = true, default_value = "table")]
    pub output: String,

    /// Disable colors in output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List and search available models
    Models(ModelsCommand),
    
    /// Interactive chat with loaded models
    Chat(ChatCommand),
    
    /// Load a model for inference
    Load(LoadCommand),
    
    /// Unload a loaded model
    Unload(UnloadCommand),
    
    /// Show status of loaded models
    Status(StatusCommand),
    
    /// Manage CLI configuration
    Config(ConfigCommand),
    
    /// Check server health
    Health(HealthCommand),
    
    /// Download a model from remote repository
    Download(DownloadCommand),
}

#[derive(Parser, Debug)]
pub struct ModelsCommand {
    /// Show local models only
    #[arg(long)]
    pub local: bool,

    /// Search term to filter models
    #[arg(short, long)]
    pub search: Option<String>,

    /// Filter by author/organization
    #[arg(short, long)]
    pub author: Option<String>,

    /// Filter by tags (e.g., gguf, mlx)
    #[arg(short, long)]
    pub tags: Option<String>,

    /// Filter by pipeline type
    #[arg(short, long)]
    pub pipeline: Option<String>,

    /// Maximum number of models to show
    #[arg(short, long, default_value = "20")]
    pub limit: u32,

    /// Sort by field (downloads, author, created, size)
    #[arg(long, default_value = "downloads")]
    pub sort: String,

    /// Sort direction (asc, desc)
    #[arg(long, default_value = "desc")]
    pub direction: String,
}

#[derive(Parser, Debug)]
pub struct ChatCommand {
    /// Model to chat with (if not specified, will prompt to select)
    #[arg(short, long)]
    pub model: Option<String>,

    /// System prompt to use
    #[arg(short, long)]
    pub system: Option<String>,

    /// Single message to send (non-interactive mode)
    #[arg(short = 'i', long)]
    pub input: Option<String>,

    /// Maximum tokens to generate
    #[arg(long, default_value = "1000")]
    pub max_tokens: u32,

    /// Temperature for sampling (0.0 to 2.0)
    #[arg(short, long, default_value = "0.7")]
    pub temperature: f32,

    /// Enable streaming output
    #[arg(long)]
    pub stream: bool,

    /// Load conversation history from file
    #[arg(long)]
    pub load_history: Option<String>,

    /// Save conversation history to file
    #[arg(long)]
    pub save_history: Option<String>,
}

#[derive(Parser, Debug)]
pub struct LoadCommand {
    /// Model identifier to load
    pub model_id: String,

    /// Specific filename to load (optional)
    #[arg(short = 'F', long)]
    pub filename: Option<String>,

    /// Force reload if already loaded
    #[arg(short, long)]
    pub force: bool,

    /// Show loading progress
    #[arg(short, long)]
    pub progress: bool,
}

#[derive(Parser, Debug)]
pub struct UnloadCommand {
    /// Model instance ID to unload
    pub instance_id: String,

    /// Force unload even if in use
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct StatusCommand {
    /// Show detailed status information
    #[arg(short, long)]
    pub detailed: bool,

    /// Show only specific model
    #[arg(short, long)]
    pub model: Option<String>,

    /// Refresh interval in seconds (for watch mode)
    #[arg(short, long)]
    pub refresh: Option<u64>,
}

#[derive(Parser, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Initialize configuration with defaults
    Init,
    
    /// Reset configuration to defaults
    Reset,
}

#[derive(Parser, Debug)]
pub struct HealthCommand {
    /// Show detailed health information
    #[arg(short, long)]
    pub detailed: bool,

    /// Check specific health aspects (server, models, memory)
    #[arg(short, long)]
    pub check: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct DownloadCommand {
    /// Model name to download (e.g., "microsoft/DialoGPT-small")
    pub model_name: String,

    /// Model format hint (gguf, mlx, safetensors, candle)
    #[arg(short, long)]
    pub format: Option<String>,

    /// Show download progress
    #[arg(short, long)]
    pub progress: bool,

    /// Force re-download if already exists
    #[arg(long)]
    pub force: bool,

    /// Custom download directory (optional)
    #[arg(short, long)]
    pub directory: Option<String>,
}