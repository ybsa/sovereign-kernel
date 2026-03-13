use sk_engine::agent_loop::AgentLoopConfig;
use std::sync::Arc;

use crate::SovereignKernel;

/// Creates a standardized AgentLoopConfig with all default tools registered.
pub fn create_agent_config(
    kernel: Arc<SovereignKernel>,
    driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    system_prompt: String,
    model_name: String,
    agent_id: sk_types::AgentId,
    browser_manager: Arc<sk_engine::runtime::browser::BrowserManager>,
    skill_registry: Arc<tokio::sync::RwLock<sk_tools::skills::SkillRegistry>>,
) -> AgentLoopConfig {
    let mut tools = vec![
        sk_tools::memory_tools::remember_tool(),
        sk_tools::memory_tools::recall_tool(),
        sk_tools::memory_tools::forget_tool(),
        sk_tools::web_search::web_search_tool(),
        sk_tools::web_fetch::web_fetch_tool(),
        sk_tools::file_ops::read_file_tool(),
        sk_tools::file_ops::write_file_tool(),
        sk_tools::file_ops::list_dir_tool(),
        sk_tools::file_ops::delete_file_tool(),
        sk_tools::file_ops::move_file_tool(),
        sk_tools::file_ops::copy_file_tool(),
        sk_tools::shell::shell_exec_tool(),
        sk_tools::code_exec::code_exec_tool(),
        sk_tools::shared_memory::shared_memory_store_tool(),
        sk_tools::shared_memory::shared_memory_recall_tool(),
        sk_tools::scheduler::schedule_create_tool(),
        sk_tools::scheduler::schedule_list_tool(),
        sk_tools::scheduler::schedule_delete_tool(),
    ];
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
        name: "spawn_witch_skeleton".into(),
        description: "Dynamically spawn a background witch_skeleton. It will run in Sandbox mode by default. You can continue working while it runs. Use check_witch_skeleton to see its status.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "skeleton_name": { "type": "string", "description": "Name of the skeleton (e.g. 'researcher')" },
                "task_description": { "type": "string", "description": "What the skeleton should do. Provide complete details and goals." },
                "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Capabilities the skeleton needs (e.g. 'web', 'file_read', 'browser')" },
                "mode_hint": { "type": "string", "enum": ["safe", "unrestricted", "scheduled"], "description": "Execution mode hint. 'safe' is forced sandbox. 'unrestricted' allows host access tools (requires approval). 'scheduled' creates a persistent cron job." }
            },
            "required": ["skeleton_name", "task_description", "capabilities"]
        }),
    });

    tools.push(sk_types::ToolDefinition {
        name: "builder".into(),
        description: "Transform a natural language task into a spawned agent village member. The kernel will automatically analyze the task, determine necessary tools, and set appropriate security modes.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "The mission for the agent (e.g. 'Monitor my CPU and notify me if it exceeds 90% for a minute')" }
            },
            "required": ["task"]
        }),
    });

    tools.push(sk_types::ToolDefinition {
        name: "check_witch_skeleton".into(),
        description: "Check the latest status of a spawned witch_skeleton.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "skeleton_id": { "type": "string", "description": "The Agent ID of the skeleton" }
            },
            "required": ["skeleton_id"]
        }),
    });

    let b = browser_manager;
    let aid = agent_id;
    let k = kernel;

    AgentLoopConfig {
        driver,
        system_prompt,
        tools,
        model: model_name,
        max_tokens: 4096,
        temperature: 0.7,
        stream_handler: None,
        checkpoint_handler: None,
        tool_executor: Box::new(move |tool_call| {
            let kernel = k.clone();
            let browser = b.clone();
            let aid = aid;
            let agent_id_str = aid.to_string();
            let config = kernel.config.clone();
            let default_mode = config.execution_mode;
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

            // Enforce conversational SafetyGate in Sandbox mode
            if mode == sk_types::config::ExecutionMode::Sandbox {
                if let Err(detail) =
                    kernel
                        .safety
                        .check(&tool_call.name, &tool_call.input, Some(&aid))
                {
                    // Log the blocked action
                    let payload = serde_json::json!({
                        "tool": tool_call.name,
                        "args": tool_call.input,
                        "reason": "Safety Block",
                    });
                    if let Err(e) = kernel.memory.audit.append_log(
                        &aid,
                        "Sandbox",
                        "tool_call_blocked",
                        &payload,
                    ) {
                        tracing::warn!("Failed to append to audit log: {}", e);
                    }
                    return Err(sk_types::SovereignError::ToolExecutionError(detail));
                }
            }

            // Log the approved/safe action execution
            let mode_str = match mode {
                sk_types::config::ExecutionMode::Sandbox => "Sandbox",
                sk_types::config::ExecutionMode::Unrestricted => "Unrestricted",
            };
            let payload = serde_json::json!({
                "tool": tool_call.name,
                "args": tool_call.input,
            });
            if let Err(e) = kernel
                .memory
                .audit
                .append_log(&aid, mode_str, "tool_call", &payload)
            {
                tracing::warn!("Failed to append to audit log: {}", e);
            }

            let skills = skill_registry.clone();

            match tool_call.name.as_str() {
                "agent_message" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let to_agent_id_str = args
                            .get("to_agent_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        if let Ok(to_id) = std::str::FromStr::from_str(to_agent_id_str) {
                            match kernel.bus.send(Some(&aid), &to_id, message.to_string()) {
                                Ok(_) => {
                                    Ok(format!("Message successfully sent to agent {}", to_id))
                                }
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
                "spawn_witch_skeleton" => {
                    if let Some(args) = tool_call.input.as_object() {
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

                        Ok(format!("Witch Skeleton spawned successfully! Skeleton ID: {}. It is running in Sandbox mode in the background.", skeleton_id_str))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "check_witch_skeleton" => {
                    if let Some(args) = tool_call.input.as_object() {
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
                                        Ok(format!("Skeleton latest activity: {:?}", last_msg))
                                    } else {
                                        Ok("Skeleton initialized, but no activity yet.".to_string())
                                    }
                                } else {
                                    Ok("Skeleton has no active session.".to_string())
                                }
                            } else {
                                Ok("Failed to check skeleton.".to_string())
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
                    if let Some(args) = tool_call.input.as_object() {
                        let task_str = args.get("task").and_then(|v| v.as_str()).unwrap_or("");
                        let _k = kernel.clone();
                        let model_name = kernel.model_name.clone();
                        let driver = kernel.driver.clone();

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let intent = crate::wizard::SetupWizard::analyze_task_intent(driver, &model_name, task_str).await?;
                                let plan = crate::wizard::SetupWizard::build_plan(intent);
                                
                                Ok(format!(
                                    "I have analyzed your request and prepared a setup plan:\n\n{}\n\nTo spawn this agent, use the `spawn_witch_skeleton` tool with the following parameters:\n- skeleton_name: {}\n- task_description: {}\n- capabilities: {:?}\n- mode_hint: {}",
                                    plan.summary,
                                    plan.intent.name,
                                    plan.intent.task,
                                    plan.intent.capabilities,
                                    plan.intent.mode.as_deref().unwrap_or("safe")
                                ))
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

                    if let Some(args) = tool_call.input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("");
                        match kernel.memory.shared.store(aid, content, topic) {
                            Ok(_) => {
                                Ok("Successfully stored fact in shared semantic memory."
                                    .to_string())
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

                    if let Some(args) = tool_call.input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        match kernel.memory.shared.recall(query) {
                            Ok(results) => {
                                if results.is_empty() {
                                    Ok("No relevant shared knowledge found.".to_string())
                                } else {
                                    let mut out = String::from("Recalled shared knowledge:\n");
                                    for (author, content, date) in results {
                                        out.push_str(&format!(
                                            "- [{}] (by {}): {}\n",
                                            date, author, content
                                        ));
                                    }
                                    Ok(out)
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
                    if let Some(args) = tool_call.input.as_object() {
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
                                Ok(format!(
                                    "Scheduled job '{}' created successfully with ID: {}",
                                    name, id
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
                        Ok("You have no scheduled jobs.".to_string())
                    } else {
                        let mut out = String::from("Your scheduled jobs:\n");
                        for job in jobs {
                            out.push_str(&format!(
                                "- ID: {} | Name: '{}' | Enabled: {} | Next Run: {:?}\n",
                                job.id, job.name, job.enabled, job.next_run
                            ));
                        }
                        Ok(out)
                    }
                }
                "schedule_delete" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let job_id_str = args.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
                        if let Ok(job_id) = std::str::FromStr::from_str(job_id_str) {
                            match kernel.cron.remove_job(job_id) {
                                Ok(_) => {
                                    let _ = kernel.cron.persist();
                                    Ok(format!("Successfully deleted background job {}", job_id))
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
                    if let Some(args) = tool_call.input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_remember(&kernel.memory, aid, content)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "recall" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let limit = args
                            .get("limit")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize)
                            .unwrap_or(5);
                        sk_tools::memory_tools::handle_recall(&kernel.memory, aid, query, limit)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "forget" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let memory_id =
                            args.get("memory_id").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_forget(&kernel.memory, memory_id)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "web_search" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::web_search::handle_web_search(query)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "web_fetch" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(sk_tools::web_fetch::handle_web_fetch(url))
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "read_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_read_file(
                            &config.effective_workspaces_dir(),
                            path,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "write_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let append = args
                            .get("append")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        sk_tools::file_ops::handle_write_file(
                            &config.effective_workspaces_dir(),
                            path,
                            content,
                            append,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_dir" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        sk_tools::file_ops::handle_list_dir(
                            &config.effective_workspaces_dir(),
                            path,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "delete_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_delete_file(
                            &config.effective_workspaces_dir(),
                            path,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "move_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args
                            .get("destination")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        sk_tools::file_ops::handle_move_file(
                            &config.effective_workspaces_dir(),
                            source,
                            dest,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "copy_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args
                            .get("destination")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        sk_tools::file_ops::handle_copy_file(
                            &config.effective_workspaces_dir(),
                            source,
                            dest,
                        )
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "shell_exec" => {
                    if let Some(args) = tool_call.input.as_object() {
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
                                    let sandbox_config = &config.docker;
                                    let workspace = config.effective_workspaces_dir();
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
                                            Ok(healer(response, 8000))
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
                                        &config.exec_policy,
                                        command,
                                        working_dir,
                                        timeout_secs,
                                    ))
                                    .map(|out| healer(out, 8000))
                            })
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "code_exec" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("");
                        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
                        let use_sandbox = args
                            .get("use_sandbox")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if use_sandbox {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    let sandbox_config = &config.docker;
                                    let workspace = config.effective_workspaces_dir();

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
                                            &agent_id_str,
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
                                            Ok(response)
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
                                        &config.exec_policy,
                                        language,
                                        code,
                                    ))
                                    .map(|out| healer(out, 8000))
                            })
                        }
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "ottos_outpost" => {
                    if let Some(args) = tool_call.input.as_object() {
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
                                let workspace = config.effective_workspaces_dir();
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
                                        // Pass through the healer to truncate if output is massive
                                        Ok(crate::executor::healer(response, 8000))
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
                "browser_navigate" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_navigate(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_click" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_click(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_type" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_type(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_screenshot" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_screenshot(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_read_page" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_read_page(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "browser_close" => {
                    if let Some(_args) = tool_call.input.as_object() {
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_engine::runtime::browser::tool_browser_close(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(sk_types::SovereignError::ToolExecutionError)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "get_skill" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let locks = skills.clone();
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let lock = locks.read().await;
                                Ok(sk_tools::skills::handle_get_skill(&lock, name))
                            })
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_read_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::file_full::handle_host_read_file(path)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_write_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
                        sk_tools::host::file_full::handle_host_write_file(path, content, append)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_list_dir" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::file_full::handle_host_list_dir(path)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_desktop_control" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
                        let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::desktop_control::handle_desktop_control(action, value)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_system_config" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
                        let target = args.get("target").and_then(|v| v.as_str());
                        let value = args.get("value").and_then(|v| v.as_str());
                        sk_tools::host::system_config::handle_system_config(action, target, value)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "host_install_app" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let package_id = args.get("package_id").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::host::app_installer::handle_app_installer(package_id)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_skills" => {
                    let locks = skills.clone();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let lock = locks.read().await;
                            Ok(sk_tools::skills::handle_list_skills(&lock))
                        })
                    })
                }
                "compile_rust_skill" => {
                    if let Some(args) = tool_call.input.as_object() {
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
                            let workspace = config.effective_workspaces_dir();
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
                            let mut build_config = config.docker.clone();
                            build_config.image = "rust:1.80-slim".to_string();

                            let config_hash = sk_engine::runtime::docker_sandbox::config_hash(&build_config);
                            let container = if let Some(c) = kernel.sandbox_pool.acquire(config_hash, build_config.reuse_cool_secs) {
                                c
                            } else {
                                sk_engine::runtime::docker_sandbox::create_sandbox(&build_config, &agent_id_str, &workspace)
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
                                    let lock = skills.write().await;
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
                                    let mut lock = skills.write().await;
                                    lock.reload();
                                    drop(lock);

                                    Ok(format!("Successfully compiled Rust skill '{}' and hot-reloaded the registry!", skill_name))
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
                                match mcp_lock.call_tool(&tool_call.name, &tool_call.input).await {
                                    Ok(res) => Ok(healer(res, 8000)),
                                    Err(e) => Err(sk_types::SovereignError::ToolExecutionError(
                                        e.to_string(),
                                    )),
                                }
                            })
                        })
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(format!(
                            "Unknown tool: {}",
                            tool_call.name
                        )))
                    }
                }
            }
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
