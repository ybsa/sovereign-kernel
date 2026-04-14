use async_trait::async_trait;
use sk_engine::agent_loop::{run_agent_loop, AgentLoopConfig};
use sk_engine::llm_driver::{
    CompletionRequest, CompletionResponse, LlmDriver, LlmError, StopReason, TokenUsage,
};
use sk_engine::runtime::subprocess_sandbox::validate_command_allowlist;
use sk_types::config::{ExecPolicy, ExecSecurityMode};
use sk_types::{AgentId, Session, SovereignResult, ToolCall, ToolDefinition, ToolResult};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// A handler that executes a specific tool call.
type ToolHandler = Arc<
    dyn Fn(ToolCall) -> Pin<Box<dyn Future<Output = SovereignResult<ToolResult>> + Send>>
        + Send
        + Sync,
>;

/// Local tool registry for testing.
#[derive(Clone)]
struct ToolRegistry {
    handlers: HashMap<String, ToolHandler>,
    definitions: Vec<ToolDefinition>,
}

impl ToolRegistry {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            definitions: Vec::new(),
        }
    }

    fn register<F>(&mut self, definition: ToolDefinition, handler: F)
    where
        F: Fn(ToolCall) -> Pin<Box<dyn Future<Output = SovereignResult<ToolResult>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.handlers
            .insert(definition.name.clone(), Arc::new(handler));
        self.definitions.push(definition);
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    fn executor(&self) -> sk_engine::agent_loop::ToolExecutor {
        let registry = self.clone();
        Box::new(move |call| {
            let registry = registry.clone();
            Box::pin(async move {
                if let Some(handler) = registry.handlers.get(&call.name) {
                    handler(call).await
                } else {
                    Err(sk_types::SovereignError::ToolExecutionError(format!(
                        "Unknown tool: {}",
                        call.name
                    )))
                }
            })
        })
    }
}

/// A Mock LLM Driver that returns predefined responses sequentially.
struct MockLlmDriver {
    pub responses: Mutex<VecDeque<CompletionResponse>>,
}

impl MockLlmDriver {
    fn new(responses: Vec<CompletionResponse>) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from(responses)),
        }
    }
}

#[async_trait]
impl LlmDriver for MockLlmDriver {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut queue = self.responses.lock().unwrap();
        if let Some(resp) = queue.pop_front() {
            Ok(resp)
        } else {
            Ok(CompletionResponse {
                content: "End of mock sequence".into(),
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
            })
        }
    }

    fn provider(&self) -> &str {
        "mock"
    }
}

// ---------------------------------------------------------------------------
// TEST 1: End-to-End Tool Execution (Agent calls File/Shell, loop completes)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_agent_successful_tool_call() {
    let mut registry = ToolRegistry::new();

    // Register a mock file_read tool
    registry.register(
        ToolDefinition {
            name: "mock_file_read".into(),
            description: "Reads a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
        },
        |call| {
            Box::pin(async move {
                Ok(ToolResult {
                    tool_use_id: call.id.clone(),
                    content: "File contents: Hello World!".into(),
                    is_error: false,
                    signal: None,
                })
            })
        },
    );

    let driver = Arc::new(MockLlmDriver::new(vec![
        // Turn 1: LLM decides to call mock_file_read
        CompletionResponse {
            content: "I will read the file.".into(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "mock_file_read".into(),
                input: serde_json::json!({"path": "hello.txt"}),
            }],
            stop_reason: StopReason::ToolUse,
            usage: TokenUsage::default(),
        },
        // Turn 2: LLM receives tool result and finishes
        CompletionResponse {
            content: "The file says: Hello World!".into(),
            tool_calls: vec![],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage::default(),
        },
    ]));

    let config = AgentLoopConfig {
        driver,
        system_prompt: "You are a test agent".into(),
        tools: registry.definitions(),
        model: "mock-model".into(),
        max_tokens: 1000,
        temperature: 0.0,
        tool_executor: registry.executor(),
        max_iterations_per_task: 5,
        max_tokens_per_task: 5000,
        step_dump_enabled: false,
        forensics_root: std::env::temp_dir(),
        stream_handler: None,
        on_usage: None,
        checkpoint_handler: None,
        thinking_mode: false,
        context_window_messages: 10,
        supports_tool_schemas: true,
    };

    let mut session = Session::new(AgentId::new());
    let result = run_agent_loop(config, &mut session, "Read hello.txt")
        .await
        .unwrap();

    // Verify LLM final response
    assert_eq!(result.response, "The file says: Hello World!");
    assert_eq!(result.iterations, 2);
    assert_eq!(result.tool_calls_made, 1);
}

// ---------------------------------------------------------------------------
// TEST 2: Security Sandbox Hard Block (Agent calls dangerous shell command)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_agent_security_sandbox_blocked_command() {
    let mut registry = ToolRegistry::new();

    // The Sandbox execution policy restricted to only safe bins
    let policy = ExecPolicy {
        mode: ExecSecurityMode::Allowlist,
        safe_bins: vec!["ls".into(), "echo".into()],
        allowed_commands: vec![],
        blocked_args: vec!["-c".into(), "/C".into()],
        timeout_secs: 30,
        max_output_bytes: 10240,
        no_output_timeout_secs: 30,
    };

    // Register a shell tool that honors the Sandbox validate_command_allowlist
    registry.register(
        ToolDefinition {
            name: "shell_exec".into(),
            description: "Executes a shell command".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                }
            }),
        },
        move |call| {
            let policy = policy.clone();
            Box::pin(async move {
                let command = call.input["command"].as_str().unwrap_or("");

                // Invoke actual Sandbox validation
                if let Err(e) = validate_command_allowlist(command, &policy) {
                    return Ok(ToolResult {
                        tool_use_id: call.id.clone(),
                        content: format!("SECURITY VIOLATION: {}", e),
                        is_error: true,
                        signal: None,
                    });
                }

                Ok(ToolResult {
                    tool_use_id: call.id.clone(),
                    content: "Command executed successfully.".into(),
                    is_error: false,
                    signal: None,
                })
            })
        },
    );

    let driver = Arc::new(MockLlmDriver::new(vec![
        // Turn 1: LLM decides to call shell_exec with a dangerous argument
        CompletionResponse {
            content: "I will run a command.".into(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "shell_exec".into(),
                input: serde_json::json!({"command": "echo /C rm -rf /"}), // /C should trigger blocked_args
            }],
            stop_reason: StopReason::ToolUse,
            usage: TokenUsage::default(),
        },
        // Turn 2: LLM receives security violation and apologizes
        CompletionResponse {
            content: "I am not allowed to run that.".into(),
            tool_calls: vec![],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage::default(),
        },
    ]));

    let config = AgentLoopConfig {
        driver,
        system_prompt: "You are a test agent".into(),
        tools: registry.definitions(),
        model: "mock-model".into(),
        max_tokens: 1000,
        temperature: 0.0,
        tool_executor: registry.executor(),
        max_iterations_per_task: 5,
        max_tokens_per_task: 5000,
        step_dump_enabled: false,
        forensics_root: std::env::temp_dir(),
        stream_handler: None,
        on_usage: None,
        checkpoint_handler: None,
        thinking_mode: false,
        context_window_messages: 10,
        supports_tool_schemas: true,
    };

    let mut session = Session::new(AgentId::new());
    let result = run_agent_loop(config, &mut session, "Run dangerous command")
        .await
        .unwrap();

    // Check if the LLM acknowledged the security block
    assert_eq!(result.response, "I am not allowed to run that.");
    assert_eq!(result.tool_calls_made, 1);

    // Check that the session history actually contains the SECURITY VIOLATION tool output
    let tool_output = session.messages.iter().find(|m| {
        if m.role == sk_types::Role::User {
            if let sk_types::message::MessageContent::Blocks(blocks) = &m.content {
                for b in blocks {
                    if let sk_types::message::ContentBlock::ToolResult { content, .. } = b {
                        if content.text_content().contains("SECURITY VIOLATION") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    });

    assert!(
        tool_output.is_some(),
        "The security sandbox did not intercept the tool call correctly."
    );
}
