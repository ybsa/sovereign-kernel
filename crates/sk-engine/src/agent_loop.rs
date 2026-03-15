//! Core agent execution loop.
//!
//! Based on Sovereign Kernel's agent_loop.rs — receives a message, recalls memories,
//! calls the LLM, executes tool calls, saves the conversation.
//! Stripped of Hands, browser, and Docker logic.

use crate::llm_driver::{CompletionRequest, LlmDriver, StopReason};
use sk_types::{Message, Session, SovereignResult, ToolCall, ToolDefinition};
use tracing::{debug, info, warn};



/// Result of an agent loop execution.
#[derive(Debug)]
pub struct AgentLoopResult {
    /// Final assistant response text.
    pub response: String,
    /// Updated session with full message history.
    pub session: Session,
    /// Total tokens used across all LLM calls.
    pub total_tokens: u32,
    /// Number of tool calls made.
    pub tool_calls_made: u32,
    /// Number of LLM iterations.
    pub iterations: u32,
}

/// Configuration for an agent loop run.
pub type ToolExecutor = Box<dyn Fn(&ToolCall) -> SovereignResult<String> + Send + Sync>;
pub type StreamHandler = Box<dyn Fn(&str) + Send + Sync>;

use std::sync::Arc;

pub struct AgentLoopConfig {
    /// The LLM driver to use.
    pub driver: Arc<dyn LlmDriver + Send + Sync>,
    /// System prompt (including Soul injection).
    pub system_prompt: String,
    /// Available tools (builtin + MCP).
    pub tools: Vec<ToolDefinition>,
    /// Model to use.
    pub model: String,
    /// Maximum tokens per response.
    pub max_tokens: u32,
    /// Temperature.
    pub temperature: f32,
    /// Tool executor callback.
    pub tool_executor: ToolExecutor,
    /// Maximum number of iterations (LLM calls) per task. Prevents runaway loops.
    pub max_iterations_per_task: u32,
    /// Maximum total tokens (input + output) per task. Prevents runaway costs.
    pub max_tokens_per_task: u32,
    /// Whether to enable forensic step dumping to disk (.steps/ folder).
    pub step_dump_enabled: bool,
    /// Root directory for forensics dumps.
    pub forensics_root: std::path::PathBuf,
    /// Optional streaming callback to receive tokens as they are generated.
    pub stream_handler: Option<StreamHandler>,
    /// Optional callback to report token usage per iteration for global budgeting.
    pub on_usage: Option<Box<dyn Fn(crate::llm_driver::TokenUsage) -> SovereignResult<()> + Send + Sync>>,
    /// Optional callback to save state checkpoints for recovery.
    pub checkpoint_handler: Option<Box<dyn Fn(&Session) -> SovereignResult<()> + Send + Sync>>,
}

use crate::loop_guard::LoopGuard;
use crate::retry;
use tokio::time::sleep;

/// Run the agent execution loop for a single user message.
///
/// This is the core of the Sovereign Kernel: load context → recall memories →
/// LLM → tool calls → save. Loops until the model ends its turn or we hit MAX_ITERATIONS.
pub async fn run_agent_loop(
    config: AgentLoopConfig,
    session: &mut Session,
    user_message: &str,
) -> SovereignResult<AgentLoopResult> {
    let forensics = if config.step_dump_enabled {
        Some(crate::forensics::StepForensics::new(
            &config.forensics_root,
            &session.id.to_string(),
        ))
    } else {
        None
    };

    // 1. Add user message to session
    session.push_message(Message::user(user_message));

    // 2. Build initial messages array with system prompt
    let mut messages = vec![Message::system(&config.system_prompt)];
    messages.extend(session.messages.iter().cloned());

    let mut total_tokens = 0u32;
    let mut tool_calls_made = 0u32;
    let mut iterations = 0u32;
    let mut consecutive_errors = 0u32;
    let final_response;

    let mut loop_guard = LoopGuard::new();
    let mut last_checkpoint = std::time::Instant::now();

    // 3. Agent loop: LLM → tool calls → LLM → ... → end turn
    loop {
        // Trigger periodic checkpoint if requested
        if let Some(ref handler) = config.checkpoint_handler {
            if last_checkpoint.elapsed().as_secs() >= 30 {
                if let Err(e) = handler(session) {
                    warn!("Failed to save periodic checkpoint: {}", e);
                } else {
                    debug!("Saved periodic state checkpoint");
                    last_checkpoint = std::time::Instant::now();
                }
            }
        }

        iterations += 1;
        if iterations > config.max_iterations_per_task {
            warn!(
                "Agent loop hit max_iterations_per_task ({})",
                config.max_iterations_per_task
            );
            return Err(sk_types::SovereignError::LoopLimitExceeded {
                reason: "Maximum iterations exceeded".to_string(),
                current: iterations as u64,
                limit: config.max_iterations_per_task as u64,
            });
        }

        debug!(iteration = iterations, "Agent loop iteration");

        // Call LLM with retry backoff
        let mut attempt = 0;
        let response = loop {
            let request = CompletionRequest {
                model: config.model.clone(),
                messages: messages.clone(),
                tools: config.tools.clone(),
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                stream: false,
            };

            match config.driver.complete(request).await {
                Ok(resp) => break resp,
                Err(e) if e.is_retryable() || attempt < retry::max_retries() => {
                    attempt += 1;
                    if attempt > retry::max_retries() {
                        return Err(e.into());
                    }
                    let delay = retry::backoff_delay(attempt);
                    warn!(
                        "LLM error: {e}. Retrying ({attempt}/{}) in {:?}",
                        retry::max_retries(),
                        delay
                    );
                    sleep(delay).await;
                }
                Err(e) => return Err(e.into()),
            }
        };

        total_tokens += response.usage.total_tokens;

        if let Some(ref handler) = config.on_usage {
            handler(response.usage.clone())?;
        }

        if let Some(ref f) = forensics {
            let _ = f.dump_step(
                iterations,
                &messages,
                &response.content,
                &response.tool_calls,
                response.usage.clone(),
            );
        }

        if total_tokens > config.max_tokens_per_task {
            warn!(
                "Agent loop hit max_tokens_per_task ({})",
                config.max_tokens_per_task
            );
            return Err(sk_types::SovereignError::LoopLimitExceeded {
                reason: "Maximum tokens exceeded".to_string(),
                current: total_tokens as u64,
                limit: config.max_tokens_per_task as u64,
            });
        }

        // Context budget check: Trigger THE HEALER (Compaction) if we exceed 80% of budget
        if !crate::context_budget::fits_in_context(
            total_tokens as usize,
            (config.max_tokens as usize * 10) * 8 / 10, // 80% of budget
        ) {
            info!("Context pressure detected. Triggering THE HEALER for session compaction...");

            let compaction_config = crate::compactor::CompactionConfig::default();
            let drv = config.driver.clone();
            let model = config.model.clone();

            // Run compaction
            match crate::compactor::compact_session(drv, &model, session, &compaction_config).await
            {
                Ok(result) => {
                    info!(
                        compacted = result.compacted_count,
                        "THE HEALER has successfully healed the session."
                    );

                    // Update session with compacted state
                    session.messages = result.kept_messages;
                    session.summary = Some(result.summary);

                    // Recalculate total_tokens (approximate)
                    total_tokens = session
                        .summary
                        .as_ref()
                        .map(|s| s.len() as u32 / 4)
                        .unwrap_or(0);
                    for m in &session.messages {
                        total_tokens += m.content.text_length() as u32 / 4;
                    }
                }
                Err(e) => {
                    warn!("THE HEALER failed to heal the session: {}", e);
                }
            }
        }

        // Empty response guard validation
        if response.content.trim().is_empty() && response.tool_calls.is_empty() {
            warn!("LLM returned empty output without tool calls. Nudging.");
            messages.push(Message::user("Please provide a response or call a tool."));
            continue;
        }

        // Output streaming
        if let Some(handler) = &config.stream_handler {
            if !response.content.is_empty() {
                handler(&response.content);
            }
        }

        // Add assistant message
        let assistant_msg = if response.tool_calls.is_empty() {
            Message::assistant(&response.content)
        } else {
            let mut blocks = Vec::new();
            if !response.content.is_empty() {
                blocks.push(sk_types::message::ContentBlock::Text {
                    text: response.content.clone(),
                });
            }
            for tc in &response.tool_calls {
                blocks.push(sk_types::message::ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.input.clone(),
                });
            }
            Message {
                role: sk_types::message::Role::Assistant,
                content: sk_types::message::MessageContent::Blocks(blocks),
            }
        };
        messages.push(assistant_msg.clone());
        session.push_message(assistant_msg);

        // Check stop reason
        match response.stop_reason {
            StopReason::EndTurn | StopReason::MaxTokens | StopReason::ContentFilter => {
                final_response = response.content;
                break;
            }
            StopReason::ToolUse => {
                // Execute tool calls
                for tool_call in &response.tool_calls {
                    tool_calls_made += 1;
                    debug!(tool = %tool_call.name, "Executing tool call");

                    // 1. Check loop guard
                    let args_str = serde_json::to_string(&tool_call.input).unwrap_or_default();
                    if loop_guard.check(&tool_call.name, &args_str) {
                        warn!(tool = %tool_call.name, "Loop detected by guard!");
                        let tool_msg = sk_types::message::Message {
                            role: sk_types::message::Role::User,
                            content: sk_types::message::MessageContent::Blocks(vec![
                                sk_types::message::ContentBlock::ToolResult {
                                    tool_use_id: tool_call.id.clone(),
                                    content: "System Error: Infinite loop detected. You are calling the same tool with the same arguments repeatedly. Please try a completely different approach.".to_string(),
                                    is_error: true,
                                }
                            ]),
                        };
                        messages.push(tool_msg.clone());
                        session.push_message(tool_msg);
                        continue;
                    }

                    // 2. Execute tool
                    let result = match (config.tool_executor)(tool_call) {
                        Ok(output) => {
                            consecutive_errors = 0;
                            if output.is_empty() {
                                "Success (No output returned)".to_string()
                            } else {
                                output
                            }
                        }
                        Err(e) => {
                            consecutive_errors += 1;
                            format!("Error executing tool: {e}")
                        }
                    };

                    let tool_msg = sk_types::message::Message {
                        role: sk_types::message::Role::User,
                        content: sk_types::message::MessageContent::Blocks(vec![
                            sk_types::message::ContentBlock::ToolResult {
                                tool_use_id: tool_call.id.clone(),
                                content: result,
                                is_error: consecutive_errors > 0, // Mark as error block if it was an error
                            },
                        ]),
                    };
                    messages.push(tool_msg.clone());
                    session.push_message(tool_msg);

                    // 3. Circuit Breaker
                    if consecutive_errors >= 5 {
                        warn!("Circuit breaker tripped: 5 consecutive tool execution errors.");
                        final_response = "System Error: Circuit breaker tripped due to 5 consecutive tool failures. The system has automatically halted to prevent infinite error loops.".to_string();
                        return Ok(AgentLoopResult {
                            response: final_response,
                            session: session.clone(),
                            total_tokens,
                            tool_calls_made,
                            iterations,
                        });
                    }
                }
                // Loop back to LLM with tool results
            }
            StopReason::Unknown(ref reason) => {
                warn!(reason = %reason, "Unknown stop reason");
                final_response = response.content;
                break;
            }
        }
    }

    info!(
        iterations,
        tool_calls = tool_calls_made,
        tokens = total_tokens,
        "Agent loop completed"
    );

    if let Some(ref f) = forensics {
        let status = if final_response.starts_with("System Error") {
            "error"
        } else {
            "success"
        };
        let _ = f.dump_summary(total_tokens, iterations, status);
    }

    Ok(AgentLoopResult {
        response: final_response,
        session: session.clone(),
        total_tokens,
        tool_calls_made,
        iterations,
    })
}
