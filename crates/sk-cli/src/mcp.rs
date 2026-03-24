use sk_types::KernelConfig;
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut};

pub async fn run(config: KernelConfig, action: &str, args: &[String]) -> anyhow::Result<()> {
    match action {
        "list" => list_servers(&config),
        "add" => {
            if args.len() < 2 {
                println!("Usage: sovereign mcp add <name> <command> [args...]");
                println!(
                    "Example: sovereign mcp add sqlite npx -y @modelcontextprotocol/server-sqlite"
                );
                return Ok(());
            }
            let name = &args[0];
            let command = &args[1];
            let cmd_args = &args[2..];
            add_server(&config, name, command, cmd_args).await?;
        }
        "remove" => {
            if args.is_empty() {
                println!("Usage: sovereign mcp remove <name>");
                return Ok(());
            }
            remove_server(&config, &args[0]).await?;
        }
        _ => {
            println!("Unknown action: {}", action);
            println!("Available actions: list, add, remove");
        }
    }
    Ok(())
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sovereign-kernel")
        .join("config.toml")
}

fn list_servers(config: &KernelConfig) {
    println!("🔌 Connected MCP Servers:");
    if config.mcp_servers.is_empty() {
        println!("  (None configured)");
        return;
    }
    for server in &config.mcp_servers {
        print!("  - {} [Transport: ", server.name);
        match &server.transport {
            sk_types::config::McpTransportEntry::Stdio { command, args } => {
                let args_str = args.join(" ");
                println!("stdio -> {} {}]", command, args_str);
            }
            sk_types::config::McpTransportEntry::Sse { url } => {
                println!("sse -> {}]", url);
            }
        }
    }
}

async fn notify_daemon(config: &KernelConfig) -> anyhow::Result<()> {
    let api_url = format!("http://{}/v1/config", config.api_listen);
    let client = reqwest::Client::new();

    // Read the latest config from disk instead of sending the old one
    let config_path = get_config_path();
    if !config_path.exists() {
        return Ok(()); // Nothing to do
    }

    let new_config = sk_types::KernelConfig::load(&config_path)?;

    let mut req = client.post(&api_url).json(&new_config);
    if let Ok(key) = std::env::var("SOVEREIGN_API_KEY") {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }

    let res = req.send().await;
    match res {
        Ok(r) if r.status().is_success() => {
            println!("🔄 Daemon automatically reloaded the new configuration.");
        }
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            println!("⚠️  Failed to notify daemon: {} - {}", status, text);
        }
        Err(_) => {
            println!(
                "ℹ️  Daemon is not currently running. Changes will take effect on next start."
            );
        }
    }
    Ok(())
}

async fn add_server(
    config: &KernelConfig,
    name: &str,
    command: &str,
    args: &[String],
) -> anyhow::Result<()> {
    let config_path = get_config_path();
    if !config_path.exists() {
        anyhow::bail!("Config file not found at {}", config_path.display());
    }

    let toml_content = fs::read_to_string(&config_path)?;
    let mut doc = toml_content.parse::<DocumentMut>()?;

    // Check if server already exists
    if let Some(servers) = doc.get("mcp_servers").and_then(|i| i.as_array_of_tables()) {
        for server in servers.iter() {
            if server.get("name").and_then(|n| n.as_str()) == Some(name) {
                println!("⚠️  MCP server '{}' is already configured.", name);
                return Ok(());
            }
        }
    }

    // Prepare table
    let mut server_table = toml_edit::Table::new();
    server_table.insert("name", value(name));

    let mut transport = toml_edit::Table::new();
    transport.insert("type", value("stdio"));
    transport.insert("command", value(command));

    let mut args_arr = toml_edit::Array::new();
    for a in args {
        args_arr.push(a);
    }
    transport.insert("args", value(args_arr));

    server_table.insert("transport", toml_edit::Item::Table(transport));

    let mut env = toml_edit::Table::new();
    env.insert("type", value("map"));
    let map_table = toml_edit::Table::new();
    env.insert("map", toml_edit::Item::Table(map_table));
    server_table.insert("env", toml_edit::Item::Table(env));

    // Append to array of tables
    if let Some(arr) = doc
        .get_mut("mcp_servers")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        arr.push(server_table);
    } else {
        let mut arr = toml_edit::ArrayOfTables::new();
        arr.push(server_table);
        doc.insert("mcp_servers", toml_edit::Item::ArrayOfTables(arr));
    }

    fs::write(&config_path, doc.to_string())?;
    println!("✅ Added MCP server '{}'.", name);

    notify_daemon(config).await?;

    Ok(())
}

async fn remove_server(config: &KernelConfig, name: &str) -> anyhow::Result<()> {
    let config_path = get_config_path();
    if !config_path.exists() {
        anyhow::bail!("Config file not found at {}", config_path.display());
    }

    let toml_content = fs::read_to_string(&config_path)?;
    let mut doc = toml_content.parse::<DocumentMut>()?;

    let mut removed = false;
    if let Some(servers) = doc
        .get_mut("mcp_servers")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        let mut idx_to_remove = None;
        for (i, server) in servers.iter().enumerate() {
            if server.get("name").and_then(|n| n.as_str()) == Some(name) {
                idx_to_remove = Some(i);
                break;
            }
        }
        if let Some(idx) = idx_to_remove {
            servers.remove(idx);
            removed = true;
        }
    }

    if removed {
        // If the array is empty, remove the key entirely to keep it clean
        let is_empty = doc
            .get("mcp_servers")
            .and_then(|i| i.as_array_of_tables())
            .map(|a| a.is_empty())
            .unwrap_or(false);
        if is_empty {
            doc.remove("mcp_servers");
        }

        fs::write(&config_path, doc.to_string())?;
        println!("🗑️ Removed MCP server '{}'.", name);

        notify_daemon(config).await?;
    } else {
        println!("⚠️  MCP server '{}' not found in configuration.", name);
    }

    Ok(())
}
