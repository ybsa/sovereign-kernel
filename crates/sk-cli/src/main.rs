//! Sovereign Kernel CLI — the single entry point.

use clap::{Args, Parser, Subcommand};

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
mod memory;
mod middleware;
mod openai_compat;
mod run;
mod status;
mod treasury;

#[derive(Parser, Debug)]
#[command(name = "sovereign", version, about = "Sovereign Kernel — Agentic OS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose logging (debug level)
    #[arg(short, long)]
    verbose: bool,

    /// Write structured logs to this file (JSON format)
    #[arg(long)]
    log_file: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive chat with the kernel
    Chat(ChatArgs),
    /// First-run setup wizard
    Init,
    /// Start the kernel as a background daemon
    Start(StartArgs),
    /// Run a task autonomously
    #[command(alias = "do")]
    Run(RunArgs),
    /// Check kernel status
    Status,
    /// Stop the daemon or a specific agent
    Kill(KillArgs),
    /// Stop the daemon (Legacy)
    Stop,
    /// Manage autonomous hands (list, activate, deactivate, status)
    Hands(HandsArgs),
    /// Audit Trail commands
    Audit(AuditArgs),
    /// Open the terminal web dashboard at http://localhost:PORT
    Dashboard(DashboardArgs),
    /// Manage Village inhabitants (list, inspect, stop, remove)
    Agents(AgentArgs),
    /// Manage MCP server connections (list, add, remove)
    Mcp(McpArgs),
    /// Manage channel bridges (list, info)
    Channels(ChannelArgs),
    /// Run system diagnostics and health checks
    Doctor,
    /// Manage budgets and track LLM costs
    Treasury {
        #[command(subcommand)]
        command: treasury::TreasuryCommands,
    },
    /// Manage and export agent memory
    Memory {
        #[command(subcommand)]
        command: memory::MemoryCommands,
    },
}

#[derive(Args, Debug)]
struct ChatArgs {
    /// Maximum LLM loop iterations for this session
    #[arg(long)]
    max_iterations: Option<u32>,
    /// Maximum LLM tokens for this session
    #[arg(long)]
    max_tokens: Option<u32>,
    /// Maximum budget in USD for this session
    #[arg(long)]
    budget_usd: Option<f64>,
}

#[derive(Args, Debug)]
struct StartArgs {
    /// Detach the process and run in background
    #[arg(short, long)]
    detach: bool,
}

#[derive(Args, Debug)]
struct RunArgs {
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
}

#[derive(Args, Debug)]
struct KillArgs {
    /// Agent ID to kill (optional, stops daemon if omitted)
    id: Option<String>,
}

#[derive(Args, Debug)]
struct HandsArgs {
    /// Action: list, activate, deactivate, status
    #[arg(default_value = "list")]
    action: String,
    /// Additional arguments (hand name, instance ID, etc.)
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Args, Debug)]
struct AuditArgs {
    /// Action: logs, verify
    #[arg(default_value = "logs")]
    action: String,
    /// Additional arguments
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Args, Debug)]
struct DashboardArgs {
    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,
    /// Do not open browser automatically
    #[arg(long)]
    no_open: bool,
}

#[derive(Args, Debug)]
struct AgentArgs {
    /// Action: list, inspect, stop, remove
    #[arg(default_value = "list")]
    action: String,
    /// Agent ID or name
    id: Option<String>,
}

#[derive(Args, Debug)]
struct McpArgs {
    /// Action: list, add, remove
    #[arg(default_value = "list")]
    action: String,
    /// Additional arguments (name, command, args...)
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Args, Debug)]
struct ChannelArgs {
    /// Action: list, info
    #[arg(default_value = "list")]
    action: String,
    /// Target channel (e.g., telegram, discord)
    channel: Option<String>,
}

fn main() {
    // Spawn a thread with a larger stack size for the CLI
    // This is a workaround for STATUS_STACK_OVERFLOW on Windows due to deep clap recursion
    const STACK_SIZE: usize = 4 * 1024 * 1024; // 4MB

    let child = std::thread::Builder::new()
        .name("cli-main".into())
        .stack_size(STACK_SIZE)
        .spawn(|| {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async_main())
        })
        .expect("Failed to spawn CLI thread");

    if let Err(e) = child.join() {
        eprintln!("THREAD PANIC: {:?}", e);
        std::process::exit(1);
    }
}

async fn async_main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Parse CLI arguments
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
        let local_path = std::path::PathBuf::from("config.toml");
        let local_sub_path = std::path::PathBuf::from("sovereign-kernel").join("config.toml");
        let user_path = dirs::config_dir()
            .map(|d| d.join("sovereign-kernel").join("config.toml"));

        if local_path.exists() {
            sk_types::KernelConfig::load(&local_path)?
        } else if local_sub_path.exists() {
            sk_types::KernelConfig::load(&local_sub_path)?
        } else if let Some(p) = user_path {
            if p.exists() {
                sk_types::KernelConfig::load(&p)?
            } else {
                sk_types::KernelConfig::default()
            }
        } else {
            sk_types::KernelConfig::default()
        }
    };

    match cli.command {
        Commands::Chat(args) => {
            let mut cfg = config;
            if let Some(i) = args.max_iterations {
                cfg.max_iterations_per_task = i;
            }
            if let Some(t) = args.max_tokens {
                cfg.max_tokens_per_task = t;
            }
            if let Some(b) = args.budget_usd {
                cfg.total_token_budget_usd_cents = Some((b * 100.0) as u64);
            }
            chat::run(cfg).await?
        }
        Commands::Init => init::run().await?,
        Commands::Start(args) => daemon::start(config, args.detach).await?,
        Commands::Run(args) => {
            let mut cfg = config;
            if let Some(i) = args.max_iterations {
                cfg.max_iterations_per_task = i;
            }
            if let Some(t) = args.max_tokens {
                cfg.max_tokens_per_task = t;
            }
            if let Some(b) = args.budget_usd {
                cfg.total_token_budget_usd_cents = Some((b * 100.0) as u64);
            }
            run::execute(cfg, &args.task, &args.mode, args.schedule).await?
        }
        Commands::Status => status::print_status().await?,
        Commands::Kill(args) => {
            if let Some(agent_id) = args.id {
                println!("Killing agent {}...", agent_id);
            } else {
                daemon::stop().await?;
            }
        }
        Commands::Stop => daemon::stop().await?,
        Commands::Hands(args) => hands::run(config, &args.action, &args.args).await?,
        Commands::Audit(args) => audit::run(config, &args.action, &args.args).await?,
        Commands::Dashboard(args) => {
            let kernel = std::sync::Arc::new(sk_kernel::SovereignKernel::init(config).await?);
            let state = std::sync::Arc::new(dashboard::AppState {
                kernel,
                started_at: std::time::Instant::now(),
                telegram_connected: false,
            });
            let url = format!("http://localhost:{}", args.port);
            println!("⚡ Sovereign Kernel Dashboard → {url}");
            if !args.no_open {
                let _ = open::that(&url);
            }
            dashboard::start_server(state, args.port).await?;
        }
        Commands::Agents(args) => agents::run(config, &args.action, args.id).await?,
        Commands::Mcp(args) => mcp::run(config, &args.action, &args.args).await?,
        Commands::Doctor => doctor::run(&config).await?,
        Commands::Treasury { command } => {
            let api_url = config.api_listen.clone();
            let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
            let args = treasury::TreasuryArgs { command };
            treasury::run(args, &format!("http://{}", api_url), api_key.as_deref()).await?;
        }
        Commands::Channels(args) => {
            channels::run(config, &args.action, args.channel.as_deref()).await?;
        }
        Commands::Memory { command } => {
            memory::handle_memory_command(command).await?;
        }
    }

    Ok(())
}
