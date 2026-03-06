use sk_engine::agent_loop::AgentLoopConfig;
use sk_types::AgentId;
use std::sync::Arc;

use crate::SovereignKernel;

/// Creates a standardized AgentLoopConfig with all default tools registered.
pub fn create_agent_config<'a>(
    kernel: Arc<SovereignKernel>,
    driver: &'a dyn sk_engine::llm_driver::LlmDriver,
    system_prompt: String,
    model_name: String,
    agent_id: AgentId,
    browser_manager: Arc<sk_engine::media::browser::BrowserManager>,
    skill_registry: Arc<sk_tools::skills::SkillRegistry>,
) -> AgentLoopConfig<'a> {
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
    tools.push(sk_tools::skills::get_skill_tool());
    tools.push(sk_tools::skills::list_skills_tool());

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
        name: "agent_spawn_worker".into(),
        description: "Dynamically spawn a background worker agent. It will run in Sandbox mode and will ask the user for permission on actions. You can continue working while it runs. Use agent_check_worker to see its status.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "worker_name": { "type": "string", "description": "Name of the worker (e.g. 'researcher')" },
                "task_description": { "type": "string", "description": "What the worker should do. Provide complete details and goals." },
                "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Capabilities the worker needs (e.g. 'web', 'file_read', 'browser')" }
            },
            "required": ["worker_name", "task_description", "capabilities"]
        }),
    });

    tools.push(sk_types::ToolDefinition {
        name: "agent_check_worker".into(),
        description: "Check the latest status of a spawned worker agent.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "worker_id": { "type": "string", "description": "The Agent ID of the worker" }
            },
            "required": ["worker_id"]
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
        tool_executor: Box::new(move |tool_call| {
            let kernel = k.clone();
            let browser = b.clone();
            let aid = aid.clone();
            let skills = skill_registry.clone();
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
                "agent_spawn_worker" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let worker_name = args
                            .get("worker_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("worker");
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

                        let intent = crate::wizard::AgentIntent {
                            name: worker_name.to_string(),
                            description: format!("Temporary worker for parent {}", aid),
                            task: task_desc.to_string(),
                            skills: vec![],
                            model_tier: "simple".to_string(),
                            scheduled: false,
                            schedule: None,
                            capabilities: caps,
                        };

                        let _plan = crate::wizard::SetupWizard::build_plan(intent);
                        let worker_id = sk_types::AgentId::new();

                        // Per User request, FORCED to Sandbox mode.
                        let _ = kernel.memory.structured.set(
                            worker_id,
                            "forced_sandbox",
                            serde_json::Value::Bool(true),
                        );

                        // Create initialization session
                        let mut worker_session = sk_types::Session::new(worker_id.clone());
                        let startup_message = format!("You are a spawned worker agent. Your task is: {}\nWhen you finish or need help, use the agent_message tool to message your manager agent ID: {}", task_desc, aid);
                        worker_session.push_message(sk_types::Message::user(&startup_message));

                        let _ = kernel.memory.sessions.save(&worker_session);

                        let worker_id_str = worker_id.to_string();
                        let kernel_clone = kernel.clone();

                        // Spawn background worker execution
                        tokio::spawn(async move {
                            if let Ok(session) =
                                kernel_clone.memory.sessions.load(worker_session.id)
                            {
                                if let Some(mut session) = session {
                                    let _ = kernel_clone
                                        .run_agent(&mut session, &startup_message)
                                        .await;
                                }
                            }
                        });

                        Ok(format!("Worker spawned successfully! Worker ID: {}. It is running in Sandbox mode in the background.", worker_id_str))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "agent_check_worker" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let worker_id_str =
                            args.get("worker_id").and_then(|v| v.as_str()).unwrap_or("");
                        if let Ok(worker_id) = std::str::FromStr::from_str(worker_id_str) {
                            if let Ok(sessions) = kernel.memory.sessions.list_for_agent(worker_id) {
                                if let Some((session_id, _, _)) = sessions.first() {
                                    if let Ok(Some(session)) =
                                        kernel.memory.sessions.load(*session_id)
                                    {
                                        let last_msg =
                                            session.messages.last().map(|m| m.content.clone());
                                        Ok(format!("Worker latest activity: {:?}", last_msg))
                                    } else {
                                        Ok("Worker initialized, but no activity yet.".to_string())
                                    }
                                } else {
                                    Ok("Worker has no active session.".to_string())
                                }
                            } else {
                                Ok("Failed to check worker.".to_string())
                            }
                        } else {
                            Err(sk_types::SovereignError::ToolExecutionError(
                                "Invalid worker_id format".into(),
                            ))
                        }
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
                        .get(aid.clone(), "capabilities")
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
                        .get(aid.clone(), "capabilities")
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
                            agent_id: aid.clone(),
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
                    let jobs = kernel.cron.list_jobs(aid.clone());
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
                        sk_tools::memory_tools::handle_remember(
                            &kernel.memory,
                            aid.clone(),
                            content,
                        )
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
                        sk_tools::memory_tools::handle_recall(
                            &kernel.memory,
                            aid.clone(),
                            query,
                            limit,
                        )
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

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_tools::shell::handle_shell_exec(
                                    &config.exec_policy,
                                    command,
                                    working_dir,
                                    timeout_secs,
                                ),
                            )
                        })
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

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                sk_tools::code_exec::handle_code_exec(
                                    &config.exec_policy,
                                    language,
                                    code,
                                ),
                            )
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
                                sk_engine::media::browser::tool_browser_navigate(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
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
                                sk_engine::media::browser::tool_browser_click(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
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
                                sk_engine::media::browser::tool_browser_type(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
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
                                sk_engine::media::browser::tool_browser_screenshot(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
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
                                sk_engine::media::browser::tool_browser_read_page(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
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
                                sk_engine::media::browser::tool_browser_close(
                                    &tool_call.input,
                                    &browser,
                                    &agent_id_str,
                                ),
                            )
                        })
                        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "get_skill" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        Ok(sk_tools::skills::handle_get_skill(&skills, name))
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_skills" => Ok(sk_tools::skills::handle_list_skills(&skills)),
                _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
                    "Unknown tool: {}",
                    tool_call.name
                ))),
            }
        }),
        stream_handler: None,
    }
}
