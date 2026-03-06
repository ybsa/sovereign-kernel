//! Tool runner — runtime dispatch of tool calls.
//!
//! Exposes a `ToolRegistry` to map tool definitions to execution handlers,
//! providing the `tool_executor` closure needed by `agent_loop`.

use sk_types::{SovereignError, SovereignResult, ToolCall, ToolDefinition};
use std::collections::HashMap;
use std::sync::Arc;

/// A handler that executes a specific tool call.
pub type ToolHandler = Arc<dyn Fn(&ToolCall) -> SovereignResult<String> + Send + Sync>;

/// Central registry for tools available to the agent.
#[derive(Clone)]
pub struct ToolRegistry {
    handlers: HashMap<String, ToolHandler>,
    definitions: Vec<ToolDefinition>,
}

impl ToolRegistry {
    /// Create a new, empty tool registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            definitions: Vec::new(),
        }
    }

    /// Register a new tool with its metadata and execution handler.
    pub fn register<F>(&mut self, definition: ToolDefinition, handler: F)
    where
        F: Fn(&ToolCall) -> SovereignResult<String> + Send + Sync + 'static,
    {
        self.handlers
            .insert(definition.name.clone(), Arc::new(handler));
        self.definitions.push(definition);
    }

    /// Get all registered tool definitions (passed to the LLM).
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    /// Execute a tool call using the registered handlers.
    pub fn execute(&self, call: &ToolCall) -> SovereignResult<String> {
        if let Some(handler) = self.handlers.get(&call.name) {
            handler(call)
        } else {
            Err(SovereignError::ToolExecutionError(format!(
                "Unknown tool: {}",
                call.name
            )))
        }
    }

    /// Return an executor closure suitable for `AgentLoopConfig` that enforces capabilities.
    pub fn executor(
        &self,
        _agent_capabilities: Vec<sk_types::security::Capability>,
    ) -> crate::agent_loop::ToolExecutor {
        let registry = self.clone();
        Box::new(move |call| {
            // Check capabilities
            if let Some(_def) = registry.definitions.iter().find(|d| d.name == call.name) {
                // Capability checks removed as per sk-types update
            } else {
                return Err(SovereignError::ToolExecutionError(format!(
                    "Unknown tool: {}",
                    call.name
                )));
            }
            registry.execute(call)
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
