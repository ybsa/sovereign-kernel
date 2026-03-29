use sk_types::{AgentId, SovereignError};
use sk_engine::agent_loop::AgentLoopConfig;
use std::sync::Arc;
use tracing::info;

use crate::SovereignKernel;

/// Detect if a model name suggests a "small" local model (under ~10B parameters).
fn is_small_model(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    // Detect by parameter count markers
    let size_markers = ["1b", "2b", "3b", "4b", "7b", "8b", "3.2", "3.1:8b"];
    for marker in &size_markers {
        if lower.contains(marker) {
            return true;
        }
    }
    // Detect by common small model families used locally
    let small_families = ["llama3.2", "phi", "tinyllama", "gemma:2b", "gemma2:2b", "qwen2:0.5b", "qwen2:1.5b"];
    for family in &small_families {
        if lower.contains(family) {
            return true;
        }
    }
    false
}

/// Creates a standardized AgentLoopConfig with all default tools registered.
pub fn create_agent_config(
    kernel: Arc<SovereignKernel>,
    driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    system_prompt: String,
    model_name: String,
    agent_id: AgentId,
    browser_manager: Arc<sk_engine::runtime::browser::BrowserManager>,
    _skill_registry: Arc<std::sync::RwLock<sk_tools::skills::SkillRegistry>>,
) -> AgentLoopConfig {
    let small_model = is_small_model(&model_name);
    if small_model {
        info!(model = %model_name, "Detected small/local model — using reduced tool set (8 core tools)");
    }

    // --- CORE TOOLS (always available) ---
    let mut tools = vec![
        sk_tools::memory_tools::remember_tool(),
        sk_tools::memory_tools::recall_tool(),
        sk_tools::file_ops::read_file_tool(),
        sk_tools::file_ops::write_file_tool(),
        sk_tools::file_ops::list_dir_tool(),
        sk_tools::shell::shell_exec_tool(),
        sk_tools::web_search::web_search_tool(),
        sk_tools::web_fetch::web_fetch_tool(),
    ];

    // --- EXTENDED TOOLS (only for large models) ---
    if !small_model {
        tools.push(sk_tools::memory_tools::forget_tool());
        tools.push(sk_tools::file_ops::delete_file_tool());
        tools.push(sk_tools::file_ops::move_file_tool());
        tools.push(sk_tools::file_ops::copy_file_tool());
        tools.push(sk_tools::code_exec::code_exec_tool());
        tools.push(sk_tools::shared_memory::shared_memory_store_tool());
        tools.push(sk_tools::shared_memory::shared_memory_recall_tool());
        tools.push(sk_tools::scheduler::schedule_create_tool());
        tools.push(sk_tools::scheduler::schedule_list_tool());
        tools.push(sk_tools::scheduler::schedule_delete_tool());
        tools.push(sk_tools::voice_tools::text_to_speech_tool());
        tools.push(sk_tools::voice_tools::speech_to_text_tool());
        tools.extend(sk_tools::browser_tools::browser_tools());
        tools.extend(sk_tools::host::host_tools());
        tools.push(sk_tools::skills::get_skill_tool());
        tools.push(sk_tools::skills::list_skills_tool());
        tools.push(sk_tools::ottos_outpost::ottos_outpost_tool());

        // Pull in dynamic tools from the MCP registry
        if let Ok(mcp_lock) = kernel.mcp.try_read() {
            tools.extend(mcp_lock.all_tools());
        } else {
            tracing::warn!("Failed to acquire MCP read lock, MCP tools will be missing.");
        }

        // Agent-to-Agent message tool
        tools.push(sk_types::ToolDefinition {
            name: "agent_message".into(),
            description: "Send a direct message to another active agent on the system. Use this to coordinate or delegate tasks.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "to_agent_id": {
                        "type": "string",
                        "description": "The unique Agent ID of the recipient."
                    },
                    "message": {
                        "type": "string",
                        "description": "The contents of the message to send."
                    }
                },
                "required": ["to_agent_id", "message"]
            }),
        });

        tools.push(sk_types::ToolDefinition {
            name: "summon_skeleton".into(),
            description: "The Witch (The Summoner) uses her magic to dynamically spawn a background skeleton worker. It will run in Sandbox mode by default. You can continue working while it runs. Use check_skeleton to see its status.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "skeleton_name": { "type": "string", "description": "Name of the skeleton (e.g. 'researcher')" },
                    "task_description": { "type": "string", "description": "What the skeleton should do (summoned by the Witch)." },
                    "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Capabilities the skeleton needs (e.g. 'web', 'file_read', 'browser')" },
                    "mode_hint": { "type": "string", "enum": ["safe", "unrestricted", "scheduled"], "description": "Execution mode hint." }
                },
                "required": ["skeleton_name", "task_description", "capabilities"]
            }),
        });

        tools.push(sk_types::ToolDefinition {
            name: "builder".into(),
            description: "The Builder (The Architect) transforms a natural language task into a permanent Village member (Hand). The Architect forges the permanent infrastructure, while the Witch summons the workers.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "task": { "type": "string", "description": "The permanent Hand to forge (e.g. 'Create a WhatsApp specialist')" }
                },
                "required": ["task"]
            }),
        });

        tools.push(sk_types::ToolDefinition {
            name: "check_skeleton".into(),
            description: "Check the status of a worker summoned by the Witch.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "skeleton_id": { "type": "string", "description": "The Agent ID of the skeleton" }
                },
                "required": ["skeleton_id"]
            }),
        });
    }

    let b = browser_manager;
    let aid = agent_id;
    let k = kernel.clone();

    let cfg_snap = kernel.config.read().unwrap();
    let max_iter = cfg_snap.max_iterations_per_task;
    let max_tok = cfg_snap.max_tokens_per_task;
    let step_dump = cfg_snap.step_dump_enabled;
    let forensics = cfg_snap.effective_workspaces_dir();
    drop(cfg_snap);

    // --- OS-Aware System Prompt ---
    let mut system_prompt = system_prompt;
    let os_name = std::env::consts::OS;
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    
    system_prompt.push_str(&format!(
        "\n\n## System Context\n- You are running on **{}**.\n- Use {} file paths (e.g. `{}`).\n- Your available tools are EXACTLY: {}\n- ONLY call tools from this list. Do NOT invent tool names.\n",
        os_name,
        if os_name == "windows" { "Windows" } else { "Unix" },
        if os_name == "windows" { "C:\\Users\\..." } else { "/home/..." },
        tool_names.join(", ")
    ));

    system_prompt.push_str("\n## Tool Use Instructions\n- When using a tool, provide ONLY the raw JSON arguments as defined in the schema.\n- DO NOT include types, descriptions, or the schema itself in the values.\n- Example: Use `{\"content\": \"hello\"}`, NOT `{\"content\": {\"type\": \"string\", \"content\": \"hello\"}}`.\n- If the user asks a simple question like 'hi', just respond with text — do NOT call any tools.\n");

    AgentLoopConfig {
        driver,
        system_prompt,
        tools,
        model: model_name.clone(),
        max_tokens: 4096,
        temperature: 0.7,
        max_iterations_per_task: max_iter,
        max_tokens_per_task: max_tok,
        step_dump_enabled: step_dump,
        forensics_root: forensics,
        stream_handler: None,
        on_usage: {
            let kernel = kernel.clone();
            let aid = agent_id;
            let model = model_name.clone();
            Some(Box::new(move |usage| {
                let cost = crate::metering::MeteringEngine::estimate_cost(
                    &model,
                    usage.prompt_tokens as u64,
                    usage.completion_tokens as u64,
                );
                kernel.metering.record_cost(aid, cost);

                // Enforce global budget
                kernel
                    .config
                    .read()
                    .unwrap()
                    .check_budget(kernel.metering.total_cost())
                    .map_err(|e| SovereignError::Internal(e.to_string()))?;

                Ok(())
            }))
        },
        checkpoint_handler: None,
        tool_executor: Box::new(move |tool_call| {
            let tool_id = tool_call.id.clone();
            let kernel = k.clone();
            let browser = b.clone();
            let aid = aid;
            
            // --- Resilient Parser (Schema Stripper) ---
            let mut sanitized_input = tool_call.input.clone();
            if let Some(obj) = sanitized_input.as_object_mut() {
                for (_key, value) in obj.iter_mut() {
                    // Check if value is an object that looks like a schema-hallucination
                    // e.g. {"type": "string", "content": "actual_val"} 
                    // or {"type": "string", "description": "...", "value": "actual_val"}
                    if let Some(val_obj) = value.as_object() {
                         if val_obj.contains_key("type") && (val_obj.contains_key("content") || val_obj.contains_key("value")) {
                             if let Some(actual) = val_obj.get("content").or_else(|| val_obj.get("value")) {
                                 tracing::debug!("Schema stripper: recovered actual value from hallucinated schema object");
                                 *value = actual.clone();
                             }
                         }
                    }
                }
            }
            // ------------------------------------------

            let agent_id_str = aid.to_string();
            let config_snap = kernel.config.read().unwrap();
            let default_mode = config_snap.execution_mode;
            let approval_list = config_snap.approval_whitelist.clone();
            let workspaces_dir = config_snap.effective_workspaces_dir();
            let exec_policy = config_snap.exec_policy.clone();
            let docker_cfg = config_snap.docker.clone();
            drop(config_snap);
            let force_sandbox = kernel
                .memory
                .structured
                .get(aid, "forced_sandbox")
                .unwrap_or(None)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mode = if force_sandbox {
                sk_types::config::ExecutionMode::Sandbox
            } else {
                default_mode
            };

            let _mode_str = match mode {
                sk_types::config::ExecutionMode::Sandbox => "Sandbox",
                sk_types::config::ExecutionMode::Unrestricted => "Unrestricted",
            };

            // 0. Enforce ExecutionMode::Sandbox Restrictions
            if mode == sk_types::config::ExecutionMode::Sandbox {
                let risk = crate::approval::ApprovalManager::classify_risk(
                    &tool_call.name,
                    Some(&sanitized_input),
                );
                let is_host_read =
                    tool_call.name == "host_read_file" || tool_call.name == "host_list_dir";
                let is_host_mutate = tool_call.name.starts_with("host_") && !is_host_read;
                let is_raw_shell = tool_call.name == "shell_exec"
                    && !tool_call
                        .input
                        .get("use_sandbox")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                let is_dangerous = risk == sk_types::approval::RiskLevel::Critical
                    || risk == sk_types::approval::RiskLevel::High;

                if is_host_mutate || is_raw_shell || is_dangerous {
                    return Err(sk_types::SovereignError::CapabilityDenied(format!(
                        "Tool '{}' is blocked by Sandbox mode. Switch to Unrestricted mode.",
                        tool_call.name
                    )));
                }
            }

            // 1. Check Whitelist
            let is_whitelisted = approval_list.iter().any(|w| {
                tool_call.name == *w
                    || (tool_call.name == "shell_exec"
                        && tool_call
                            .input
                            .get("command")
                            .and_then(|v| v.as_str())
                            .map(|s| s.contains(w))
                            .unwrap_or(false))
            });

            // 2. Enforce conversational SafetyGate for non-whitelisted dangerous actions
            if !is_whitelisted {
                if let Err(detail) =
                    kernel
                        .safety
                        .check(&tool_call.name, &sanitized_input, Some(&aid))
                {
                    return Err(sk_types::SovereignError::ToolExecutionError(detail));
                }
            }

            // Execute the tool and capture the structured result
            let mut result = sk_types::ToolResult {
                tool_use_id: tool_id.clone(),
                content: String::new(),
                is_error: false,
                signal: None,
            };

            match tool_call.name.as_str() {
                "village_forge" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let task_desc = args
                            .get("task_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        Ok(sk_tools::village_forge::handle_village_forge(
                            &tool_id, task_desc,
                        ))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "agent_message" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let to_agent_id_str = args
                            .get("to_agent_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        if let Ok(to_id) = std::str::FromStr::from_str(to_agent_id_str) {
                            match kernel.bus.send(Some(&aid), &to_id, message.to_string()) {
                                Ok(_) => Ok(healer_result(
                                    &tool_id,
                                    format!("Message successfully sent to agent {}", to_id),
                                    false,
                                )),
                                Err(e) => Err(sk_types::SovereignError::ToolExecutionError(e)),
                            }
                        } else {
                            Err(sk_types::SovereignError::ToolExecutionError(
                                "Invalid to_agent_id format".into(),
                            ))
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "summon_skeleton" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let skeleton_name = args
                            .get("skeleton_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("skeleton");
                        let task_desc = args
                            .get("task_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let caps: Vec<String> = args
                            .get("capabilities")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let mode_hint = args.get("mode_hint").and_then(|v| v.as_str());

                        let intent = crate::wizard::AgentIntent {
                            name: skeleton_name.to_string(),
                            description: format!("Temporary skeleton for manager {}", aid),
                            task: task_desc.to_string(),
                            skills: vec![],
                            model_tier: "simple".to_string(),
                            scheduled: false,
                            schedule: None,
                            capabilities: caps,
                            mode: mode_hint.map(|s| s.to_string()),
                        };

                        let _plan = crate::wizard::SetupWizard::build_plan(intent);
                        let skeleton_id = sk_types::AgentId::new();

                        // If mode_hint is 'unrestricted', we DON'T force sandbox.
                        let is_unrestricted = mode_hint == Some("unrestricted");
                        if !is_unrestricted {
                            let _ = kernel.memory.structured.set(
                                skeleton_id,
                                "forced_sandbox",
                                serde_json::Value::Bool(true),
                            );
                        }

                        // Create initialization session
                        let mut skeleton_session = sk_types::Session::new(skeleton_id);
                        let startup_message = format!("You are a spawned witch_skeleton. Your task is: {}\nWhen you finish or need help, use the agent_message tool to message your manager agent ID: {}", task_desc, aid);
                        skeleton_session.push_message(sk_types::Message::user(&startup_message));

                        let _ = kernel.memory.sessions.save(&skeleton_session);

                        let skeleton_id_str = skeleton_id.to_string();
                        let kernel_clone = kernel.clone();

                        // Spawn background skeleton execution
                        tokio::spawn(async move {
                            if let Ok(Some(mut session)) =
                                kernel_clone.memory.sessions.load(skeleton_session.id)
                            {
                                let _ =
                                    kernel_clone.run_agent(&mut session, &startup_message).await;
                            }
                        });

                        Ok(healer_result(&tool_id, format!("Witch Skeleton summoned successfully! Skeleton ID: {}. It is running in Sandbox mode in the background.", skeleton_id_str), false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "check_skeleton" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let skeleton_id_str = args
                            .get("skeleton_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if let Ok(skeleton_id) = std::str::FromStr::from_str(skeleton_id_str) {
                            if let Ok(sessions) = kernel.memory.sessions.list_for_agent(skeleton_id)
                            {
                                if let Some((session_id, _, _)) = sessions.first() {
                                    if let Ok(Some(session)) =
                                        kernel.memory.sessions.load(*session_id)
                                    {
                                        let last_msg =
                                            session.messages.last().map(|m| m.content.clone());
                                        Ok(healer_result(
                                            &tool_id,
                                            format!("Skeleton latest activity: {:?}", last_msg),
                                            false,
                                        ))
                                    } else {
                                        Ok(healer_result(
                                            &tool_id,
                                            "Skeleton initialized, but no activity yet."
                                                .to_string(),
                                            false,
                                        ))
                                    }
                                } else {
                                    Ok(healer_result(
                                        &tool_id,
                                        "Skeleton has no active session.".to_string(),
                                        false,
                                    ))
                                }
                            } else {
                                Ok(healer_result(
                                    &tool_id,
                                    "Failed to check skeleton.".to_string(),
                                    false,
                                ))
                            }
                        } else {
                            Err(sk_types::SovereignError::ToolExecutionError(
                                "Invalid skeleton_id format".into(),
                            ))
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "builder" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let task_str = args.get("task").and_then(|v| v.as_str()).unwrap_or("");
                        let _k = kernel.clone();
                        let model_name = kernel.model_name.clone();
                        let driver = kernel.driver.clone();

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let intent = crate::wizard::SetupWizard::analyze_task_intent(driver, &model_name, task_str).await?;
                                let plan = crate::wizard::SetupWizard::build_plan(intent);

                                Ok(healer_result(&tool_id, format!(
                                    "I have analyzed your request and prepared a setup plan:\n\n{}\n\nTo summon this worker, use the `summon_skeleton` tool with the following parameters:\n- skeleton_name: {}\n- task_description: {}\n- capabilities: {:?}\n- mode_hint: {}",
                                    plan.summary,
                                    plan.intent.name,
                                    plan.intent.task,
                                    plan.intent.capabilities,
                                    plan.intent.mode.as_deref().unwrap_or("safe")
                                ), false))
                            })
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "shared_memory_store" => {
                    // Requires SharedMemory capability
                    let granted = kernel
                        .memory
                        .structured
                        .get(aid, "capabilities")
                        .unwrap_or(None)
                        .map(|v| {
                            serde_json::from_value::<Vec<sk_types::capability::Capability>>(v)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    // We must check if the agent actually has this Capability
                    let has_cap = granted.iter().any(|c| {
                        sk_types::capability::capability_matches(
                            c,
                            &sk_types::capability::Capability::SharedMemory,
                        )
                    });
                    if !has_cap && mode != sk_types::config::ExecutionMode::Unrestricted {
                        return Err(sk_types::SovereignError::CapabilityDenied(
                            "Agent lacks `SharedMemory` capability".into(),
                        ));
                    }

                    if let Some(args) = sanitized_input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("");
                        match kernel.memory.shared.store(aid, content, topic) {
                            Ok(_) => Ok(healer_result(
                                &tool_id,
                                "Successfully stored fact in shared semantic memory.".to_string(),
                                false,
                            )),
                            Err(e) => {
                                Err(sk_types::SovereignError::ToolExecutionError(e.to_string()))
                            }
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "shared_memory_recall" => {
                    // Requires SharedMemory capability
                    let granted = kernel
                        .memory
                        .structured
                        .get(aid, "capabilities")
                        .unwrap_or(None)
                        .map(|v| {
                            serde_json::from_value::<Vec<sk_types::capability::Capability>>(v)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    let has_cap = granted.iter().any(|c| {
                        sk_types::capability::capability_matches(
                            c,
                            &sk_types::capability::Capability::SharedMemory,
                        )
                    });
                    if !has_cap && mode != sk_types::config::ExecutionMode::Unrestricted {
                        return Err(sk_types::SovereignError::CapabilityDenied(
                            "Agent lacks `SharedMemory` capability".into(),
                        ));
                    }

                    if let Some(args) = sanitized_input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        match kernel.memory.shared.recall(query) {
                            Ok(results) => {
                                if results.is_empty() {
                                    Ok(healer_result(
                                        &tool_id,
                                        "No relevant shared knowledge found.".to_string(),
                                        false,
                                    ))
                                } else {
                                    let mut out = String::from("Recalled shared knowledge:\n");
                                    for (author, content, date) in results {
                                        out.push_str(&format!(
                                            "- [{}] (by {}): {}\n",
                                            date, author, content
                                        ));
                                    }
                                    Ok(healer_result(&tool_id, out, false))
                                }
                            }
                            Err(e) => {
                                Err(sk_types::SovereignError::ToolExecutionError(e.to_string()))
                            }
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "schedule_create" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let name = args
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unnamed_job");
                        let schedule_type = args
                            .get("schedule_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let task_desc = args
                            .get("task_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let every_secs = args.get("every_secs").and_then(|v| v.as_u64());
                        let cron_expr = args.get("cron_expr").and_then(|v| v.as_str());
                        let schedule = match schedule_type {
                            "every" => {
                                if let Some(secs) = every_secs {
                                    sk_types::scheduler::CronSchedule::Every { every_secs: secs }
                                } else {
                                    return Err(sk_types::SovereignError::ToolExecutionError(
                                        "Missing every_secs argument".into(),
                                    ));
                                }
                            }
                            "cron" => {
                                if let Some(expr) = cron_expr {
                                    sk_types::scheduler::CronSchedule::Cron {
                                        expr: expr.to_string(),
                                        tz: None,
                                    }
                                } else {
                                    return Err(sk_types::SovereignError::ToolExecutionError(
                                        "Missing cron_expr argument".into(),
                                    ));
                                }
                            }
                            _ => {
                                return Err(sk_types::SovereignError::ToolExecutionError(
                                    "Invalid schedule_type".into(),
                                ))
                            }
                        };
                        let job = sk_types::scheduler::CronJob {
                            id: sk_types::scheduler::CronJobId::new(),
                            agent_id: aid,
                            name: name.to_string(),
                            enabled: true,
                            schedule,
                            action: sk_types::scheduler::CronAction::AgentTurn {
                                message: task_desc.to_string(),
                                model_override: None,
                                timeout_secs: None,
                            },
                            delivery: sk_types::scheduler::CronDelivery::LastChannel,
                            created_at: chrono::Utc::now(),
                            last_run: None,
                            next_run: None,
                        };
                        match kernel.cron.add_job(job, false) {
                            Ok(id) => {
                                let _ = kernel.cron.persist();
                                Ok(healer_result(
                                    &tool_id,
                                    format!(
                                        "Scheduled job '{}' created successfully with ID: {}",
                                        name, id
                                    ),
                                    false,
                                ))
                            }
                            Err(e) => {
                                Err(sk_types::SovereignError::ToolExecutionError(e.to_string()))
                            }
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "schedule_list" => {
                    let jobs = kernel.cron.list_jobs(aid);
                    if jobs.is_empty() {
                        Ok(healer_result(
                            &tool_id,
                            "You have no scheduled jobs.".to_string(),
                            false,
                        ))
                    } else {
                        let mut out = String::from("Your scheduled jobs:\n");
                        for job in jobs {
                            out.push_str(&format!(
                                "- ID: {} | Name: '{}' | Enabled: {} | Next Run: {:?}\n",
                                job.id, job.name, job.enabled, job.next_run
                            ));
                        }
                        Ok(healer_result(&tool_id, out, false))
                    }
                }
                "schedule_delete" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let job_id_str = args.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
                        if let Ok(job_id) = std::str::FromStr::from_str(job_id_str) {
                            match kernel.cron.remove_job(job_id) {
                                Ok(_) => {
                                    let _ = kernel.cron.persist();
                                    Ok(healer_result(
                                        &tool_id,
                                        format!("Successfully deleted background job {}", job_id),
                                        false,
                                    ))
                                }
                                Err(e) => {
                                    Err(sk_types::SovereignError::ToolExecutionError(e.to_string()))
                                }
                            }
                        } else {
                            Err(sk_types::SovereignError::ToolExecutionError(
                                "Invalid job_id format".into(),
                            ))
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "remember" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_remember(&kernel.memory, aid, content)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "recall" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let limit = args
                            .get("limit")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize)
                            .unwrap_or(5);
                        sk_tools::memory_tools::handle_recall(&kernel.memory, aid, query, limit)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "forget" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let memory_id =
                            args.get("memory_id").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_forget(&kernel.memory, memory_id)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "web_search" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::web_search::handle_web_search(query)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "web_fetch" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(sk_tools::web_fetch::handle_web_fetch(url))
                        })
                        .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "read_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_read_file(&workspaces_dir, path)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "write_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let append = args
                            .get("append")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        sk_tools::file_ops::handle_write_file(
                            &workspaces_dir,
                            path,
                            content,
                            append,
                        )
                        .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_dir" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        sk_tools::file_ops::handle_list_dir(&workspaces_dir, path)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "delete_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_delete_file(&workspaces_dir, path)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "move_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args
                            .get("destination")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        sk_tools::file_ops::handle_move_file(&workspaces_dir, source, dest)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "copy_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args
                            .get("destination")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        sk_tools::file_ops::handle_copy_file(&workspaces_dir, source, dest)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "shell_exec" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                        let working_dir = args.get("working_dir").and_then(|v| v.as_str());
                        let timeout_secs = args.get("timeout_secs").and_then(|v| v.as_u64());
                        let use_sandbox = args
                            .get("use_sandbox")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if use_sandbox {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    let sandbox_config = &docker_cfg;
                                    let workspace = workspaces_dir.clone();
                                    let config_hash =
                                        sk_engine::runtime::docker_sandbox::config_hash(
                                            sandbox_config,
                                        );

                                    let container = if let Some(c) = kernel
                                        .sandbox_pool
                                        .acquire(config_hash, sandbox_config.reuse_cool_secs)
                                    {
                                        c
                                    } else {
                                        sk_engine::runtime::docker_sandbox::create_sandbox(
                                            sandbox_config,
                                            &agent_id_str,
                                            &workspace,
                                        )
                                        .await
                                        .map_err(|e| {
                                            sk_types::SovereignError::ToolExecutionError(e)
                                        })?
                                    };

                                    let timeout =
                                        std::time::Duration::from_secs(timeout_secs.unwrap_or(30));
                                    let res = sk_engine::runtime::docker_sandbox::exec_in_sandbox(
                                        &container, command, timeout,
                                    )
                                    .await;

                                    kernel.sandbox_pool.release(container, config_hash);

                                    match res {
                                        Ok(exec_res) => {
                                            let mut response = String::new();
                                            response.push_str(&format!(
                                                "Exit Code: {}\n",
                                                exec_res.exit_code
                                            ));
                                            if !exec_res.stdout.trim().is_empty() {
                                                response.push_str(&format!(
                                                    "STDOUT:\n{}\n",
                                                    exec_res.stdout.trim()
                                                ));
                                            }
                                            if !exec_res.stderr.trim().is_empty() {
                                                response.push_str(&format!(
                                                    "STDERR:\n{}\n",
                                                    exec_res.stderr.trim()
                                                ));
                                            }
                                            if response.trim()
                                                == format!("Exit Code: {}", exec_res.exit_code)
                                            {
                                                response.push_str(
                                                    "Command executed successfully with no output.",
                                                );
                                            }
                                            Ok(healer_result(&tool_id, response, false))
                                        }
                                        Err(e) => {
                                            Err(sk_types::SovereignError::ToolExecutionError(e))
                                        }
                                    }
                                })
                            })
                        } else {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current()
                                    .block_on(sk_tools::shell::handle_shell_exec(
                                        &exec_policy,
                                        command,
                                        working_dir,
                                        timeout_secs,
                                    ))
                                    .map(|out| healer_result(&tool_id, out, false))
                            })
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "code_exec" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("");
                        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
                        let use_sandbox = args
                            .get("use_sandbox")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if use_sandbox {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    let sandbox_config = &docker_cfg;
                                    let workspace = workspaces_dir.clone();

                                    // 1. Create a script file in the workspace subfolder
                                    let file_id = uuid::Uuid::new_v4().to_string();
                                    let temp_rel_dir = format!(".sk_temp/code_{}", file_id);
                                    let temp_abs_dir = workspace.join(&temp_rel_dir);
                                    std::fs::create_dir_all(&temp_abs_dir).map_err(|e| {
                                        sk_types::SovereignError::ToolExecutionError(e.to_string())
                                    })?;

                                    let script_filename = match language {
                                        "python" => "script.py",
                                        "node" => "script.js",
                                        "bash" => "script.sh",
                                        _ => {
                                            return Err(
                                                sk_types::SovereignError::ToolExecutionError(
                                                    format!("Unsupported language: {}", language),
                                                ),
                                            )
                                        }
                                    };
                                    let script_abs_path = temp_abs_dir.join(script_filename);
                                    std::fs::write(&script_abs_path, code).map_err(|e| {
                                        sk_types::SovereignError::ToolExecutionError(e.to_string())
                                    })?;

                                    // 2. Prepare docker command
                                    let binary = match language {
                                        "python" => "python3",
                                        "node" => "node",
                                        "bash" => "sh",
                                        _ => unreachable!(),
                                    };
                                    let container_script_path = format!(
                                        "{}/{}/{}",
                                        sandbox_config.workdir, temp_rel_dir, script_filename
                                    );
                                    let command = format!("{} {}", binary, container_script_path);

                                    // 3. Run in sandbox
                                    let config_hash =
                                        sk_engine::runtime::docker_sandbox::config_hash(
                                            sandbox_config,
                                        );
                                    let container = if let Some(c) = kernel
                                        .sandbox_pool
                                        .acquire(config_hash, sandbox_config.reuse_cool_secs)
                                    {
                                        c
                                    } else {
                                        sk_engine::runtime::docker_sandbox::create_sandbox(
                                            sandbox_config,
                                            &aid.to_string(), // Fixed: was using undefined agent_id_str
                                            &workspace,
                                        )
                                        .await
                                        .map_err(|e| {
                                            sk_types::SovereignError::ToolExecutionError(e)
                                        })?
                                    };

                                    let timeout = std::time::Duration::from_secs(30);
                                    let res = sk_engine::runtime::docker_sandbox::exec_in_sandbox(
                                        &container, &command, timeout,
                                    )
                                    .await;

                                    kernel.sandbox_pool.release(container, config_hash);

                                    // Cleanup temp file
                                    let _ = std::fs::remove_dir_all(&temp_abs_dir);

                                    match res {
                                        Ok(exec_res) => {
                                            let mut response = String::new();
                                            response.push_str(&format!(
                                                "Exit Code: {}\n",
                                                exec_res.exit_code
                                            ));
                                            if !exec_res.stdout.trim().is_empty() {
                                                response.push_str(&format!(
                                                    "STDOUT:\n{}\n",
                                                    exec_res.stdout.trim()
                                                ));
                                            }
                                            if !exec_res.stderr.trim().is_empty() {
                                                response.push_str(&format!(
                                                    "STDERR:\n{}\n",
                                                    exec_res.stderr.trim()
                                                ));
                                            }
                                            if response.trim()
                                                == format!("Exit Code: {}", exec_res.exit_code)
                                            {
                                                response.push_str(
                                                    "Script executed successfully with no output.",
                                                );
                                            }
                                            Ok(healer_result(&tool_id, response, false))
                                        }
                                        Err(e) => {
                                            Err(sk_types::SovereignError::ToolExecutionError(e))
                                        }
                                    }
                                })
                            })
                        } else {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current()
                                    .block_on(sk_tools::code_exec::handle_code_exec(
                                        &exec_policy,
                                        language,
                                        code,
                                    ))
                                    .map(|out| healer_result(&tool_id, out, false))
                            })
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "ottos_outpost" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("");
                        let exec_env_str = args
                            .get("execution_env")
                            .and_then(|v| v.as_str())
                            .unwrap_or("docker");
                        let exec_env = match exec_env_str {
                            "native" => sk_engine::runtime::ottos_outpost::ExecutionEnv::Native,
                            _ => sk_engine::runtime::ottos_outpost::ExecutionEnv::Docker,
                        };

                        let dependencies = args
                            .get("dependencies")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");

                        let mut input_files = Vec::new();
                        if let Some(files) = args.get("input_files").and_then(|v| v.as_array()) {
                            for f in files {
                                if let Some(fname) = f.get("filename").and_then(|v| v.as_str()) {
                                    if let Some(fcontent) =
                                        f.get("content").and_then(|v| v.as_str())
                                    {
                                        input_files.push((fname.to_string(), fcontent.to_string()));
                                    }
                                }
                            }
                        }

                        let req = sk_engine::runtime::ottos_outpost::OttosOutpostRequest {
                            language: language.to_string(),
                            execution_env: exec_env,
                            dependencies,
                            code: code.to_string(),
                            input_files,
                        };

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let workspace = workspaces_dir.clone();
                                match sk_engine::runtime::ottos_outpost::execute_ottos_outpost(req, &workspace).await {
                                    Ok(res) => {
                                        let mut response = String::new();
                                        response.push_str(&format!("Exit Code: {}\n", res.exit_code));
                                        if !res.stdout.trim().is_empty() {
                                            response.push_str(&format!("STDOUT:\n{}\n", res.stdout.trim()));
                                        }
                                        if !res.stderr.trim().is_empty() {
                                            response.push_str(&format!("STDERR:\n{}\n", res.stderr.trim()));
                                        }
                                        if response.trim() == format!("Exit Code: {}", res.exit_code) {
                                            response.push_str("Tool synthesized and executed successfully with no output.");
                                        }
                                        Ok(healer_result(&tool_id, response, false))
                                    }
                                    Err(e) => Err(sk_types::SovereignError::ToolExecutionError(e.to_string())),
                                }
                            })
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_navigate" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_navigate(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "browser_click" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_click(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "browser_type" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_type(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "browser_screenshot" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_screenshot(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "browser_read_page" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_read_page(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "browser_close" => tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        sk_engine::runtime::browser::tool_browser_close(
                            &sanitized_input,
                            &browser,
                            &aid.to_string(),
                        ),
                    )
                })
                .map(|out| healer_result(&tool_id, out, false))
                .map_err(sk_types::SovereignError::ToolExecutionError),
                "get_skill" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let lock = kernel.skills.read().unwrap();
                        Ok(healer_result(
                            &tool_id,
                            sk_tools::skills::handle_get_skill(&lock, name),
                            false,
                        ))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_skills" => {
                    let lock = kernel.skills.read().unwrap();
                    Ok(healer_result(
                        &tool_id,
                        sk_tools::skills::handle_list_skills(&lock),
                        false,
                    ))
                }
                "text_to_speech" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let voice = args.get("voice").and_then(|v| v.as_str());
                        let output_path = args.get("output_path").and_then(|v| v.as_str());

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(sk_tools::voice_tools::handle_text_to_speech(
                                    text,
                                    voice,
                                    output_path,
                                ))
                                .map(|out| healer_result(&tool_id, out, false))
                                .map_err(sk_types::SovereignError::ToolExecutionError)
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "speech_to_text" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let file_path =
                            args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(sk_tools::voice_tools::handle_speech_to_text(file_path))
                                .map(|out| healer_result(&tool_id, out, false))
                                .map_err(sk_types::SovereignError::ToolExecutionError)
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "compile_rust_skill" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let skill_name = args
                            .get("skill_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let description = args
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let dependencies_toml = args
                            .get("dependencies_toml")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let code = args
                            .get("code")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let instructions = args
                            .get("instructions")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                            // 1. Prepare Cargo workspace
                            let file_id = uuid::Uuid::new_v4().to_string();
                            let temp_rel_dir = format!(".sk_temp/compile_rust_{}_{}", skill_name, file_id);
                            let workspace = workspaces_dir.clone();
                            let temp_abs_dir = workspace.join(&temp_rel_dir);
                            if let Err(e) = std::fs::create_dir_all(&temp_abs_dir) {
                                return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to create temp dir: {}", e)));
                            }

                            // 2. Write Cargo.toml
                            let cargo_toml = format!(r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
"#, skill_name, dependencies_toml);
                            if let Err(e) = std::fs::write(temp_abs_dir.join("Cargo.toml"), cargo_toml) {
                                return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to write Cargo.toml: {}", e)));
                            }

                            // 3. Write src/main.rs
                            if let Err(e) = std::fs::create_dir_all(temp_abs_dir.join("src")) {
                                return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to create src dir: {}", e)));
                            }
                            if let Err(e) = std::fs::write(temp_abs_dir.join("src/main.rs"), code) {
                                return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to write main.rs: {}", e)));
                            }

                            // 4. Run `cargo build --release` in Docker sandbox using rust image
                            let mut build_config = docker_cfg.clone();
                            build_config.image = "rust:1.80-slim".to_string();

                            let config_hash = sk_engine::runtime::docker_sandbox::config_hash(&build_config);
                            let container = if let Some(c) = kernel.sandbox_pool.acquire(config_hash, build_config.reuse_cool_secs) {
                                c
                            } else {
                                sk_engine::runtime::docker_sandbox::create_sandbox(&build_config, &aid.to_string(), &workspace)
                                    .await.map_err(|e| sk_types::SovereignError::ToolExecutionError(format!("Docker create failed: {}", e)))?
                            };

                            // Docker container runs with `workspace` mounted to `build_config.workdir`
                            let container_script_path = format!("{}/{}", build_config.workdir, temp_rel_dir);
                            let command = format!("cd {} && cargo build --release", container_script_path);

                            let timeout = std::time::Duration::from_secs(300); // 5 min compilation timeout
                            let res = sk_engine::runtime::docker_sandbox::exec_in_sandbox(&container, &command, timeout).await;

                            kernel.sandbox_pool.release(container, config_hash);

                            match res {
                                Ok(exec_res) if exec_res.exit_code == 0 => {
                                    // 5. Success! Move binary to skills directory.
                                    let lock = kernel.skills.write().unwrap();
                                    let skills_dir = lock.dir.clone();
                                    drop(lock);

                                    let target_skill_dir = skills_dir.join(&skill_name);
                                    if let Err(e) = std::fs::create_dir_all(&target_skill_dir) {
                                         return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to create target skill dir: {}", e)));
                                    }

                                    // Extract compiled binary
                                    let binary_src = temp_abs_dir.join("target/release").join(&skill_name);
                                    let binary_dst = target_skill_dir.join(&skill_name);

                                    if let Err(e) = std::fs::copy(&binary_src, &binary_dst) {
                                         return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to copy binary (did it compile successfully?): {}\nSTDOUT:\n{}\nSTDERR:\n{}", e, exec_res.stdout, exec_res.stderr)));
                                    }

                                    #[cfg(unix)]
                                    {
                                        use std::os::unix::fs::PermissionsExt;
                                        if let Ok(mut perms) = std::fs::metadata(&binary_dst).map(|m| m.permissions()) {
                                            perms.set_mode(0o755);
                                            let _ = std::fs::set_permissions(&binary_dst, perms);
                                        }
                                    }

                                    // 6. Write SKILL.md
                                    let skill_md_content = format!("---\nname: {}\ndescription: {}\nmetadata:\n  compiled: true\n---\n{}", skill_name, description, instructions);
                                    if let Err(e) = std::fs::write(target_skill_dir.join("SKILL.md"), skill_md_content) {
                                         return Err(sk_types::SovereignError::ToolExecutionError(format!("Failed to write SKILL.md: {}", e)));
                                    }

                                    // Cleanup temp compilation dir
                                    let _ = std::fs::remove_dir_all(&temp_abs_dir);

                                    // 7. Hot reload!
                                    let mut lock = kernel.skills.write().unwrap();
                                    lock.reload();
                                    drop(lock);

                                    Ok(healer_result(&tool_id, format!("Successfully compiled Rust skill '{}' and hot-reloaded the registry!", skill_name), false))
                                }
                                Ok(exec_res) => {
                                    Err(sk_types::SovereignError::ToolExecutionError(format!("cargo build failed. Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", exec_res.exit_code, exec_res.stdout, exec_res.stderr)))
                                }
                                Err(e) => {
                                    Err(sk_types::SovereignError::ToolExecutionError(format!("Docker execution failed: {}", e)))
                                }
                            }
                        })
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_read_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::file_full::handle_host_read_file(path)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_write_file" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let append = args
                            .get("append")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        sk_tools::host::file_full::handle_host_write_file(path, content, append)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_list_dir" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::file_full::handle_host_list_dir(path)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_desktop_control" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
                        let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::desktop_control::handle_desktop_control(action, value)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_system_config" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
                        let target = args.get("target").and_then(|v| v.as_str());
                        let value = args.get("value").and_then(|v| v.as_str());
                        sk_tools::host::system_config::handle_system_config(action, target, value)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_install_app" => {
                    if let Some(args) = sanitized_input.as_object() {
                        let package_id = args
                            .get("package_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        sk_tools::host::app_installer::handle_app_installer(package_id)
                            .map(|out| healer_result(&tool_id, out, false))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                _ => {
                    let is_mcp = {
                        if let Ok(mcp_lock) = kernel.mcp.try_read() {
                            mcp_lock.is_mcp_tool(&tool_call.name)
                        } else {
                            false
                        }
                    };

                    if is_mcp {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let mut mcp_lock = kernel.mcp.write().await;
                                match mcp_lock.call_tool(&tool_call.name, &sanitized_input).await {
                                    Ok(res) => Ok(healer_result(&tool_id, res, false)),
                                    Err(e) => Err(sk_types::SovereignError::ToolExecutionError(
                                        e.to_string(),
                                    )),
                                }
                            })
                        })
                    } else {
                        // --- Graceful Unknown Tool Handler ---
                        // Instead of returning an error (which trips the circuit breaker),
                        // return a helpful nudge so the model corrects itself.
                        tracing::warn!(tool = %tool_call.name, "Model called non-existent tool, returning nudge");
                        Ok(healer_result(
                            &tool_id,
                            format!(
                                "Error: '{}' is not a valid tool. You can ONLY call these tools: remember, recall, list_dir, read_file, write_file, shell_exec, web_search, web_fetch. Please try again with one of these exact names.",
                                tool_call.name
                            ),
                            true, // is_error = true so model knows to correct
                        ))
                    }
                }
            }?;

            // 3. Act on Signals
            if let Some(signal) = &result.signal {
                match signal {
                    sk_types::ToolSignal::VillageForge { task_description } => {
                        info!(agent = %aid, task = %task_description, "Intercepted VillageForge signal — Initiating capability forge");

                        let kernel_clone = kernel.clone();
                        let task_clone = task_description.clone();

                        let forge_res = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async move {
                                // 1. Analyze intent via SetupWizard
                                let driver = kernel_clone.driver.clone();
                                let model = kernel_clone.model_name.clone();

                                info!("Forging capability: Analyzing task...");
                                let intent = crate::wizard::SetupWizard::analyze_task_intent(
                                    driver, &model, &task_clone
                                ).await?;

                                let hand_id = format!("forged-{}", intent.name.to_lowercase().replace(" ", "-"));
                                info!(hand_id = %hand_id, "Generating Hand specification...");

                                // 2. Generate Hand Spec (TOML)
                                let toml_content = format!(r#"
id = "{}"
name = "{}"
description = "{}"
category = "productivity"
tools = {:?}

[agent]
name = "{}"
description = "A specialized forged worker for {}"
system_prompt = "You are a specialized worker focused on: {}. Your goal is to assist the primary user in this specific domain."
"#, hand_id, intent.name, intent.task, intent.capabilities, intent.name, intent.name, intent.task);

                                // 3. Save to custom hands dir (~/.sovereign/hands/)
                                let hands_dir = kernel_clone
                                    .config
                                    .read()
                                    .unwrap()
                                    .data_dir
                                    .join("hands");
                                if !hands_dir.exists() {
                                    let _ = std::fs::create_dir_all(&hands_dir);
                                }

                                let file_path = hands_dir.join(format!("{}.toml", hand_id));
                                info!(path = %file_path.display(), "Saving forged Hand specification...");
                                std::fs::write(&file_path, toml_content).map_err(|e| {
                                    sk_types::SovereignError::Internal(format!("Failed to save forged hand: {}", e))
                                })?;

                                // 4. Reload Hands Registry
                                info!("Reloading Hand Registry...");
                                let mut lock = kernel_clone.hands.write().unwrap();
                                lock.load_custom_hands(&hands_dir);
                                drop(lock);

                                Ok::<String, sk_types::SovereignError>(hand_id)
                            })
                        });

                        match forge_res {
                            Ok(hand_id) => {
                                result.content = format!("CONGRATULATIONS! I have successfully forged a new capability: `{}`. \n\nThis Hand is now registered and ready to be summoned. You can now use the `summon_skeleton` tool with `skeleton_name: \"{}\"` to activate this specialized capability. \n\nOriginal Task: {}", hand_id, hand_id, task_description);
                            }
                            Err(e) => {
                                result.content = format!("FORGE FAILURE: I attempted to forge the capability for '{}' but encountered an error: {}. Please try again or refine your request.", task_description, e);
                                result.is_error = true;
                            }
                        }
                    }
                }
            }

            Ok(result)
        }),
    }
}

/// The Healer: Truncates large tool outputs to prevent context bloat.
pub fn healer(output: String, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output;
    }

    let keep = max_chars / 2;
    let head = &output[..keep];
    let tail = &output[output.len() - keep..];
    let removed = output.len() - max_chars;

    format!(
        "{}\n\n[... THE HEALER: Truncated {} characters to save tokens. Use a more specific command or read a sub-range if you need the full data. ...]\n\n{}",
        head, removed, tail
    )
}

/// Helper to wrap a string result into a ToolResult.
pub fn healer_result(tool_use_id: &str, output: String, is_error: bool) -> sk_types::ToolResult {
    sk_types::ToolResult {
        tool_use_id: tool_use_id.to_string(),
        content: healer(output, 8000),
        is_error,
        signal: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healer_no_truncation() {
        let input = "hello world".to_string();
        let healed = healer(input.clone(), 50);
        assert_eq!(healed, input);
    }

    #[test]
    fn test_healer_truncation() {
        let mut input = String::new();
        for i in 0..10_000 {
            input.push_str(&format!("Line {}\n", i));
        }

        // input has > 70,000 characters
        let original_len = input.len();
        let healed = healer(input, 1000);

        // Should be around ~1000 chars + the marker string
        assert!(healed.len() > 1000);
        assert!(healed.len() < 1200);

        assert!(healed.starts_with("Line 0\n"));
        // The marker should mention exactly how many characters were removed
        let marker = format!("Truncated {} characters", original_len - 1000);
        assert!(healed.contains(&marker));
        assert!(healed.ends_with("Line 9999\n"));
    }
}
