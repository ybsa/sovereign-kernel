//! Browser automation tools.

use sk_types::security::Capability;
use sk_types::ToolDefinition;

/// Returns the suite of browser automation tools.
pub fn browser_tools() -> Vec<ToolDefinition> {
    vec![
        browser_navigate_tool(),
        browser_click_tool(),
        browser_type_tool(),
        browser_screenshot_tool(),
        browser_read_page_tool(),
        browser_close_tool(),
    ]
}

fn browser_navigate_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_navigate".into(),
        source: "".into(),
        description: "Launch a headless browser (if not already running) and navigate to a URL. Returns page title and readable content.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The absolute URL to visit" }
            },
            "required": ["url"]
        }),
        required_capabilities: vec![Capability::HttpRequest], // Re-using HttpRequest capability for browsing
    }
}

fn browser_click_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_click".into(),
        source: "".into(),
        description: "Click an element on the current page using a CSS selector or visible text."
            .into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector or text content of the element to click" }
            },
            "required": ["selector"]
        }),
        required_capabilities: vec![Capability::HttpRequest],
    }
}

fn browser_type_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_type".into(),
        source: "".into(),
        description: "Type text into an input field on the current page.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector of the input field" },
                "text": { "type": "string", "description": "Text to type" }
            },
            "required": ["selector", "text"]
        }),
        required_capabilities: vec![Capability::HttpRequest],
    }
}

fn browser_screenshot_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_screenshot".into(),
        source: "".into(),
        description: "Take a screenshot of the current page. Returns a base64 encoded PNG or URL access string.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
        required_capabilities: vec![Capability::HttpRequest],
    }
}

fn browser_read_page_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_read_page".into(),
        source: "".into(),
        description: "Read the current page content as clean markdown. Useful after clicking or navigating if content updates.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
        required_capabilities: vec![Capability::HttpRequest],
    }
}

fn browser_close_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_close".into(),
        source: "".into(),
        description: "Close the persistent browser session. Use this when finished browsing to free up memory.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
        required_capabilities: vec![], // No special capabilities needed to clean up own browser
    }
}
