//! Web fetch tool.
use sk_types::ToolDefinition;
pub fn web_fetch_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_fetch".into(),
        description: "Fetch content from a URL.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}),
        source: "builtin".into(),
    }
}
