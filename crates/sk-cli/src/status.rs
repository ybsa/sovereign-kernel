

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

    // 3. Try to get real-time status from the API
    if let Ok(api_status) = get_api_status().await {
        println!("  Kernel Version: {}", api_status.version);
        println!("  Primary Model:  {} ({})", api_status.model, api_status.driver);
        
        if !api_status.mcp_servers.is_empty() {
            println!("\n🏘️  Connected MCP Servers:");
            for server in api_status.mcp_servers {
                println!("  - {:<15} ({} tools)", server.name, server.tool_count);
            }
        }
    }

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

#[derive(Debug, serde::Deserialize)]
struct ApiStatus {
    version: String,
    model: String,
    driver: String,
    mcp_servers: Vec<ApiMcpStatus>,
}

#[derive(Debug, serde::Deserialize)]
struct ApiMcpStatus {
    name: String,
    tool_count: usize,
}

async fn get_api_status() -> anyhow::Result<ApiStatus> {
    // 1. Get SOVEREIGN_API_KEY for auth
    let api_key = std::env::var("SOVEREIGN_API_KEY").unwrap_or_default();
    
    // 2. Determine URL (defaulting to the standard 4242 port for API)
    // In a real scenario, this would come from config, but for status check
    // we try the default.
    let url = "http://127.0.0.1:4242/v1/status";
    
    // 3. Use curl to fetch the status (avoiding new dependency)
    let output = std::process::Command::new("curl")
        .arg("-s")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg(url)
        .output()?;
        
    if !output.status.success() {
        anyhow::bail!("Failed to fetch API status");
    }
    
    let status: ApiStatus = serde_json::from_slice(&output.stdout)?;
    Ok(status)
}
