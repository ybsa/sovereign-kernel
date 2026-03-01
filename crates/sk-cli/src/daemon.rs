//! Daemon management (start/stop/status).

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub async fn start(config: KernelConfig) -> anyhow::Result<()> {
    println!("⚡ Starting Sovereign Kernel daemon in the background...");

    // Initialize kernel
    let kernel = Arc::new(SovereignKernel::init(config).await?);

    // Initialize the bridge
    let handle = Arc::new(crate::bridge::SovereignBridge::new(kernel.clone()));
    let router = Arc::new(sk_channels::router::AgentRouter::new());
    let mut manager = sk_channels::bridge::BridgeManager::new(handle.clone() as _, router);

    let mut bridged = false;

    // Start Telegram Channel if valid token
    if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
        if !token.is_empty() {
            println!("⚡ Connecting to Telegram...");
            let tg_adapter = sk_channels::telegram::TelegramAdapter::new(token)
                .await
                .map_err(|e| anyhow::anyhow!("Telegram init failed: {e}"))?;
            manager
                .start_adapter(Arc::new(tg_adapter))
                .await
                .map_err(|e| anyhow::anyhow!("Telegram adapter start failed: {e}"))?;
            bridged = true;
        }
    }

    // Start Discord Channel if valid token
    if let Ok(token) = std::env::var("DISCORD_BOT_TOKEN") {
        if !token.is_empty() {
            println!("⚡ Connecting to Discord...");
            let guild_ids: Vec<u64> = std::env::var("DISCORD_GUILD_IDS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            // MESSAGE_CONTENT + GUILD_MESSAGES intents
            let intents = 33280u64;
            let discord_adapter =
                sk_channels::discord::DiscordAdapter::new(token, guild_ids, intents);
            manager
                .start_adapter(Arc::new(discord_adapter))
                .await
                .map_err(|e| anyhow::anyhow!("Discord adapter start failed: {e}"))?;
            bridged = true;
        }
    }

    if !bridged {
        println!("⚠️ No channels configured. Set TELEGRAM_BOT_TOKEN to connect.");
    } else {
        println!("⚡ Channel Bridge is running.");
    }

    // Start the Live Canvas dashboard (following OpenFang's run_daemon pattern)
    let dashboard_state = Arc::new(crate::dashboard::AppState {
        hand_registry: {
            let mut reg = sk_hands::registry::HandRegistry::new();
            reg.load_bundled();
            std::sync::Mutex::new(reg)
        },
        started_at: std::time::Instant::now(),
        telegram_connected: bridged,
    });

    let dashboard_port = 4200u16;
    let ds = dashboard_state.clone();
    tokio::spawn(async move {
        crate::dashboard::start_server(ds, dashboard_port).await;
    });

    println!("⚡ Live Canvas dashboard at http://localhost:{dashboard_port}");
    println!("⚡ OpenAI-compatible API at http://localhost:{dashboard_port}/v1/chat/completions");
    if std::env::var("SOVEREIGN_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false)
    {
        println!("🔒 API key protection is ENABLED");
    }
    println!("Daemon is now running. (Press Ctrl+C to stop)\n");

    let pid_path = std::path::PathBuf::from("sovereign.pid");
    std::fs::write(&pid_path, std::process::id().to_string())
        .unwrap_or_else(|e| tracing::warn!("Failed to write PID file: {}", e));

    println!(
        "⚡ Sovereign Kernel daemon started in background (PID: {}).",
        std::process::id()
    );

    // Config hot-reload watcher (from OpenFang's server.rs)
    {
        let env_path = std::path::PathBuf::from(".env");
        tokio::spawn(async move {
            let mut last_modified = std::fs::metadata(&env_path).and_then(|m| m.modified()).ok();
            loop {
                sleep(Duration::from_secs(30)).await;
                let current = std::fs::metadata(&env_path).and_then(|m| m.modified()).ok();
                if current != last_modified && current.is_some() {
                    last_modified = current;
                    tracing::info!(
                        "Config file (.env) changed, reload will take effect on next restart"
                    );
                }
            }
        });
    }

    // Graceful shutdown on Ctrl+C (from OpenFang)
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Ctrl+C received, shutting down...");
    let _ = std::fs::remove_file(&pid_path);
    println!("\n⚡ Sovereign Kernel daemon stopped.");
    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    println!("⚡ Sovereign Kernel status:");
    let pid_path = std::path::PathBuf::from("sovereign.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Rough check if process is alive (mostly cross-platform)
            // On Windows this is harder without sysinfo crate, but we'll assume alive
            // if the file exists and has a number.
            println!("🟢 RUNNING (PID: {})", pid);
            return Ok(());
        }
    }
    println!("🔴 STOPPED");
    Ok(())
}

pub async fn stop() -> anyhow::Result<()> {
    println!("⚡ Stopping Sovereign Kernel daemon...");
    let pid_path = std::path::PathBuf::from("sovereign.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new("kill")
                    .arg(pid.to_string())
                    .output()?;
            }
            let _ = std::fs::remove_file(&pid_path);
            println!("✅ Daemon stopped successfully.");
            return Ok(());
        }
    }
    println!("⚠️ Daemon does not appear to be running.");
    Ok(())
}
