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
        description: "Launch a headless browser (if not already running) and navigate to a URL. Returns page title and readable content.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The absolute URL to visit" }
            },
            "required": ["url"]
        }), // Re-using HttpRequest capability for browsing
    }
}

fn browser_click_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_click".into(),
        description: "Click an element on the current page using a CSS selector or visible text."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector or text content of the element to click" }
            },
            "required": ["selector"]
        }),
    }
}

fn browser_type_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_type".into(),
        description: "Type text into an input field on the current page.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector of the input field" },
                "text": { "type": "string", "description": "Text to type" }
            },
            "required": ["selector", "text"]
        }),
    }
}

fn browser_screenshot_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_screenshot".into(),
        description: "Take a screenshot of the current page. Returns a base64 encoded PNG or URL access string.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn browser_read_page_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_read_page".into(),
        description: "Read the current page content as clean markdown. Useful after clicking or navigating if content updates.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn browser_close_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_close".into(),
        description: "Close the persistent browser session. Use this when finished browsing to free up memory.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }), // No special capabilities needed to clean up own browser
    }
}
