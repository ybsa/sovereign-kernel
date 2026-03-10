//! Sovereign Kernel CLI — the single entry point.
//!
//! Commands:
//! - `sovereign chat`      — interactive REPL
//! - `sovereign init`      — first-run setup
//! - `sovereign start`     — start as daemon
//! - `sovereign status`    — check kernel status
//! - `sovereign dashboard` — open the terminal web dashboard
//! - `sovereign hands`     — manage autonomous hands
//! - `sovereign audit`     — view audit logs

use clap::{Parser, Subcommand};
// Removed unused EnvFilter import

mod audit;
mod bridge;
mod chat;
mod daemon;
mod dashboard;
mod hands;
mod init;
mod middleware;
mod openai_compat;

#[derive(Parser)]
#[command(name = "sovereign", version, about = "Sovereign Kernel — Agentic OS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Enable verbose logging (debug level)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Write structured logs to this file (JSON format)
    #[arg(long, global = true)]
    log_file: Option<String>,
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
    /// Manage autonomous hands (list, activate, deactivate, status)
    Hands {
        /// Action: list, activate, deactivate, status
        #[arg(default_value = "list")]
        action: String,
        /// Additional arguments (hand name, instance ID, etc.)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Audit Trail commands
    Audit {
        /// Action: logs, verify
        #[arg(default_value = "logs")]
        action: String,
        /// Additional arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Open the terminal web dashboard at http://localhost:PORT
    Dashboard {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Do not open browser automatically
        #[arg(long)]
        no_open: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };

    if let Some(ref log_path) = cli.log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .unwrap_or_else(|e| panic!("Failed to open log file {}: {}", log_path, e));

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file)
            .json() // Write as JSON to the log file for structure
            .with_ansi(false);

        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(tracing_subscriber::fmt::layer()) // And write standard tracing to stdout
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    // Load config
    let config = if let Some(ref path) = cli.config {
        sk_types::KernelConfig::load(std::path::Path::new(path))?
    } else {
        // Look for config in default locations
        let default_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sovereign-kernel")
            .join("config.toml");

        if default_path.exists() {
            sk_types::KernelConfig::load(&default_path)?
        } else {
            sk_types::KernelConfig::default()
        }
    };

    match cli.command {
        Commands::Chat => chat::run(config).await?,
        Commands::Init => init::run().await?,
        Commands::Start => daemon::start(config).await?,
        Commands::Status => daemon::status().await?,
        Commands::Stop => daemon::stop().await?,
        Commands::Hands { action, args } => hands::run(&action, &args).await?,
        Commands::Audit { action, args } => audit::run(config, &action, &args).await?,
        Commands::Dashboard { port, no_open } => {
            let state = std::sync::Arc::new(dashboard::AppState {
                hand_registry: std::sync::Mutex::new({
                    let mut reg = sk_hands::registry::HandRegistry::new();
                    reg.load_bundled();
                    reg
                }),
                started_at: std::time::Instant::now(),
                telegram_connected: false,
            });
            let url = format!("http://localhost:{port}");
            println!("⚡ Sovereign Kernel Dashboard → {url}");
            println!("   Press Ctrl+C to stop.");
            if !no_open {
                let _ = open::that(&url);
            }
            if let Err(e) = dashboard::start_server(state, port).await {
                tracing::error!("Dashboard server failed to start: {}", e);
            }
        }
    }

    Ok(())
}
