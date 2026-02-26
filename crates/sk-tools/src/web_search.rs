//! Web search tool.
use sk_types::ToolDefinition;
pub fn web_search_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_search".into(),
        description: "Search the web for information.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        source: "builtin".into(),
    }
}
