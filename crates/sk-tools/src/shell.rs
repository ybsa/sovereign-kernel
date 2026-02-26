//! Shell command execution tool.
use sk_types::ToolDefinition;
pub fn shell_exec_tool() -> ToolDefinition {
    ToolDefinition { name: "shell_exec".into(), description: "Execute a shell command.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}), source: "builtin".into() }
}
