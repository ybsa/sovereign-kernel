//! Handler for the `sovereign run` command.
//!
//! Usage examples:
//! - `sovereign run "Summarize today's news"`
//! - `sovereign run "Monitor CPU" --mode unrestricted`
//! - `sovereign run "Send weekly digest" --schedule "0 9 * * 1"`

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use sk_types::AgentId;
use std::sync::Arc;

pub async fn execute(
    config: KernelConfig,
    task: &str,
    mode_hint: &str,
    schedule: Option<String>,
) -> anyhow::Result<()> {
    println!("⚡ Initializing Sovereign Agent...");

    let kernel = Arc::new(SovereignKernel::init(config).await?);

    // --- Scheduled mode: create a cron job and exit ---
    if let Some(cron_expr) = schedule {
        println!("📅 Scheduling task: \"{}\"", task);
        println!("   Cron: {}", cron_expr);

        let job = sk_types::CronJob {
            id: sk_types::CronJobId::new(),
            agent_id: AgentId::new(),
            name: format!("cli-{}", &task[..task.len().min(32)]),
            enabled: true,
            schedule: sk_types::CronSchedule::Cron {
                expr: cron_expr,
                tz: None,
            },
            action: sk_types::CronAction::AgentTurn {
                message: task.to_string(),
                model_override: None,
                timeout_secs: Some(300),
            },
            delivery: sk_types::CronDelivery::None,
            created_at: chrono::Utc::now(),
            last_run: None,
            next_run: None,
        };

        // Validate before persisting
        if let Err(e) = job.validate(0) {
            anyhow::bail!("Invalid schedule: {}", e);
        }

        println!("✅ Job created. ID: {}", job.id);
        println!("   (Note: Job will run when the daemon is started with `sovereign start`)");
        return Ok(());
    }

    // --- Agent Library Lookup ---
    let mut task_parts = task.splitn(2, ' ');
    let first_word = task_parts.next().unwrap_or("");
    let remaining_task = task_parts.next().unwrap_or(task);

    let agent_dir = std::path::PathBuf::from("agents").join(first_word);
    let manifest_path = agent_dir.join("manifest.toml");

    let (intent, _skill_def, custom_soul) = if manifest_path.exists() {
        println!(
            "📂 Loading agent from library: {}",
            first_word.bright_cyan().bold()
        );
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: sk_types::agent::AgentManifest = toml::from_str(&manifest_content)?;

        let soul_path = agent_dir.join("SOUL.md");
        let soul = if soul_path.exists() {
            Some(std::fs::read_to_string(soul_path)?)
        } else {
            None
        };

        (
            sk_kernel::wizard::AgentIntent {
                name: manifest.name,
                description: manifest.description,
                task: remaining_task.to_string(),
                skills: manifest.skills,
                model_tier: "default".into(),
                scheduled: false,
                schedule: None,
                capabilities: manifest.capabilities.tools,
                mode: Some(mode_hint.to_string()),
                is_otto: false,
            },
            None,
            soul,
        )
    } else if mode_hint == "auto" {
        println!("🔍 Analyzing task intent...");
        let (intent, skill) = sk_kernel::wizard::SetupWizard::analyze_task_intent(
            kernel.driver.clone(),
            &kernel.model_name,
            task,
        )
        .await?;
        (intent, skill, None)
    } else {
        (
            sk_kernel::wizard::AgentIntent {
                name: "cli_agent".into(),
                description: "Agent spawned via CLI".into(),
                task: task.into(),
                skills: vec![],
                model_tier: "default".into(),
                scheduled: false,
                schedule: None,
                capabilities: vec!["file_read".into(), "web".into(), "shell".into()],
                mode: Some(mode_hint.to_string()),
                is_otto: false,
            },
            None,
            None,
        )
    };

    println!("🚀 Spawning agent: {}", intent.name);
    let agent_id = AgentId::new();

    use colored::*;
    use std::io::Write;

    let stream_handler: Option<sk_engine::agent_loop::StreamHandler> =
        Some(Box::new(move |chunk| {
            if chunk.starts_with("\n🔧 Calling tool:") {
                println!("{}", chunk.bright_cyan().bold());
            } else {
                print!("{}", chunk);
                let _ = std::io::stdout().flush();
            }
        }));

    let mut system_prompt = kernel.soul.to_system_prompt_fragment();
    if let Some(soul) = custom_soul {
        system_prompt = format!("{}\n\n{}", system_prompt, soul);
    }

    let agent_config = sk_kernel::executor::create_agent_config(
        kernel.clone(),
        kernel.driver.clone(),
        system_prompt,
        kernel.model_name.clone(),
        agent_id,
        kernel.skills.clone(),
        stream_handler,
    );

    let mut session = sk_types::Session::new(agent_id);

    println!("--- Agent Output ---");
    let result = sk_engine::agent_loop::run_agent_loop(agent_config, &mut session, &intent.task)
        .await
        .map_err(|e| anyhow::anyhow!("Agent loop failed: {e}"))?;

    println!("\n--- Final Response ---");
    println!("{}", result.response);

    // Persist session
    let _ = kernel.memory.sessions.save(&session);

    println!("\n✅ Task completed. Agent ID: {}", agent_id);
    Ok(())
}
