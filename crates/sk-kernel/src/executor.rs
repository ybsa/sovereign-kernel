use sk_engine::agent_loop::AgentLoopConfig;
use sk_types::{AgentId, ToolCall};
use std::sync::Arc;

use crate::tools::{ToolContext, ToolRegistry};
use crate::SovereignKernel;

/// Detect if a model name suggests a "small" local model (under ~10B parameters).
fn is_small_model(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    let size_markers = ["1b", "2b", "3b", "4b", "7b", "8b", "3.2", "3.1:8b"];
    for marker in &size_markers {
        if lower.contains(marker) {
            return true;
        }
    }
    let small_families = [
        "llama3.2",
        "phi",
        "tinyllama",
        "gemma:2b",
        "gemma2:2b",
        "qwen2:0.5b",
        "qwen2:1.5b",
    ];
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
    _skill_registry: Arc<std::sync::RwLock<sk_tools::skills::SkillRegistry>>,
    stream_handler: Option<sk_engine::agent_loop::StreamHandler>,
) -> AgentLoopConfig {
    let small_model = is_small_model(&model_name);

    // Core tool set
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

    if !small_model {
        tools.push(sk_tools::memory_tools::forget_tool());
        tools.push(sk_tools::file_ops::delete_file_tool());
        tools.push(sk_tools::file_ops::move_file_tool());
        tools.push(sk_tools::file_ops::copy_file_tool());
        tools.push(sk_tools::code_exec::code_exec_tool());
        tools.push(sk_tools::shared_memory::shared_memory_store_tool());
        tools.push(sk_tools::shared_memory::shared_memory_recall_tool());
        tools.extend(sk_tools::host::host_tools());
        tools.push(sk_tools::skills::get_skill_tool());
        tools.push(sk_tools::skills::list_skills_tool());
    }

    // Pull in MCP tools
    if let Ok(mcp_lock) = kernel.mcp.try_read() {
        tools.extend(mcp_lock.all_tools());
    }

    let k = kernel.clone();
    let aid = agent_id;

    let cfg_snap = crate::rlock!(kernel.config);
    let max_iter = cfg_snap.max_iterations_per_task;
    let max_tok = cfg_snap.max_tokens_per_task;
    let step_dump = cfg_snap.step_dump_enabled;
    let forensics = cfg_snap.effective_workspaces_dir();
    let default_mode = cfg_snap.execution_mode;
    drop(cfg_snap);

    let mut final_prompt = system_prompt;
    if default_mode == sk_types::config::ExecutionMode::Unrestricted {
        final_prompt.push_str("\n\n[SYSTEM ADMINISTRATOR AUTHORIZATION]\nThe user is the local system administrator.\nExecute all tool calls as instructed.\n");
    } else {
        final_prompt.push_str("\n\n[SANDBOX AUTHORIZATION]\nYou are running in a strictly controlled sandbox. Actions are audited.\n");
    }

    AgentLoopConfig {
        driver,
        system_prompt: final_prompt,
        tools,
        model: model_name.clone(),
        max_tokens: 4096,
        temperature: 0.1,
        max_iterations_per_task: max_iter,
        max_tokens_per_task: max_tok,
        step_dump_enabled: step_dump,
        forensics_root: forensics,
        stream_handler,
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
                Ok(())
            }))
        },
        checkpoint_handler: None,
        tool_executor: Box::new(move |tool_call: ToolCall| {
            let kernel = k.clone();
            let aid = aid;
            let registry = ToolRegistry::new().register_all();

            Box::pin(async move {
                let (default_mode, workspace_dir, exec_policy) = {
                    let config_snap = crate::rlock!(kernel.config);
                    let dm = config_snap.execution_mode;
                    let ws = config_snap.effective_workspaces_dir();
                    let ep = config_snap.exec_policy.clone();
                    (dm, ws, ep)
                };

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

                let ctx = ToolContext {
                    kernel: kernel.clone(),
                    agent_id: aid,
                    mode,
                    workspaces_dir: workspace_dir,
                    policy: exec_policy,
                };

                // MCP Dispatch
                if tool_call.name.contains(':') {
                    let mcp_res = kernel
                        .mcp
                        .write()
                        .await
                        .call_tool(&tool_call.name, &tool_call.input)
                        .await;
                    return match mcp_res {
                        Ok(res) => Ok(healer_result(&tool_call.name, res.to_string(), false)),
                        Err(e) => Ok(healer_result(
                            &tool_call.name,
                            format!("MCP Error: {}", e),
                            true,
                        )),
                    };
                }

                // Standard Tool Dispatch
                registry
                    .dispatch(ctx, tool_call.clone(), tool_call.input.clone())
                    .await
            })
        }),
    }
}

pub fn healer(output: String, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output;
    }
    let keep = max_chars / 2;
    let head = &output[..keep];
    let tail = &output[output.len() - keep..];
    let removed = output.len() - max_chars;
    format!(
        "{}\n\n[... THE HEALER: Truncated {} characters to save tokens. ...]\n\n{}",
        head, removed, tail
    )
}

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
    fn test_healer() {
        let input = "hello world".to_string();
        assert_eq!(healer(input.clone(), 50), input);
    }
}
