//! First-run setup wizard.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub async fn run() -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("  ⚡ Sovereign Kernel — First Run Wizard");
    println!("═══════════════════════════════════════════════════════\n");

    let env_path = PathBuf::from(".env");
    let soul_dir = PathBuf::from("soul");
    let soul_path = soul_dir.join("SOUL.md");

    if !env_path.exists() || !soul_path.exists() {
        println!("We noticed you are starting fresh. Let's configure your Agent!");

        let mut name = String::new();
        print!("What is your name? : ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut name)?;
        let name = name.trim();

        let mut goal = String::new();
        print!("What is your agent's primary goal? : ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut goal)?;
        let goal = goal.trim();

        let mut gemini_key = String::new();
        print!("Enter your Gemini API Key (or press Enter to skip): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut gemini_key)?;
        let gemini_key = gemini_key.trim();

        let mut anthropic_key = String::new();
        print!("Enter your Anthropic API Key (or press Enter to skip): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut anthropic_key)?;
        let anthropic_key = anthropic_key.trim();

        // Channel setup — Telegram
        println!("\n── Channel Setup ──────────────────────────────────────");
        let mut telegram_token = String::new();
        print!("Enter your Telegram Bot Token (or press Enter to skip): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut telegram_token)?;
        let telegram_token = telegram_token.trim();

        if !env_path.exists()
            && (!gemini_key.is_empty() || !anthropic_key.is_empty() || !telegram_token.is_empty())
        {
            let mut env_content = String::new();
            if !gemini_key.is_empty() {
                env_content.push_str(&format!("GEMINI_API_KEY={}\n", gemini_key));
            }
            if !anthropic_key.is_empty() {
                env_content.push_str(&format!("ANTHROPIC_API_KEY={}\n", anthropic_key));
            }
            if !telegram_token.is_empty() {
                env_content.push_str(&format!("TELEGRAM_BOT_TOKEN={}\n", telegram_token));
            }
            fs::write(&env_path, env_content)?;
            println!("✓ Created .env file (excluded in .gitignore to protect privacy)");
        }

        if !soul_path.exists() {
            fs::create_dir_all(&soul_dir)?;
            let soul_content = format!(
                r#"# SOUL.md

[AGENT_NAME]: Sovereign Agent
[USER_NAME]: {name}

## Identity
You are the Sovereign Agent. Your user is {name}. You are concise, highly hyper-competent in Rust, and prioritize local-first execution. 

## Goals
1. {goal}
2. Protect the user's privacy by preferring local execution whenever possible.
3. Execute MCP tools flawlessly.

## Boundaries
- Never delete files without explicitly asking for confirmation first.
- Be extremely brief and direct in your answers. Do not use filler words.
"#
            );
            fs::write(&soul_path, soul_content)?;
            println!("✓ Created soul/SOUL.md custom identity template");
        }

        // Show available hands
        println!("\n── Available Autonomous Hands ─────────────────────────");
        let mut hand_registry = sk_hands::registry::HandRegistry::new();
        let loaded = hand_registry.load_bundled();
        if loaded > 0 {
            let defs = hand_registry.list_definitions();
            for def in &defs {
                println!("  {} {} — {}", def.icon, def.name, def.description);
            }
            println!("\n  Run `sovereign hands list` for details or `sovereign hands activate <name>` to enable.");
        } else {
            println!("  No bundled hands found.");
        }

        // Connection summary
        println!("\n── Setup Summary ──────────────────────────────────────");
        if !gemini_key.is_empty() {
            println!("  ✅ Gemini API configured");
        }
        if !anthropic_key.is_empty() {
            println!("  ✅ Anthropic API configured");
        }
        if !telegram_token.is_empty() {
            println!("  ✅ Telegram Bot configured");
        }
        if gemini_key.is_empty() && anthropic_key.is_empty() {
            println!("  ⚠️  No LLM API key set — chat will not work until you add one to .env");
        }
        println!();
    }

    // Existing config dir creation
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sovereign-kernel");
    fs::create_dir_all(&config_dir)?;
    println!("✓ Config directory: {}", config_dir.display());

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sovereign-kernel");
    fs::create_dir_all(&data_dir)?;
    println!("✓ Data directory: {}", data_dir.display());

    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        let default_config = format!(
            r#"# Sovereign Kernel Configuration
# ==================================

data_dir = "{}"
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"

[embedding]
provider = "openai"
model = "text-embedding-3-small"

memory_decay_rate = 0.1
context_window_tokens = 128000
"#,
            data_dir.display().to_string().replace('\\', "/")
        );
        fs::write(&config_path, default_config)?;
        println!("✓ Config file: {}", config_path.display());
    } else {
        println!("  Config file already exists: {}", config_path.display());
    }

    println!("\nSetup complete! Run `cargo run -p sk-cli -- start` and `cargo run -p sk-cli -- chat` to begin.");

    Ok(())
}
