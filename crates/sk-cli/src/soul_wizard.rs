//! Persona (Soul) management and automated Agent spawning.

use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(clap::Subcommand, Debug)]
pub enum SoulCommands {
    /// List all available personas (Souls)
    List,
    /// Create a brand new persona
    Create,
    /// Launch a new Agent instance with a specific Soul and Task
    Spawn {
        /// Optional specific soul name
        soul: Option<String>,
        /// The task to perform
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Update an existing agent's persona on the fly
    ReSoul {
        /// Agent ID or Name
        id: String,
        /// New soul name
        soul: String,
    },
}

pub async fn run(command: SoulCommands) -> Result<()> {
    match command {
        SoulCommands::List => list_souls()?,
        SoulCommands::Create => {
            run_creation_wizard().await?;
        }
        SoulCommands::Spawn { soul, task } => {
            run_spawn_wizard(soul, task).await?;
        }
        SoulCommands::ReSoul { id, soul } => {
            re_soul_agent(&id, &soul).await?;
        }
    }
    Ok(())
}

fn list_souls() -> Result<()> {
    let soul_dir = PathBuf::from("soul");
    if !soul_dir.exists() {
        println!("No souls found in ./soul/");
        return Ok(());
    }

    println!("\n🎭 Available Agent Personas (Souls):");
    for entry in fs::read_dir(soul_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".md") {
            println!("  • {}", name.strip_suffix(".md").unwrap().bright_cyan());
        }
    }
    Ok(())
}

pub async fn run_creation_wizard() -> Result<()> {
    println!("\n✨ {}", "Persona Creation Wizard".bright_white().bold());

    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is this persona called? (e.g. Coder, Researcher)")
        .interact_text()?;

    let role: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is their primary role?")
        .interact_text()?;

    let traits: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Describe their personality traits (vibe, tone)")
        .interact_text()?;

    let boundaries: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Any strict boundaries or rules?")
        .default("Never delete files without asking. Be concise.".into())
        .interact_text()?;

    let soul_content = format!(
        r#"# SOUL.md - {name} Identity

## Core Truths
You are {name}. Your role is {role}.
Your character traits: {traits}.

## Boundaries
{boundaries}

## Continuity
This file is your core identity. Refer to it to stay in character.
"#
    );

    let soul_path =
        PathBuf::from("soul").join(format!("{}.md", name.to_lowercase().replace(' ', "_")));
    if !Path::new("soul").exists() {
        fs::create_dir_all("soul")?;
    }
    fs::write(&soul_path, soul_content)?;

    println!(
        "\n✅ Persona created: {}",
        soul_path.display().to_string().green()
    );

    let spawn_now = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to spawn an agent with this soul now?")
        .default(0)
        .items(&["Yes, deploy now", "No, just save for later"])
        .interact()?;

    if spawn_now == 0 {
        run_spawn_wizard(Some(name), None).await?;
    }

    Ok(())
}

pub async fn run_spawn_wizard(
    selected_soul: Option<String>,
    selected_task: Option<String>,
) -> Result<()> {
    let soul_name = match selected_soul {
        Some(s) => s,
        None => {
            let soul_dir = PathBuf::from("soul");
            let mut souls = vec![];
            for entry in fs::read_dir(soul_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") {
                    souls.push(name.strip_suffix(".md").unwrap().to_string());
                }
            }
            if souls.is_empty() {
                return Err(anyhow::anyhow!(
                    "No souls found. Please create one with 'sovereign soul create' first."
                ));
            }
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Which soul should this agent use?")
                .items(&souls)
                .interact()?;
            souls[selection].clone()
        }
    };

    let task = match selected_task {
        Some(t) => t,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("What task should this agent perform?")
            .interact_text()?,
    };

    println!(
        "\n🚀 Deploying Agent with Soul: {}...",
        soul_name.bright_cyan()
    );

    // Load the soul content
    let soul_path =
        PathBuf::from("soul").join(format!("{}.md", soul_name.to_lowercase().replace(' ', "_")));
    let soul_content = if soul_path.exists() {
        Some(fs::read_to_string(soul_path)?)
    } else {
        None
    };

    // Load active config (simplified lookup)
    let config = sk_types::config::KernelConfig::default();

    // In a real implementation, we'd use the same logic as main.rs to find the config.
    // For this wizard, we trigger the local agent loop directly.
    super::run::execute(config, &task, "auto", None, soul_content).await?;

    Ok(())
}

async fn re_soul_agent(id: &str, soul_name: &str) -> Result<()> {
    println!("Updating Agent {} to use Soul: {}...", id, soul_name);
    // This requires a running kernel. We'd hit the API: POST /v1/agents/{id}/soul
    println!("✅ Soul update signal sent (Mock). In a live session, the agent's identity has been shifted.");
    Ok(())
}
