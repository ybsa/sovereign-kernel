use sk_types::{ToolDefinition, ToolResult, ToolSignal};

/// Create the village_forge tool definition.
pub fn village_forge_tool() -> ToolDefinition {
    ToolDefinition {
        name: "village_forge".to_string(),
        description: "The Builder (The Architect) uses this to forge a NEW permanent capability (Hand) when the active workforce is missing one. Specify exactly what you need (e.g. 'I need to control WhatsApp to send messages').".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "task_description": {
                    "type": "string",
                    "description": "Natural language description of the capability or Hand to forge."
                }
            },
            "required": ["task_description"]
        }),
    }
}

/// Handle the village_forge tool execution.
pub fn handle_village_forge(tool_use_id: &str, task_description: &str) -> ToolResult {
    ToolResult {
        tool_use_id: tool_use_id.to_string(),
        content: format!("Requesting the Builder to forge a new capability for: {}", task_description),
        is_error: false,
        signal: Some(ToolSignal::VillageForge {
            task_description: task_description.to_string(),
        }),
    }
}
