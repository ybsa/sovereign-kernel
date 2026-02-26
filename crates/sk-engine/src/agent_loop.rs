//! Core agent execution loop.
//!
//! Based on OpenFang's agent_loop.rs — receives a message, recalls memories,
//! calls the LLM, executes tool calls, saves the conversation.
//! Stripped of Hands, browser, and Docker logic.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, StopReason};
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
    pub tool_executor: Box<dyn Fn(&ToolCall) -> SovereignResult<String> + Send + Sync>,
}

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
    let mut final_response = String::new();

    // 3. Agent loop: LLM → tool calls → LLM → ... → end turn
    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            warn!("Agent loop hit MAX_ITERATIONS ({MAX_ITERATIONS})");
            break;
        }

        debug!(iteration = iterations, "Agent loop iteration");

        // Call LLM
        let request = CompletionRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            tools: config.tools.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            stream: false,
        };

        let response: CompletionResponse = config.driver.complete(request).await?;
        total_tokens += response.usage.total_tokens;

        // Add assistant message
        let mut assistant_msg = Message::assistant(&response.content);
        assistant_msg.tool_calls = response.tool_calls.clone();
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

                    let result = match (config.tool_executor)(tool_call) {
                        Ok(output) => output,
                        Err(e) => format!("Error: {e}"),
                    };

                    let tool_msg = Message::tool_result(&tool_call.id, &result);
                    messages.push(tool_msg.clone());
                    session.push_message(tool_msg);
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
