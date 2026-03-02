use sk_engine::agent_loop::AgentLoopConfig;
use sk_types::AgentId;
use std::sync::Arc;

/// Creates a standardized AgentLoopConfig with all default tools registered.
pub fn create_agent_config<'a>(
    driver: &'a dyn sk_engine::llm_driver::LlmDriver,
    system_prompt: String,
    model_name: String,
    kernel_memory: Arc<sk_memory::MemorySubstrate>,
    browser_manager: Arc<sk_engine::media::browser::BrowserManager>,
    agent_id: AgentId,
    skill_registry: Arc<sk_tools::skills::SkillRegistry>,
    safety_enabled: bool,
    safety_gate: Option<Arc<crate::safety::SafetyGate>>,
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
        sk_tools::shell::shell_exec_tool(),
    ];
    tools.extend(sk_tools::browser_tools::browser_tools());
    tools.push(sk_tools::skills::get_skill_tool());
    tools.push(sk_tools::skills::list_skills_tool());

    let k = kernel_memory;
    let b = browser_manager;
    let aid = agent_id;

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

            // Safety check
            if safety_enabled {
                let args = tool_call
                    .input
                    .clone();

                // If we have a specific gate, use it
                let blocked = if let Some(gate) = &safety_gate {
                    let enabled = gate.enabled && !gate.is_trust_all();
                    if enabled {
                        gate.check(&tool_call.name, &args, Some(&aid)).is_err()
                    } else {
                        false
                    }
                } else {
                    // Default safety check if no gate provided
                    crate::safety::classify_tool(&tool_call.name, &args)
                        == crate::safety::RiskLevel::Dangerous
                };

                if blocked {
                    // Try to get specific error message from check
                    let err_msg = if let Some(gate) = &safety_gate {
                        match gate.check(&tool_call.name, &args, Some(&aid)) {
                            Err(e) => e,
                            _ => format!(
                                "🛡️ SAFETY BLOCK: Tool '{}' requires approval.",
                                tool_call.name
                            ),
                        }
                    } else {
                        format!("🛡️ SAFETY BLOCK: Tool '{}' was blocked because it could be destructive. \
                                 The user needs to approve this action. \
                                 Tell the user what you want to do and ask them to reply 'approve' to allow it.",
                                 tool_call.name)
                    };

                    return Err(sk_types::SovereignError::ToolExecutionError(err_msg));
                }
            }

            match tool_call.name.as_str() {
                "remember" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_remember(
                            &kernel,
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
                        sk_tools::memory_tools::handle_recall(&kernel, aid.clone(), query, limit)
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
                        sk_tools::memory_tools::handle_forget(&kernel, memory_id)
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
                        sk_tools::web_fetch::handle_web_fetch(url)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "read_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_read_file(path)
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
                        sk_tools::file_ops::handle_write_file(path, content)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "list_dir" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        sk_tools::file_ops::handle_list_dir(path)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "shell_exec" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::shell::handle_shell_exec(command)
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
                "list_skills" => {
                    Ok(sk_tools::skills::handle_list_skills(&skills))
                }
                _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
                    "Unknown tool: {}",
                    tool_call.name
                ))),
            }
        }),
        stream_handler: None,
    }
}
