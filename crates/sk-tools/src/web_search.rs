//! Web search tool.
use sk_types::ToolDefinition;
pub fn web_search_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_search".into(),
        description: "Search the web for information.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        source: "".into(),
        required_capabilities: vec![sk_types::security::Capability::HttpRequest],
    }
}

pub fn handle_web_search(query: &str) -> Result<String, sk_types::SovereignError> {
    Ok(format!(
        "Web search request for '{}' registered (API client not injected into current session).",
        query
    ))
}
