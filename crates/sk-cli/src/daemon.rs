//! Daemon management (start/stop/status).

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub async fn start(config: KernelConfig) -> anyhow::Result<()> {
    println!("⚡ Starting Sovereign...");

    // Initialize kernel
    let kernel = Arc::new(SovereignKernel::init(config.clone()).await?);

    // Start background background job scheduler
    kernel.start_background_services().await;

    // Resurrect crashed agents
    kernel.resurrect_all_active_agents().await;

    // Start the API Bridge server
    let k_server = kernel.clone();
    tokio::spawn(async move {
        if let Err(e) = k_server.start_api_server().await {
            tracing::error!("Failed to start API Bridge: {}", e);
        }
    });

    // Initialize the bridge
    let handle = Arc::new(crate::bridge::SovereignBridge::new(kernel.clone()));
    let router = Arc::new(sk_channels::router::AgentRouter::new());
    let mut manager = sk_channels::bridge::BridgeManager::new(handle.clone() as _, router);

    // Connect the channel delivery system to the kernel's background task scheduler
    kernel
        .set_delivery_handler(Arc::new(manager.delivery_handler()))
        .await;

    let mut bridged = false;

    // Start Telegram Channel if configured
    if let Some(tg_cfg) = &config.channels.telegram {
        if let Ok(token) = std::env::var(&tg_cfg.bot_token_env) {
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
    } else {
        // Fallback for missing config (from raw env) if Telegram wasn't explicitly configured in config but the token exists
        // Wait, since we are moving to configs, we might not need this. But let's keep it clean
        if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
             if !token.is_empty() {
                println!("⚡ Connecting to Telegram (legacy env)...");
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
    }

    // Start Discord Channel if configured
    if let Some(discord_cfg) = &config.channels.discord {
        if let Ok(token) = std::env::var(&discord_cfg.bot_token_env) {
            if !token.is_empty() {
                println!("⚡ Connecting to Discord...");
                let discord_adapter =
                    sk_channels::discord::DiscordAdapter::new(token, discord_cfg.allowed_guilds.clone(), discord_cfg.intents);
                manager
                    .start_adapter(Arc::new(discord_adapter))
                    .await
                    .map_err(|e| anyhow::anyhow!("Discord adapter start failed: {e}"))?;
                bridged = true;
            }
        }
    } else if let Ok(token) = std::env::var("DISCORD_BOT_TOKEN") {
        if !token.is_empty() {
            println!("⚡ Connecting to Discord (legacy env)...");
            let guild_ids: Vec<u64> = std::env::var("DISCORD_GUILD_IDS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
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

    // Start WhatsApp Channel if configured
    if let Some(wa_cfg) = &config.channels.whatsapp {
        if let Ok(access_token) = std::env::var(&wa_cfg.access_token_env) {
            let verify_token = std::env::var(&wa_cfg.verify_token_env).unwrap_or_default();
            if !access_token.is_empty() && !wa_cfg.phone_number_id.is_empty() {
                println!("⚡ Connecting to WhatsApp (Webhook Port: {})...", wa_cfg.webhook_port);
                let wa_adapter = sk_channels::whatsapp::WhatsAppAdapter::new(
                    access_token,
                    verify_token,
                    wa_cfg.phone_number_id.clone(),
                    wa_cfg.webhook_port,
                    wa_cfg.allowed_users.clone(),
                );
                manager
                    .start_adapter(Arc::new(wa_adapter))
                    .await
                    .map_err(|e| anyhow::anyhow!("WhatsApp adapter start failed: {e}"))?;
                bridged = true;
            }
        }
    }

    // Start Slack Channel if configured
    if let Some(slack_cfg) = &config.channels.slack {
        if let Ok(bot_token) = std::env::var(&slack_cfg.bot_token_env) {
            if let Ok(app_token) = std::env::var(&slack_cfg.app_token_env) {
                if !bot_token.is_empty() && !app_token.is_empty() {
                    println!("⚡ Connecting to Slack (Socket Mode)...");
                    let slack_adapter = sk_channels::slack::SlackAdapter::new(
                        app_token,
                        bot_token,
                        slack_cfg.allowed_channels.clone(),
                    );
                    manager
                        .start_adapter(Arc::new(slack_adapter))
                        .await
                        .map_err(|e| anyhow::anyhow!("Slack adapter start failed: {e}"))?;
                    bridged = true;
                }
            }
        }
    }

    // Start Signal Channel if configured
    if let Some(signal_cfg) = &config.channels.signal {
        if !signal_cfg.api_url.is_empty() && !signal_cfg.phone_number.is_empty() {
            println!("⚡ Connecting to Signal (Rest API: {})...", signal_cfg.api_url);
            let signal_adapter = sk_channels::adapters::signal::SignalAdapter::new(
                signal_cfg.api_url.clone(),
                signal_cfg.phone_number.clone(),
                signal_cfg.allowed_users.clone(),
            );
            manager
                .start_adapter(Arc::new(signal_adapter))
                .await
                .map_err(|e| anyhow::anyhow!("Signal adapter start failed: {e}"))?;
            bridged = true;
        }
    }

    // Start Matrix Channel if configured
    if let Some(matrix_cfg) = &config.channels.matrix {
        if let Ok(access_token) = std::env::var(&matrix_cfg.access_token_env) {
            if !access_token.is_empty() && !matrix_cfg.user_id.is_empty() {
                println!("⚡ Connecting to Matrix (Server: {})...", matrix_cfg.homeserver_url);
                let matrix_adapter = sk_channels::adapters::matrix::MatrixAdapter::new(
                    matrix_cfg.homeserver_url.clone(),
                    matrix_cfg.user_id.clone(),
                    access_token,
                    matrix_cfg.allowed_rooms.clone(),
                );
                manager
                    .start_adapter(Arc::new(matrix_adapter))
                    .await
                    .map_err(|e| anyhow::anyhow!("Matrix adapter start failed: {e}"))?;
                bridged = true;
            }
        }
    }

    // Start WebChat Channel if configured
    if let Some(wc_cfg) = &config.channels.webchat {
        println!("⚡ Starting WebChat WebSocket server on port {}...", wc_cfg.port);
        let wc_adapter = sk_channels::adapters::webchat::WebChatAdapter::new(
            wc_cfg.port,
            wc_cfg.default_agent.clone(),
        );
        manager
            .start_adapter(Arc::new(wc_adapter))
            .await
            .map_err(|e| anyhow::anyhow!("WebChat adapter start failed: {e}"))?;
        bridged = true;
    }

    if !bridged {
        println!("⚠️ No channels configured. Set TELEGRAM_BOT_TOKEN to connect.");
    } else {
        println!("⚡ Channel Bridge is running.");
    }

    // Start the Live Canvas dashboard (following Sovereign Kernel's run_daemon pattern)
    let dashboard_state = Arc::new(crate::dashboard::AppState {
        kernel: kernel.clone(),
        started_at: std::time::Instant::now(),
        telegram_connected: bridged,
    });

    let dashboard_port = 4200u16;
    let ds = dashboard_state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::dashboard::start_server(ds, dashboard_port).await {
            tracing::error!("Background dashboard server failed: {}", e);
        }
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

    // Check for double-start using sysinfo
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&pid_str) {
            if let Some(pid) = meta.get("pid").and_then(|v| v.as_u64()) {
                let system = sysinfo::System::new_all();
                if let Some(process) = system.process(sysinfo::Pid::from_u32(pid as u32)) {
                    let name = process.name().to_string_lossy().to_lowercase();
                    if name.contains("sovereign") {
                        anyhow::bail!("🟢 Daemon is already running (PID: {}).", pid);
                    }
                }
            }
        }
    }

    let meta = serde_json::json!({
        "pid": std::process::id(),
        "exe": std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        "cwd": std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        "start_time": chrono::Utc::now().to_rfc3339()
    });

    std::fs::write(&pid_path, serde_json::to_string_pretty(&meta).unwrap())
        .unwrap_or_else(|e| tracing::warn!("Failed to write PID file: {}", e));

    println!("⚡ Sovereign Kernel started (PID: {}).", std::process::id());

    // Config hot-reload watcher (from Sovereign Kernel's server.rs)
    let _k_reload = kernel.clone();
    tokio::spawn(async move {
        let env_path = std::path::PathBuf::from(".env");
        let mut last_modified = std::fs::metadata(&env_path).and_then(|m| m.modified()).ok();
        loop {
            sleep(Duration::from_secs(30)).await;
            let current = std::fs::metadata(&env_path).and_then(|m| m.modified()).ok();
            if current != last_modified && current.is_some() {
                last_modified = current;
                tracing::info!("Config file (.env) changed, attempting hot-reload...");
                // In a real production setup, we'd reload the config from disk here
                // For now, we just log it as the API handles the actual update
            }
        }
    });

    // Graceful shutdown on Ctrl+C and SIGTERM
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let mut sigusr1 = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Ctrl+C received, shutting down...");
        }
        _ = async {
            #[cfg(unix)]
            sigterm.recv().await;
            #[cfg(not(unix))]
            std::future::pending::<()>().await;
        } => {
            tracing::info!("SIGTERM received, shutting down...");
        }
        _ = async {
            loop {
                #[cfg(unix)]
                sigusr1.recv().await;
                #[cfg(not(unix))]
                std::future::pending::<()>().await;
                tracing::info!("SIGUSR1 received, triggering config reload...");
                // TODO: Trigger reload
            }
        } => {}
    }

    // Initiate Graceful Shutdown
    let coordinator =
        sk_engine::runtime::graceful_shutdown::ShutdownCoordinator::new(Default::default());
    coordinator.initiate();

    if let Err(e) = kernel.shutdown().await {
        tracing::error!("Error during kernel shutdown: {}", e);
    }

    let _ = std::fs::remove_file(&pid_path);
    println!("\n⚡ Sovereign Kernel stopped.");
    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    println!("⚡ Sovereign Kernel status:");
    let pid_path = std::path::PathBuf::from("sovereign.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&pid_str) {
            if let Some(pid) = meta.get("pid").and_then(|v| v.as_u64()) {
                let system = sysinfo::System::new_all();
                if let Some(process) = system.process(sysinfo::Pid::from_u32(pid as u32)) {
                    let name = process.name().to_string_lossy().to_lowercase();
                    if name.contains("sovereign") {
                        println!("🟢 RUNNING (PID: {})", pid);
                        return Ok(());
                    }
                }
            }
        }
    }
    println!("🔴 STOPPED");
    Ok(())
}

pub async fn stop() -> anyhow::Result<()> {
    println!("⚡ Stopping Sovereign Kernel...");
    let pid_path = std::path::PathBuf::from("sovereign.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&pid_str) {
            if let Some(pid) = meta.get("pid").and_then(|v| v.as_u64()) {
                let system = sysinfo::System::new_all();
                if let Some(process) = system.process(sysinfo::Pid::from_u32(pid as u32)) {
                    let name = process.name().to_string_lossy().to_lowercase();
                    if name.contains("sovereign") {
                        if process.kill() {
                            let _ = std::fs::remove_file(&pid_path);
                            println!("✅ Stopped successfully.");
                            return Ok(());
                        } else {
                            anyhow::bail!("❌ Failed to kill the daemon process.");
                        }
                    }
                }
            }
        }
    }

    // Cleanup stale PID file if present
    if pid_path.exists() {
        tracing::info!("Cleaning up stale PID file.");
        let _ = std::fs::remove_file(&pid_path);
    }

    println!("⚠️ Daemon does not appear to be running.");
    Ok(())
}
