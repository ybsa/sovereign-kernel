//! Interactive chat REPL.

use sk_kernel::SovereignKernel;
use sk_types::config::SovereignConfig;
use sk_types::AgentId;
use tracing::info;

/// Run the interactive chat REPL.
pub async fn run(config: SovereignConfig) -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("  ⚡ Sovereign Kernel v{}", env!("CARGO_PKG_VERSION"));
    println!("  Type 'exit' or 'quit' to leave, 'clear' to reset");
    println!("═══════════════════════════════════════════════════════");
    println!();

    // Initialize kernel
    let kernel = SovereignKernel::init(config).await?;

    // Create a default agent ID for this chat session
    let agent_id = AgentId::new();
    info!(agent = %agent_id, "Chat session started");

    // Initialize LLM Driver from environment
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let gemini_key = std::env::var("GEMINI_API_KEY").ok();
    
    let mut driver: Option<Box<dyn sk_engine::llm_driver::LlmDriver>> = None;
    let mut model_name = String::new();

    if let Some(key) = anthropic_key {
        driver = Some(Box::new(sk_engine::drivers::anthropic::AnthropicDriver::new(
            key,
            "https://api.anthropic.com".to_string()
        )));
        model_name = "claude-3-5-sonnet-20241022".to_string();
    } else if let Some(key) = gemini_key {
        driver = Some(Box::new(sk_engine::drivers::gemini::GeminiDriver::new(
            key, 
            "https://generativelanguage.googleapis.com".to_string()
        )));
        model_name = "gemini-2.5-flash".to_string();
    }

    if driver.is_none() {
        println!("\nSovereign: [WARNING] No ANTHROPIC_API_KEY or GEMINI_API_KEY found in environment. Chat will not work.\n");
    } else {
        println!("\nSovereign: [Connected to {}]\n", model_name);
    }

    let mut session = sk_types::Session::new(agent_id);
    let system_prompt = kernel.soul.to_system_prompt_fragment();

    // Chat loop
    loop {
        // Read user input
        print!("You: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
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
                session = sk_types::Session::new(agent_id);
                println!("Session cleared.\n");
                continue;
            }
            _ => {}
        }

        if let Some(ref d) = driver {
            let tools = vec![
                sk_tools::memory_tools::remember_tool(),
                sk_tools::memory_tools::recall_tool(),
                sk_tools::memory_tools::forget_tool(),
            ];

            let kernel_ref = std::sync::Arc::new(kernel.memory.clone());

            let config = sk_engine::agent_loop::AgentLoopConfig {
                driver: d.as_ref(),
                system_prompt: system_prompt.clone(),
                tools,
                model: model_name.clone(),
                max_tokens: 4096,
                temperature: 0.7,
                tool_executor: Box::new(move |tool_call| {
                    let k = kernel_ref.clone();
                    match tool_call.name.as_str() {
                        "remember" => {
                            if let Ok(args) = tool_call.parsed_arguments() {
                                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("chat");
                                sk_tools::memory_tools::handle_remember(&k, agent_id.clone(), content, source)
                            } else {
                                Err(sk_types::SovereignError::ToolExecutionError("Invalid arguments".into()))
                            }
                        }
                        "recall" => {
                            if let Ok(args) = tool_call.parsed_arguments() {
                                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5);
                                sk_tools::memory_tools::handle_recall(&k, agent_id.clone(), query, limit)
                            } else {
                                Err(sk_types::SovereignError::ToolExecutionError("Invalid arguments".into()))
                            }
                        }
                        "forget" => {
                             if let Ok(args) = tool_call.parsed_arguments() {
                                let memory_id = args.get("memory_id").and_then(|v| v.as_str()).unwrap_or("");
                                sk_tools::memory_tools::handle_forget(&k, memory_id)
                            } else {
                                Err(sk_types::SovereignError::ToolExecutionError("Invalid arguments".into()))
                            }
                        }
                        _ => Err(sk_types::SovereignError::ToolExecutionError(format!("Unknown tool: {}", tool_call.name)))
                    }
                }),
            };

            match sk_engine::agent_loop::run_agent_loop(config, &mut session, input).await {
                Ok(result) => {
                    println!("\nSovereign: {}\n", result.response);
                }
                Err(e) => {
                    println!("\nSovereign Error: {}\n", e);
                }
            }
        }
    }

    kernel.shutdown().await?;
    Ok(())
}
