//! Daemon management (start/stop/status).

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use std::process::Command;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub async fn start(config: KernelConfig, detach: bool) -> anyhow::Result<()> {
    if detach {
        println!("🚀 Launching Sovereign in detached background mode...");
        let exe = std::env::current_exe()?;

        // Define the arguments to re-execute without --detach
        let log_dir = config.data_dir.join("logs");
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir)?;
        }
        let log_file = log_dir.join("daemon.json").to_string_lossy().to_string();

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            const DETACHED_PROCESS: u32 = 0x00000008;
            Command::new(exe)
                .arg("--log-file")
                .arg(&log_file)
                .arg("start")
                .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
                .spawn()?;
        }
        #[cfg(not(windows))]
        {
            // On Unix, redirecting stdout/stderr to bitbucket helps detach
            Command::new(exe)
                .arg("--log-file")
                .arg(&log_file)
                .arg("start")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;
        }

        println!("✅ Daemon successfully detached and started.");
        println!("📄 Logs will be written to: {}", log_file);
        return Ok(());
    }

    println!("⚡ Starting Sovereign...");

    // Initialize kernel
    let kernel = Arc::new(SovereignKernel::init(config.clone()).await?);

    // Start background background job scheduler
    kernel.start_background_services().await;

    // Resurrect crashed agents
    kernel.resurrect_all_active_agents().await;

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
    let mut sigusr1 =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())?;

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
