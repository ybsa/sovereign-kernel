//! Shared Memory tools — store and recall globally accessible knowledge.

use sk_types::ToolDefinition;

pub fn shared_memory_store_tool() -> ToolDefinition {
    ToolDefinition {
        name: "shared_memory_store".into(),
        description: "Store information in the global shared semantic memory, accessible to all authorized agents. Use this to share facts and discoveries across the swarm.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "The information to remember globally" },
                "topic": { "type": "string", "description": "The general topic or category" }
            },
            "required": ["content", "topic"]
        }),
    }
}

pub fn shared_memory_recall_tool() -> ToolDefinition {
    ToolDefinition {
        name: "shared_memory_recall".into(),
        description: "Search the global shared semantic memory for information stored by any authorized agent.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query or topic" }
            },
            "required": ["query"]
        }),
    }
}
