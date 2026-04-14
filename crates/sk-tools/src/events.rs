//! Event publishing tool definition.

use sk_types::ToolDefinition;

pub fn event_publish_tool() -> ToolDefinition {
    ToolDefinition {
        name: "event_publish".into(),
        description: "Publish an event to the kernel event bus so other agents or the system can react to it.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "event_type": { "type": "string", "description": "Event category (e.g. 'alert', 'data_ready', 'threshold_exceeded')." },
                "payload": { "type": "object", "description": "Event data as key-value pairs.", "default": {} }
            },
            "required": ["event_type"]
        }),
    }
}

pub fn process_list_tool() -> ToolDefinition {
    ToolDefinition {
        name: "process_list".into(),
        description: "List currently running OS processes with their PID, name, and CPU/memory usage.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "filter": { "type": "string", "description": "Optional name filter (substring match). Omit to list all." }
            }
        }),
    }
}
