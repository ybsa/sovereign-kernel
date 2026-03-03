//! Interactive chat REPL.

use sk_kernel::SovereignKernel;
use sk_types::config::KernelConfig;
use sk_types::AgentId;
use tracing::info;

/// Run the interactive chat REPL.
pub async fn run(config: KernelConfig) -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("  ⚡ Sovereign Kernel v{}", env!("CARGO_PKG_VERSION"));
    println!("  Type 'exit' or 'quit' to leave, 'clear' to reset");
    
    match config.execution_mode {
        sk_types::config::ExecutionMode::Sandbox => {
            println!("  [🛡️ Mode: SANDBOX - File and command safety enabled]");
        }
        sk_types::config::ExecutionMode::Unrestricted => {
            println!("  [🚨 Mode: UNRESTRICTED - WARNING: Agent has full system access!]");
        }
    }
    println!("═══════════════════════════════════════════════════════\n");

    // Clone browser config before config is moved into kernel
    let browser_config = config.browser.clone();

    // Initialize kernel
    let kernel = SovereignKernel::init(config).await?;

    // Create a default agent ID for this chat session
    let agent_id = AgentId::new();
    info!(agent = %agent_id, "Chat session started");

    // Initialize LLM Driver from environment
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let gemini_key = std::env::var("GEMINI_API_KEY").ok();
    let openai_key = std::env::var("OPENAI_API_KEY").ok();
    let github_token = std::env::var("GITHUB_TOKEN").ok();
    let groq_key = std::env::var("GROQ_API_KEY").ok();

    let mut driver: Option<Box<dyn sk_engine::llm_driver::LlmDriver>> = None;
    let mut model_name = String::new();

    if let Some(key) = anthropic_key {
        driver = Some(Box::new(
            sk_engine::drivers::anthropic::AnthropicDriver::new(
                key,
                "https://api.anthropic.com".to_string(),
            ),
        ));
        model_name = "claude-3-5-sonnet-20241022".to_string();
    } else if let Some(key) = openai_key {
        driver = Some(Box::new(sk_engine::drivers::openai::OpenAIDriver::new(
            key,
            "https://api.openai.com/v1".to_string(),
        )));
        model_name = "gpt-4o".to_string();
    } else if let Some(key) = github_token {
        driver = Some(Box::new(sk_engine::drivers::copilot::CopilotDriver::new(
            key,
            "".to_string(),
        )));
        model_name = "gpt-4o".to_string();
    } else if let Some(key) = groq_key {
        driver = Some(Box::new(sk_engine::drivers::openai::OpenAIDriver::new(
            key,
            "https://api.groq.com/openai/v1".to_string(),
        )));
        model_name = "llama3-70b-8192".to_string();
    } else if let Some(key) = gemini_key {
        driver = Some(Box::new(sk_engine::drivers::gemini::GeminiDriver::new(
            key,
            "https://generativelanguage.googleapis.com".to_string(),
        )));
        model_name = "gemini-2.0-flash-lite".to_string();
    }

    if driver.is_none() {
        println!("\nSovereign: [WARNING] No valid API key found in environment (tried ANTHROPIC, OPENAI, GITHUB_TOKEN, GROQ, GEMINI). Chat will not work.\n");
    } else {
        println!("\nSovereign: [Connected to {}]\n", model_name);
    }

    // Load existing session or create new one
    let mut session = if let Ok(entries) = kernel.memory.sessions.list_for_agent(agent_id.clone()) {
        if let Some((latest_id, _, _)) = entries.first() {
            if let Ok(Some(loaded_session)) = kernel.memory.sessions.load(*latest_id) {
                loaded_session
            } else {
                sk_types::Session::new(agent_id.clone())
            }
        } else {
            sk_types::Session::new(agent_id.clone())
        }
    } else {
        sk_types::Session::new(agent_id.clone())
    };

    let system_prompt = kernel.soul.to_system_prompt_fragment();

    // Setup BrowserManager (using Arc so it persists and is shared)
    use sk_engine::media::browser::BrowserManager;
    use sk_tools::skills::SkillRegistry;
    use std::sync::Arc;

    let browser_manager = Arc::new(BrowserManager::new(browser_config));

    // Initialize SafetyGate
    let safety_enabled = std::env::var("SOVEREIGN_UNSAFE")
        .map(|v| v != "1")
        .unwrap_or(true);
    let safety_gate = Arc::new(crate::safety::SafetyGate::new(safety_enabled));

    // Load OpenClaw skills
    let skills_path = std::env::current_dir()?
        .join("crates")
        .join("sk-tools")
        .join("skills");
    let skill_registry = Arc::new(SkillRegistry::load_from_dir(skills_path));
    info!(skills = skill_registry.list().len(), "Skills loaded");

    // Chat loop
    loop {
        // Read user input
        print!("You: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        
        if safety_gate.has_pending(&agent_id) {
            println!("  [⏱️ Waiting for approval... default deny in 60s]");
            let res = tokio::time::timeout(std::time::Duration::from_secs(60), async {
                tokio::task::spawn_blocking(|| {
                    let mut s = String::new();
                    let _ = std::io::stdin().read_line(&mut s);
                    s
                }).await.unwrap_or_default()
            }).await;

            match res {
                Ok(s) => input = s,
                Err(_) => {
                    println!("\n  [⏱️ Timeout reached! Auto-denying action.]\n");
                    safety_gate.deny_last_for_agent(&agent_id);
                    input = "deny".to_string(); // Feed deny back to the loop
                }
            }
        } else {
            std::io::stdin().read_line(&mut input)?;
        }

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input.to_lowercase().as_str() {
            "exit" | "quit" | "/exit" | "/quit" => {
                println!("\n⚡ Sovereign Kernel signing off. Until next time.");
                break;
            }
            "clear" | "/clear" => {
                session = sk_types::Session::new(agent_id.clone());
                println!("Session cleared.\n");
                continue;
            }
            "approve" | "yes" | "y" => {
                if safety_gate.approve_last_for_agent(&agent_id) {
                    println!("🛡️ Action approved. Re-submitting to Sovereign...");
                }
            }
            "deny" | "no" | "n" => {
                if safety_gate.deny_last_for_agent(&agent_id) {
                    println!("🛡️ Action denied. Blocked signature cleared.");
                }
            }
            _ => {}
        }

        if let Some(ref d) = driver {
            let kernel_ref = kernel.memory.clone();

            let config = crate::tool_executor::create_agent_config(
                d.as_ref(),
                system_prompt.clone(),
                model_name.clone(),
                Arc::new(kernel.config.clone()),
                kernel_ref,
                browser_manager.clone(),
                agent_id.clone(),
                skill_registry.clone(),
                safety_enabled,
                Some(safety_gate.clone()),
            );

            match sk_engine::agent_loop::run_agent_loop(config, &mut session, input).await {
                Ok(result) => {
                    println!("\nSovereign: {}\n", result.response);
                }
                Err(e) => {
                    println!("\nSovereign Error: {}\n", e);
                }
            }

            // Save after every turn
            if let Err(e) = kernel.memory.sessions.save(&session) {
                tracing::warn!("Failed to save chat session: {e}");
            }
        }
    }

    kernel.shutdown().await?;
    Ok(())
}
