//! Core agent execution loop.
//!
//! Based on Sovereign Kernel's agent_loop.rs — receives a message, recalls memories,
//! calls the LLM, executes tool calls, saves the conversation.
//! Stripped of Hands, browser, and Docker logic.

use crate::llm_driver::{CompletionRequest, LlmDriver, StopReason};
use sk_types::{Message, Session, SovereignResult, ToolCall, ToolDefinition};
use tracing::{debug, info, warn};

/// Maximum iterations in the agent loop before giving up.
const MAX_ITERATIONS: u32 = 50;

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

pub struct AgentLoopConfig<'a> {
    /// The LLM driver to use.
    pub driver: &'a dyn LlmDriver,
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
    /// Optional streaming callback to receive tokens as they are generated.
    pub stream_handler: Option<StreamHandler>,
}

use crate::loop_guard::LoopGuard;
use crate::retry;
use tokio::time::sleep;

/// Run the agent execution loop for a single user message.
///
/// This is the core of the Sovereign Kernel: load context → recall memories →
/// LLM → tool calls → save. Loops until the model ends its turn or we hit MAX_ITERATIONS.
pub async fn run_agent_loop(
    config: AgentLoopConfig<'_>,
    session: &mut Session,
    user_message: &str,
) -> SovereignResult<AgentLoopResult> {
    // 1. Add user message to session
    session.push_message(Message::user(user_message));

    // 2. Build initial messages array with system prompt
    let mut messages = vec![Message::system(&config.system_prompt)];
    messages.extend(session.messages.iter().cloned());

    let mut total_tokens = 0u32;
    let mut tool_calls_made = 0u32;
    let mut iterations = 0u32;
    let mut consecutive_errors = 0u32;
    let mut final_response = String::new();

    let mut loop_guard = LoopGuard::new();

    // 3. Agent loop: LLM → tool calls → LLM → ... → end turn
    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            warn!("Agent loop hit MAX_ITERATIONS ({MAX_ITERATIONS})");
            messages.push(Message::system(
                "System Error: Maximum reasoning steps exceeded.",
            ));
            break;
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

        // Context budget check
        if !crate::context_budget::fits_in_context(
            total_tokens as usize,
            config.max_tokens as usize * 10,
        ) {
            warn!("Approaching context window limit: {} tokens", total_tokens);
            // In a full implementation, we would compact here
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

    Ok(AgentLoopResult {
        response: final_response,
        session: session.clone(),
        total_tokens,
        tool_calls_made,
        iterations,
    })
}
