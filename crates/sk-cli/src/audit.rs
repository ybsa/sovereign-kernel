//! Audit Trail CLI commands.
//!
//! Provides the ability to view the cryptographic audit log and verify
//! the Merkle chain integrity.

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;

/// Run an audit subcommand.
pub async fn run(config: KernelConfig, action: &str, _args: &[String]) -> anyhow::Result<()> {
    // Initialize kernel to get access to memory
    let kernel = SovereignKernel::init(config).await?;

    match action {
        "logs" | "log" => {
            println!("═══════════════════════════════════════════════════════");
            println!("  🛡️ Sovereign Kernel Cryptographic Audit Trail");
            println!("═══════════════════════════════════════════════════════");

            let logs = kernel.memory.audit.get_recent_logs(50)?;

            if logs.is_empty() {
                println!("No audit logs found.");
                return Ok(());
            }

            for entry in logs.iter().rev() {
                println!("[{}] Agent: {}", entry.timestamp, entry.agent_id);
                println!("  Mode:   {}", entry.execution_mode);
                println!("  Action: {}", entry.action_type);

                // Pretty print the JSON payload if possible
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&entry.action_data) {
                    println!(
                        "  Data:   {}",
                        serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| entry.action_data.clone())
                    );
                } else {
                    println!("  Data:   {}", entry.action_data);
                }

                println!("  Hash:   {}...", &entry.hash[0..16]);
                println!("-------------------------------------------------------");
            }

            println!("Found {} entries.", logs.len());
        }
        "verify" | "check" => {
            println!("Verifying Merkle chain integrity...");

            match kernel.memory.audit.verify_chain() {
                Ok(_) => {
                    println!("✅ [SUCCESS] The audit chain is fully intact and has not been tampered with.");
                }
                Err(e) => {
                    println!("❌ [CRITICAL ALERT] The audit chain is broken!");
                    println!("Reason: {}", e);
                }
            }
        }
        _ => {
            println!("Unknown audit command: {}", action);
            println!("Usage:");
            println!("  sovereign audit logs   - View recent action history");
            println!("  sovereign audit verify - Check Merkle chain integrity");
        }
    }

    Ok(())
}
