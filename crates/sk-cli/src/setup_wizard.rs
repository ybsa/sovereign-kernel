//! Ultra-smooth, non-developer friendly setup wizard.

use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Password, Select};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut};

pub async fn run() -> Result<()> {
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".bright_cyan()
    );
    println!(
        "  ⚡ {}",
        "Sovereign Kernel — Easy Setup Wizard".bright_white().bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════\n".bright_cyan()
    );

    println!("Welcome! This guide will help you set up your own AI agent in seconds.");
    println!("No coding required.\n");

    // 1. LLM Provider Selection
    let providers = vec![
        "NVIDIA NIM (Fastest, High Quality)",
        "Anthropic Claude (Best for Coding)",
        "OpenAI GPT-4 (The Standard)",
        "Google Gemini (Large context)",
        "Local Ollama (Private, Runs on your hardware)",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an AI provider")
        .default(0)
        .items(&providers)
        .interact()?;

    let (provider_id, default_model, env_var, base_url) = match selection {
        0 => (
            "nvidia",
            "meta/llama-3.1-405b-instruct",
            "NVIDIA_API_KEY",
            Some("https://integrate.api.nvidia.com/v1"),
        ),
        1 => (
            "anthropic",
            "claude-3-5-sonnet-20241022",
            "ANTHROPIC_API_KEY",
            None,
        ),
        2 => ("openai", "gpt-4o", "OPENAI_API_KEY", None),
        3 => ("gemini", "gemini-2.0-flash", "GEMINI_API_KEY", None),
        _ => (
            "ollama",
            "llama3.1",
            "OLLAMA_API_KEY",
            Some("http://localhost:11434/v1"),
        ),
    };

    // 2. API Key Entry
    if provider_id != "ollama" {
        println!(
            "\n🔑 Please paste your {} API Key.",
            provider_id.to_uppercase()
        );
        println!("(It won't be displayed on the screen for your privacy)");

        let raw_key: String = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("API Key")
            .interact()?;

        save_to_env(env_var, &clean_api_key(&raw_key))?;
    }

    // 3. Update config.toml
    update_config(provider_id, default_model, env_var, base_url)?;

    println!("\n✅ {}", "LLM configuration complete!".green());

    // 4. Initial Soul Setup
    let create_soul = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to create your first agent persona (Soul) now?")
        .default(0)
        .items(&["Yes, let's build a persona", "No, use defaults for now"])
        .interact()?;

    if create_soul == 0 {
        super::soul_wizard::run_creation_wizard().await?;
    }

    println!("\n── Setup Complete! ────────────────────────────────────");
    println!("Next steps:");
    println!("  1. Run {} to start chatting.", "sovereign chat".bold());
    println!(
        "  2. Run {} to see all commands.",
        "sovereign --help".bold()
    );
    println!("\nHappy hacking!");

    Ok(())
}

fn clean_api_key(raw: &str) -> String {
    let mut cleaned = raw.trim().to_string();

    // Remove all whitespace
    cleaned = cleaned.chars().filter(|c| !c.is_whitespace()).collect();

    // Handle nested quotes or repetitions
    loop {
        let prev = cleaned.clone();

        // Remove common header keys
        if cleaned.to_lowercase().starts_with("authorization:") {
            cleaned = cleaned[14..].to_string();
        }
        if cleaned.to_lowercase().starts_with("bearer") {
            cleaned = cleaned[6..].to_string();
        }

        // Remove common JSON structures if user pasted a JSON snippet
        cleaned = cleaned
            .trim_matches('{')
            .trim_matches('}')
            .trim_matches('"')
            .trim_matches('\'')
            .trim_matches(':')
            .trim_matches(',')
            .to_string();

        if cleaned == prev {
            break;
        }
    }

    cleaned
}

fn save_to_env(name: &str, value: &str) -> Result<()> {
    let env_path = PathBuf::from(".env");
    let content = if env_path.exists() {
        fs::read_to_string(&env_path)?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found = false;
    let target_prefix = format!("{}=", name);

    for line in lines.iter_mut() {
        if line.starts_with(&target_prefix) {
            *line = format!("{}={}", name, value);
            found = true;
            break;
        }
    }

    if !found {
        lines.push(format!("{}={}", name, value));
    }

    fs::write(env_path, lines.join("\n") + "\n")?;
    Ok(())
}

fn update_config(provider: &str, model: &str, env_var: &str, base_url: Option<&str>) -> Result<()> {
    let config_path = PathBuf::from("config.toml");
    let content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let mut doc = content.parse::<DocumentMut>()?;

    // We want to ensure there is at least one [[llm]] block,
    // or update the existing one if it's the primary.
    if let Some(llm_array) = doc.get_mut("llm").and_then(|v| v.as_array_of_tables_mut()) {
        if !llm_array.is_empty() {
            let table = llm_array.get_mut(0).unwrap();
            table.insert("provider", value(provider));
            table.insert("model", value(model));
            table.insert("api_key_env", value(env_var));
            if let Some(url) = base_url {
                table.insert("base_url", value(url));
            } else {
                table.remove("base_url");
            }
        } else {
            let mut table = toml_edit::Table::new();
            table.insert("provider", value(provider));
            table.insert("model", value(model));
            table.insert("api_key_env", value(env_var));
            if let Some(url) = base_url {
                table.insert("base_url", value(url));
            }
            llm_array.push(table);
        }
    } else {
        // Create the llm array
        let mut llm_array = toml_edit::ArrayOfTables::new();
        let mut table = toml_edit::Table::new();
        table.insert("provider", value(provider));
        table.insert("model", value(model));
        table.insert("api_key_env", value(env_var));
        if let Some(url) = base_url {
            table.insert("base_url", value(url));
        }
        llm_array.push(table);
        doc.insert("llm", toml_edit::Item::ArrayOfTables(llm_array));
    }

    fs::write(config_path, doc.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_api_key() {
        assert_eq!(clean_api_key("  nvapi-123  "), "nvapi-123");
        assert_eq!(clean_api_key("Bearer nvapi-123"), "nvapi-123");
        assert_eq!(
            clean_api_key("Authorization: Bearer nvapi-123"),
            "nvapi-123"
        );
        assert_eq!(clean_api_key("\"nvapi-123\""), "nvapi-123");
        assert_eq!(clean_api_key("'nvapi-123'"), "nvapi-123");
    }
}
