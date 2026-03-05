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
    ];
    tools.extend(sk_tools::browser_tools::browser_tools());
    tools.push(sk_tools::skills::get_skill_tool());
    tools.push(sk_tools::skills::list_skills_tool());

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
            let mode = config.execution_mode;

            // Enforce conversational SafetyGate in Sandbox mode
            if mode == sk_types::config::ExecutionMode::Sandbox {
                if let Err(detail) = kernel.safety.check(&tool_call.name, &tool_call.input, Some(&aid)) {
                    // Log the blocked action
                    let payload = serde_json::json!({
                        "tool": tool_call.name,
                        "args": tool_call.input,
                        "reason": "Safety Block",
                    });
                    if let Err(e) = kernel.memory.audit.append_log(&aid, "Sandbox", "tool_call_blocked", &payload) {
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
            if let Err(e) = kernel.memory.audit.append_log(&aid, mode_str, "tool_call", &payload) {
                tracing::warn!("Failed to append to audit log: {}", e);
            }

            match tool_call.name.as_str() {
                "remember" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::memory_tools::handle_remember(&kernel.memory, aid.clone(), content)
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
                        sk_tools::memory_tools::handle_recall(&kernel.memory, aid.clone(), query, limit)
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
                        sk_tools::file_ops::handle_read_file(&config.effective_workspaces_dir(), path)
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
                        let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
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
                        sk_tools::file_ops::handle_list_dir(&config.effective_workspaces_dir(), path)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "delete_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_delete_file(&config.effective_workspaces_dir(), path)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "move_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args.get("destination").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_move_file(&config.effective_workspaces_dir(), source, dest)
                    } else {
                        Err(sk_types::SovereignError::ToolExecutionError(
                            "Invalid arguments".into(),
                        ))
                    }
                }
                "copy_file" => {
                    if let Some(args) = tool_call.input.as_object() {
                        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let dest = args.get("destination").and_then(|v| v.as_str()).unwrap_or("");
                        sk_tools::file_ops::handle_copy_file(&config.effective_workspaces_dir(), source, dest)
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
                                )
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
                                )
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
