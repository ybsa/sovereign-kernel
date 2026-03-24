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

mod agents;
mod audit;
mod bridge;
mod channels;
mod chat;
mod daemon;
mod dashboard;
mod doctor;
mod hands;
mod init;
mod mcp;
mod middleware;
mod openai_compat;
mod run;
mod status;
mod treasury;

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
    Chat {
        /// Maximum LLM loop iterations for this session
        #[arg(long)]
        max_iterations: Option<u32>,
        /// Maximum LLM tokens for this session
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Maximum budget in USD for this session
        #[arg(long)]
        budget_usd: Option<f64>,
    },
    /// First-run setup wizard
    Init,
    /// Start the kernel as a background daemon
    Start {
        /// Detach the process and run in background
        #[arg(short, long)]
        detach: bool,
    },
    /// Run a task autonomously
    #[command(alias = "do")]
    Run {
        /// The task description (natural language)
        task: String,
        /// Execution mode: auto, safe, unrestricted
        #[arg(short, long, default_value = "auto")]
        mode: String,
        /// Optional schedule (cron expression)
        #[arg(short, long)]
        schedule: Option<String>,
        /// Maximum LLM loop iterations for this task
        #[arg(long)]
        max_iterations: Option<u32>,
        /// Maximum LLM tokens for this task
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Maximum budget in USD for this task
        #[arg(long)]
        budget_usd: Option<f64>,
    },
    /// Check kernel status
    Status,
    /// Stop the daemon or a specific agent
    Kill {
        /// Agent ID to kill (optional, stops daemon if omitted)
        id: Option<String>,
    },
    /// Stop the daemon (Legacy, use 'kill' without ID)
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
    /// Manage Village inhabitants (list, inspect, stop, remove)
    Agents {
        /// Action: list, inspect, stop, remove
        #[arg(default_value = "list")]
        action: String,
        /// Agent ID or name
        id: Option<String>,
    },
    /// Manage MCP server connections (list, add, remove)
    Mcp {
        /// Action: list, add, remove
        #[arg(default_value = "list")]
        action: String,
        /// Additional arguments (name, command, args...)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Manage channel bridges (list, info)
    Channels {
        /// Action: list, info
        #[arg(default_value = "list")]
        action: String,
        /// Target channel (e.g., telegram, discord)
        channel: Option<String>,
    },
    /// Run system diagnostics and health checks
    Doctor,
    /// Manage budgets and track LLM costs
    Treasury(treasury::TreasuryArgs),
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

    let mut _log_guard = None;

    if let Some(ref log_path) = cli.log_file {
        let path = std::path::Path::new(log_path);
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let prefix = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("sovereign.log"));

        let file_appender = tracing_appender::rolling::daily(dir, prefix);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        _log_guard = Some(guard); // Keep the worker guard alive

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
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
        Commands::Chat {
            max_iterations,
            max_tokens,
            budget_usd,
        } => {
            let mut cfg = config;
            if let Some(i) = max_iterations {
                cfg.max_iterations_per_task = i;
            }
            if let Some(t) = max_tokens {
                cfg.max_tokens_per_task = t;
            }
            if let Some(b) = budget_usd {
                cfg.total_token_budget_usd_cents = Some((b * 100.0) as u64);
            }
            chat::run(cfg).await?
        }
        Commands::Init => init::run().await?,
        Commands::Start { detach } => daemon::start(config, detach).await?,
        Commands::Run {
            task,
            mode,
            schedule,
            max_iterations,
            max_tokens,
            budget_usd,
        } => {
            let mut cfg = config;
            if let Some(i) = max_iterations {
                cfg.max_iterations_per_task = i;
            }
            if let Some(t) = max_tokens {
                cfg.max_tokens_per_task = t;
            }
            if let Some(b) = budget_usd {
                cfg.total_token_budget_usd_cents = Some((b * 100.0) as u64);
            }
            run::execute(cfg, &task, &mode, schedule).await?
        }
        Commands::Status => status::print_status().await?,
        Commands::Kill { id } => {
            if let Some(agent_id) = id {
                // TODO: Implement kill agent
                println!("Killing agent {}...", agent_id);
            } else {
                daemon::stop().await?;
            }
        }
        Commands::Stop => daemon::stop().await?,
        Commands::Hands { action, args } => hands::run(config, &action, &args).await?,
        Commands::Audit { action, args } => audit::run(config, &action, &args).await?,
        Commands::Dashboard { port, no_open } => {
            let kernel = std::sync::Arc::new(sk_kernel::SovereignKernel::init(config).await?);
            let state = std::sync::Arc::new(dashboard::AppState {
                kernel,
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
        Commands::Agents { action, id } => agents::run(config, &action, id).await?,
        Commands::Mcp { action, args } => mcp::run(config, &action, &args).await?,
        Commands::Doctor => doctor::run(&config).await?,
        Commands::Treasury(args) => {
            let api_url = config.api_listen.clone();
            let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
            treasury::run(args, &format!("http://{}", api_url), api_key.as_deref()).await?;
        }
        Commands::Channels { action, channel } => {
            channels::run(config, &action, channel.as_deref()).await?;
        }
    }

    Ok(())
}
