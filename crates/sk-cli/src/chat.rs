//! Interactive chat REPL.

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use sk_types::AgentId;
use tracing::info;

/// Run the interactive chat REPL.
pub async fn run(config: KernelConfig) -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("  ⚡ Sovereign Kernel v{}", env!("CARGO_PKG_VERSION"));
    println!("  Type 'exit' or 'quit' to leave, 'clear' to reset");

    match config.execution_mode {
        sk_types::config::ExecutionMode::Sandbox => {
            println!("  [🛡️ Mode: SANDBOX - File and command safety enabled]");
        }
        sk_types::config::ExecutionMode::Unrestricted => {
            println!("  [🚨 Mode: UNRESTRICTED - WARNING: Agent has full system access!]");
        }
    }
    println!("═══════════════════════════════════════════════════════\n");

    // Initialize kernel and wrap in Arc so we can call run_agent
    let kernel = std::sync::Arc::new(SovereignKernel::init(config).await?);

    // Create a default agent ID for this chat session
    let agent_id = AgentId::new();
    info!(agent = %agent_id, "Chat session started");

    println!("\nSovereign: [Connected to {}]\n", kernel.model_name);

    // Load existing session or create new one
    let mut session = if let Ok(entries) = kernel.memory.sessions.list_for_agent(agent_id) {
        if let Some((latest_id, _, _)) = entries.first() {
            if let Ok(Some(loaded_session)) = kernel.memory.sessions.load(*latest_id) {
                loaded_session
            } else {
                sk_types::Session::new(agent_id)
            }
        } else {
            sk_types::Session::new(agent_id)
        }
    } else {
        sk_types::Session::new(agent_id)
    };

    // Chat loop
    loop {
        // Read user input
        print!("You: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();

        if kernel.safety.has_pending(&agent_id) {
            println!("  [⏱️ Waiting for approval... default deny in 60s]");
            let res = tokio::time::timeout(std::time::Duration::from_secs(60), async {
                tokio::task::spawn_blocking(|| {
                    let mut s = String::new();
                    let _ = std::io::stdin().read_line(&mut s);
                    s
                })
                .await
                .unwrap_or_default()
            })
            .await;

            match res {
                Ok(s) => input = s,
                Err(_) => {
                    println!("\n  [⏱️ Timeout reached! Auto-denying action.]\n");
                    kernel.safety.deny_last_for_agent(&agent_id);
                    input = "deny".to_string(); // Feed deny back to the loop
                }
            }
        } else {
            std::io::stdin().read_line(&mut input)?;
        }

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input.to_lowercase().as_str() {
            "exit" | "quit" | "/exit" | "/quit" => {
                println!("\n⚡ Sovereign Kernel signing off. Until next time.");
                break;
            }
            "clear" | "/clear" => {
                session = sk_types::Session::new(agent_id);
                println!("Session cleared.\n");
                continue;
            }
            "approve" | "yes" | "y" => {
                if kernel.safety.approve_last_for_agent(&agent_id) {
                    println!("🛡️ Action approved. Re-submitting to Sovereign...");
                }
            }
            "deny" | "no" | "n" => {
                if kernel.safety.deny_last_for_agent(&agent_id) {
                    println!("🛡️ Action denied. Blocked signature cleared.");
                }
            }
            _ => {}
        }

        match kernel.run_agent(&mut session, input).await {
            Ok(result) => {
                println!("\nSovereign: {}\n", result.response);
            }
            Err(e) => {
                println!("\nSovereign Error: {}\n", e);
            }
        }
    }

    kernel.shutdown().await?;
    Ok(())
}
