//! Knowledge graph tool definitions.

use sk_types::ToolDefinition;

pub fn knowledge_add_entity_tool() -> ToolDefinition {
    ToolDefinition {
        name: "knowledge_add_entity".into(),
        description: "Add an entity to the knowledge graph (person, company, concept, place, etc.) with optional properties.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Entity name (e.g. 'Acme Corp', 'John Smith')." },
                "entity_type": { "type": "string", "description": "Type label (e.g. 'company', 'person', 'concept', 'place')." },
                "properties": { "type": "object", "description": "Optional key-value properties (e.g. {\"website\": \"acme.com\"}).", "default": {} }
            },
            "required": ["name", "entity_type"]
        }),
    }
}

pub fn knowledge_add_relation_tool() -> ToolDefinition {
    ToolDefinition {
        name: "knowledge_add_relation".into(),
        description: "Add a directed relation between two entities in the knowledge graph (e.g. 'Acme Corp' --[employs]--> 'John Smith').".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "from_entity": { "type": "string", "description": "Name of the source entity." },
                "relation": { "type": "string", "description": "Relation label (e.g. 'employs', 'competes_with', 'part_of')." },
                "to_entity": { "type": "string", "description": "Name of the target entity." },
                "weight": { "type": "number", "description": "Relation strength 0.0–1.0 (default 1.0).", "default": 1.0 }
            },
            "required": ["from_entity", "relation", "to_entity"]
        }),
    }
}

pub fn knowledge_query_tool() -> ToolDefinition {
    ToolDefinition {
        name: "knowledge_query".into(),
        description: "Search the knowledge graph for entities and their relations matching a name pattern.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Name pattern to search for (substring match)." },
                "limit": { "type": "integer", "description": "Max results to return (default 10).", "default": 10 }
            },
            "required": ["query"]
        }),
    }
}
