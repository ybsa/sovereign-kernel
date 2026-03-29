//! First-run setup wizard.

use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub async fn run() -> anyhow::Result<()> {
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".bright_cyan()
    );
    println!(
        "  ⚡ {}",
        "Sovereign Kernel — Advanced Setup Wizard"
            .bright_white()
            .bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════\n".bright_cyan()
    );

    let soul_path = PathBuf::from("soul/SOUL.md");

    println!("Welcome! Let's get your Sovereign Agent running with your preferred AI model.");

    // 1. Identity Setup
    let mut name_input = String::new();
    print!("What is your name? : ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut name_input)?;
    let name = name_input.trim().to_string();

    let mut goal_input = String::new();
    print!("What is your agent's primary goal? : ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut goal_input)?;
    let goal = goal_input.trim().to_string();

    // 2. Provider Selection
    println!("\n── Choose your AI Provider ────────────────────────────");
    println!(
        "  1. {} (Google) - Best for free/flash access",
        "Gemini".bright_green()
    );
    println!(
        "  2. {} (Anthropic) - Best for complex coding",
        "Claude".bright_yellow()
    );
    println!(
        "  3. {} (OpenAI) - The industry standard",
        "OpenAI".bright_blue()
    );
    println!(
        "  4. {} (Local) - Run on your own hardware",
        "Ollama".bright_magenta()
    );
    println!(
        "  5. {} - Define your own API endpoint",
        "Custom".bright_white()
    );

    let mut choice_input = String::new();
    print!("\nSelection (1-5) [1]: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut choice_input)?;
    let choice = choice_input.trim().parse::<u32>().unwrap_or(1);

    let (provider, model, env_var, base_url) = match choice {
        1 => {
            let mut key = String::new();
            print!("Enter your Gemini API Key: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut key)?;
            let key = key.trim();
            save_key_to_env("GEMINI_API_KEY", key)?;
            (
                "gemini".to_string(),
                "gemini-2.0-flash".to_string(),
                "GEMINI_API_KEY".to_string(),
                None,
            )
        }
        2 => {
            let mut key = String::new();
            print!("Enter your Anthropic API Key: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut key)?;
            let key = key.trim();
            save_key_to_env("ANTHROPIC_API_KEY", key)?;
            (
                "anthropic".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "ANTHROPIC_API_KEY".to_string(),
                None,
            )
        }
        3 => {
            let mut key = String::new();
            print!("Enter your OpenAI API Key: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut key)?;
            let key = key.trim();
            save_key_to_env("OPENAI_API_KEY", key)?;
            (
                "openai".to_string(),
                "gpt-4o".to_string(),
                "OPENAI_API_KEY".to_string(),
                None,
            )
        }
        4 => {
            let mut model_name_input = String::new();
            print!("Enter Ollama model name [llama3]: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut model_name_input)?;
            let model_name = model_name_input.trim();
            let model = if model_name.is_empty() {
                "llama3"
            } else {
                model_name
            };
            (
                "ollama".to_string(),
                model.to_string(),
                "OLLAMA_API_KEY".to_string(),
                Some("http://localhost:11434/v1".to_string()),
            )
        }
        _ => {
            let mut p_name_input = String::new();
            print!("Provider name: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut p_name_input)?;
            let mut m_name_input = String::new();
            print!("Model name: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut m_name_input)?;
            let mut url_input = String::new();
            print!("Base URL: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut url_input)?;
            let mut key_input = String::new();
            print!("API Key (optional): ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut key_input)?;

            let p_name = p_name_input.trim().to_string();
            let env_var = format!("{}_API_KEY", p_name.to_uppercase());
            if !key_input.trim().is_empty() {
                save_key_to_env(&env_var, key_input.trim())?;
            }

            (
                p_name,
                m_name_input.trim().to_string(),
                env_var,
                Some(url_input.trim().to_string()),
            )
        }
    };

    // 3. Create Identity
    if !soul_path.exists() {
        fs::create_dir_all("soul")?;
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
        println!("✓ Created {}", "soul/SOUL.md".green());
    }

    // 4. Update config.toml
    let config_path = PathBuf::from("config.toml");

    // Very simple TOML update logic for the wizard
    let new_config = format!(
        r#"# Sovereign Kernel Configuration
log_level = "info"
execution_mode = "sandbox"

[default_model]
provider = "{provider}"
model = "{model}"
api_key_env = "{env_var}"
{base_url}

[memory]
decay_rate = 0.1
embedding_model = "all-MiniLM-L6-v2"
"#,
        provider = provider,
        model = model,
        env_var = env_var,
        base_url = base_url
            .map(|u| format!("base_url = \"{}\"", u))
            .unwrap_or_default(),
    );

    fs::write(&config_path, new_config)?;
    println!("✓ Updated {}", "config.toml".green());

    println!("\n── Setup Complete! ────────────────────────────────────");
    println!(
        "  You are connected to: {} ({})",
        model.bright_cyan(),
        provider.bright_yellow()
    );
    println!("\nNext steps:");
    println!(
        "  1. Run {} to start as a background service.",
        "sovereign start".bold()
    );
    println!("  2. Run {} to begin chatting.", "sovereign chat".bold());
    println!("\nHappy hacking, {}!", name.bright_white());

    Ok(())
}

fn save_key_to_env(key_name: &str, key_value: &str) -> anyhow::Result<()> {
    if key_value.is_empty() {
        return Ok(());
    }

    let env_path = Path::new(".env");
    let mut lines = if env_path.exists() {
        fs::read_to_string(env_path)?
            .lines()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut found = false;
    for line in lines.iter_mut() {
        if line.starts_with(&format!("{}=", key_name)) {
            *line = format!("{}={}", key_name, key_value);
            found = true;
            break;
        }
    }

    if !found {
        lines.push(format!("{}={}", key_name, key_value));
    }

    fs::write(env_path, lines.join("\n") + "\n")?;
    Ok(())
}
