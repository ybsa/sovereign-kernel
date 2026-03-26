//! Memory tools — remember, recall, forget.

use sk_memory::MemorySubstrate;
use sk_types::{AgentId, SovereignResult, ToolDefinition};
use uuid::Uuid;

pub fn remember_tool() -> ToolDefinition {
    ToolDefinition {
        name: "remember".into(),
        description: "Store information in long-term memory for future recall.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "The information to remember" },
                "source": { "type": "string", "description": "Source/category (e.g. 'user_preference', 'learned', 'conversation')" }
            },
            "required": ["content"]
        }),
    }
}

pub fn recall_tool() -> ToolDefinition {
    ToolDefinition {
        name: "recall".into(),
        description: "Search long-term memory for relevant information.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "limit": { "type": "integer", "description": "Max results (default 5)" }
            },
            "required": ["query"]
        }),
    }
}

pub fn forget_tool() -> ToolDefinition {
    ToolDefinition {
        name: "forget".into(),
        description: "Remove a specific memory by ID.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "memory_id": { "type": "string", "description": "ID of the memory to forget" }
            },
            "required": ["memory_id"]
        }),
    }
}

pub fn handle_remember(
    substrate: &MemorySubstrate,
    agent_id: AgentId,
    content: &str,
) -> SovereignResult<String> {
    // For now we use a newly generated Uuid for BM25 since exact semantic embeddings are missing
    let memory_id = Uuid::new_v4().to_string();
    substrate.bm25.index(agent_id, &memory_id, content)?;
    Ok(format!("Successfully remembered with ID: {memory_id}"))
}

pub fn handle_recall(
    substrate: &MemorySubstrate,
    agent_id: AgentId,
    query: &str,
    limit: usize,
) -> SovereignResult<String> {
    let results = substrate.bm25.search(Some(agent_id), query, limit)?;
    if results.is_empty() {
        return Ok("No relevant memories found.".into());
    }

    let mut output = String::from("Recalled memories:\n");
    for (i, res) in results.iter().enumerate() {
        output.push_str(&format!(
            "{}. [ID: {}] {}\n",
            i + 1,
            res.memory_id,
            res.content
        ));
    }
    Ok(output)
}

pub fn handle_forget(substrate: &MemorySubstrate, memory_id: &str) -> SovereignResult<String> {
    substrate.bm25.remove(memory_id)?;
    Ok(format!("Forgot memory ID: {memory_id}"))
}
