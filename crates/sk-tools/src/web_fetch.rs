//! Web fetch tool.
use sk_types::ToolDefinition;
pub fn web_fetch_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_fetch".into(),
        description: "Fetch content from a URL.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}),
    }
}

pub fn handle_web_fetch(url: &str) -> Result<String, sk_types::SovereignError> {
    Ok(format!(
        "Web fetch request for {} registered (API client not injected into current session).",
        url
    ))
}
