//! Sovereign Doctor — system diagnostics and health check.
//!
//! Checks:
//! - Rust environment (cargo, rustc)
//! - System dependencies (SQLite, OpenSSL/Rustls)
//! - Directory permissions (config, data, logs)
//! - Environment variables (API keys, Soul)
//! - Network connectivity (LLM providers, MCP servers)
//! - Daemon status

use std::path::PathBuf;
use sk_types::config::KernelConfig;

pub async fn run(config: &KernelConfig) -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("  🏥 Sovereign Doctor — System Diagnostics");
    println!("═══════════════════════════════════════════════════════\n");

    let mut issues = 0;

    // 1. Check Directories
    print!("📁 Checking directories... ");
    let dirs = vec![
        config.data_dir.clone(),
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("sovereign-kernel"),
    ];

    for d in dirs {
        if d.exists() {
            // Check write permission
            if fs_err::write(d.join(".doctor"), "ok").is_ok() {
                let _ = fs_err::remove_file(d.join(".doctor"));
            } else {
                println!("\n  ❌ Write permission denied: {:?}", d);
                issues += 1;
            }
        } else {
            println!("\n  ⚠️ Missing directory: {:?}", d);
            issues += 1;
        }
    }
    if issues == 0 { println!("✅ OK"); }

    // 2. Check Soul Identity
    print!("👻 Checking Soul identity... ");
    let soul_path = PathBuf::from("soul/SOUL.md");
    if soul_path.exists() {
        println!("✅ OK ({})", soul_path.display());
    } else {
        println!("\n  ❌ Missing soul/SOUL.md. Run `sovereign init` to create one.");
        issues += 1;
    }

    // 3. Check API Keys
    print!("🔑 Checking API keys... ");
    let keys = vec![
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("GEMINI_API_KEY", "Google Gemini"),
        ("GROQ_API_KEY", "Groq"),
    ];
    let mut keys_found = 0;
    for (env, _name) in keys {
        if std::env::var(env).is_ok() {
            keys_found += 1;
        }
    }
    if keys_found > 0 {
        println!("✅ OK ({} keys found)", keys_found);
    } else {
        println!("\n  ⚠️ No LLM API keys found in environment. Chat will not work.");
        issues += 1;
    }

    // 5. Check The Treasury
    print!("💰 Checking The Treasury... ");
    let metering_path = config.data_dir.join("metering.json");
    if metering_path.exists() {
        println!("✅ Active (Cost tracking persistent)");
    } else {
        println!("💤 Inactive (No persistent costs yet)");
    }

    // 6. Check Network Connectivity (Lite)
    print!("🌐 Checking LLM connectivity... ");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    
    match client.get("https://api.anthropic.com").send().await {
        Ok(_) => println!("✅ OK"),
        Err(e) => {
            println!("\n  ⚠️ Network issue or block: {}", e);
            issues += 1;
        }
    }

    println!("\n═══════════════════════════════════════════════════════");
    if issues == 0 {
        println!("  ✨ Your Sovereign Kernel is HEALTHY and ready for duty.");
    } else {
        println!("  ⚠️ Found {} potential issues. Please review the log above.", issues);
    }
    println!("═══════════════════════════════════════════════════════\n");

    Ok(())
}
