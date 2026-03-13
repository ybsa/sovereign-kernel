//! Handler for the `sovereign status` command.
//!
//! Shows daemon status and lists registered agents from the memory substrate.

pub async fn print_status() -> anyhow::Result<()> {
    // 1. Check Daemon PID
    crate::daemon::status().await?;

    // 2. Try to read agents from the memory substrate
    let default_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sovereign-kernel")
        .join("sovereign.db");

    if !default_path.exists() {
        println!("\n🏘️  No village database found yet. Start the daemon first.");
        return Ok(());
    }

    let substrate =
        sk_memory::MemorySubstrate::open(&default_path, 0.1).map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("\n🏘️  Sovereign Village Status:");

    let agents = substrate.list_agents().map_err(|e| anyhow::anyhow!("{e}"))?;
    if agents.is_empty() {
        println!("  (No agents registered)");
    } else {
        println!(
            "  {:<36} {:<15} {:<12} {:<20}",
            "AGENT ID", "NAME", "STATE", "LAST ACTIVE"
        );
        println!(
            "  {:-<36} {:-<15} {:-<12} {:-<20}",
            "", "", "", ""
        );
        for agent in &agents {
            println!(
                "  {:<36} {:<15} {:<12} {:<20}",
                agent.id,
                agent.name,
                format!("{:?}", agent.state),
                agent.last_active.format("%Y-%m-%d %H:%M:%S")
            );
        }
        println!("\n  Total agents: {}", agents.len());
    }

    Ok(())
}
