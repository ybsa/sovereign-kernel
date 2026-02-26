//! Sovereign Kernel CLI — the single entry point.
//!
//! Commands:
//! - `sovereign chat`  — interactive REPL
//! - `sovereign init`  — first-run setup
//! - `sovereign start` — start as daemon (future)
//! - `sovereign status` — check kernel status (future)

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod chat;
mod daemon;
mod init;

#[derive(Parser)]
#[command(name = "sovereign", version, about = "Sovereign Kernel — Agentic OS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive chat with the kernel
    Chat,
    /// First-run setup wizard
    Init,
    /// Start the kernel as a background daemon
    Start,
    /// Check kernel status
    Status,
    /// Stop the daemon
    Stop,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config
    let config = if let Some(ref path) = cli.config {
        sk_types::SovereignConfig::load(std::path::Path::new(path))?
    } else {
        // Look for config in default locations
        let default_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sovereign-kernel")
            .join("config.toml");

        if default_path.exists() {
            sk_types::SovereignConfig::load(&default_path)?
        } else {
            sk_types::SovereignConfig::default()
        }
    };

    match cli.command {
        Commands::Chat => chat::run(config).await?,
        Commands::Init => init::run().await?,
        Commands::Start => daemon::start(config).await?,
        Commands::Status => daemon::status().await?,
        Commands::Stop => daemon::stop().await?,
    }

    Ok(())
}
