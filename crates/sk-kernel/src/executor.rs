use sk_engine::agent_loop::AgentLoopConfig;
use sk_types::{AgentId, ToolCall};
use std::sync::Arc;

use crate::tools::{ToolContext, ToolRegistry};
use crate::SovereignKernel;

/// Detect if a model name suggests a "small" local model (under ~10B parameters).
/// Small models get fewer tools and a tighter context window.
fn is_small_model(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    // Parameter count markers
    let size_markers = ["0.5b", "1b", "1.5b", "2b", "3b", "3.8b", "4b", "7b", "8b", "3.2", "3.1:8b"];
    if size_markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Known small families by name
    let small_families = [
        "llama3.2", "phi", "phi3", "tinyllama", "gemma:2b", "gemma2:2b",
        "qwen2:0.5b", "qwen2:1.5b", "smollm", "stablelm", "orca-mini",
        "neural-chat", "mistral:7b", "deepseek-r1:1.5b", "deepseek-r1:7b",
    ];
    small_families.iter().any(|f| lower.contains(f))
}

/// Detect if a model is running locally (via Ollama or direct inference).
/// Local models often have limited tool-calling support.
fn is_local_model(model_name: &str, provider: &str) -> bool {
    let lower_provider = provider.to_lowercase();
    let lower_model = model_name.to_lowercase();
    lower_provider == "ollama"
        || lower_provider == "local"
        || lower_provider == "local_gpu"
        || lower_model.starts_with("ollama/")
        // Ollama base URL hint embedded in provider name
        || lower_provider.contains("localhost")
        || lower_provider.contains("127.0.0.1")
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
    intent: Option<crate::wizard::AgentIntent>,
    user_query: Option<&str>,
) -> AgentLoopConfig {
    let small_model = is_small_model(&model_name);
    let local_model = is_local_model(&model_name, driver.provider());

    // Local/small models get a tighter rolling window and no tool schemas.
    // Cloud models with full tool-calling support get the full window.
    let context_window_messages: usize = if small_model || local_model { 6 } else { 10 };
    let supports_tool_schemas = !local_model || !small_model;

    // Map CLI capabilities if present
    let mut caps = intent
        .as_ref()
        .map(|i| i.capabilities.clone())
        .unwrap_or_else(|| vec!["file_read".into(), "shell".into(), "skills".into()]);
    if intent.as_ref().map(|i| i.is_otto).unwrap_or(false)
        && !caps.iter().any(|cap| cap.eq_ignore_ascii_case("otto"))
    {
        caps.push("otto".into());
    }

    let mut tools = crate::tools::available_tool_definitions(&caps, small_model, user_query);

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

    drop(cfg_snap);

    // For models without tool-schema support, describe tools as plain text inside
    // the system prompt so the model can still be instructed to use them.
    let final_prompt = if !supports_tool_schemas && !tools.is_empty() {
        let mut tool_lines = String::from("\n\n## Available Actions\nYou can perform these actions by responding with the action name and parameters:\n");
        for t in &tools {
            tool_lines.push_str(&format!("- **{}**: {}\n", t.name, t.description));
        }
        format!("{system_prompt}{tool_lines}")
    } else {
        system_prompt
    };

    let intent_clone = intent.clone();
    let registry = Arc::new(ToolRegistry::new().register_all());
    let intent_ctx = intent_clone.clone();

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
        thinking_mode: false,
        context_window_messages,
        supports_tool_schemas,
        tool_executor: Box::new(move |tool_call: ToolCall| {
            let kernel = k.clone();
            let aid = aid;
            let registry = registry.clone();
            let intent_ctx = intent_ctx.clone();

            Box::pin(async move {
                let (mut default_mode, workspace_dir, exec_policy) = {
                    let config_snap = crate::rlock!(kernel.config);
                    let dm = config_snap.execution_mode;
                    let ws = config_snap.effective_workspaces_dir();
                    let ep = config_snap.exec_policy.clone();
                    (dm, ws, ep)
                };

                if let Some(i) = &intent_ctx {
                    if let Some(m) = &i.mode {
                        if m.eq_ignore_ascii_case("unrestricted") {
                            default_mode = sk_types::config::ExecutionMode::Unrestricted;
                        } else if m.eq_ignore_ascii_case("sandbox") {
                            default_mode = sk_types::config::ExecutionMode::Sandbox;
                        }
                    }
                }

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
