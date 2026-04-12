use sk_types::ToolDefinition;

pub fn browser_navigate_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_navigate".into(),
        description: "Navigate to a URL using a full headless browser. Use this for sites that require JavaScript or have complex layouts.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to navigate to." }
            },
            "required": ["url"]
        }),
    }
}

pub fn browser_read_page_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_read_page".into(),
        description: "Read the full text content of a page using a real browser. Ideal for bypassing 'Enable JavaScript' blocks.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to read." }
            },
            "required": ["url"]
        }),
    }
}

pub fn browser_click_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_click".into(),
        description: "Click an element on the current page using a CSS selector.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector to click" }
            },
            "required": ["selector"]
        }),
    }
}

pub fn browser_type_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_type".into(),
        description: "Type text into an input field on the current page.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector of the input" },
                "text": { "type": "string", "description": "Text to type" }
            },
            "required": ["selector", "text"]
        }),
    }
}

pub fn browser_scroll_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_scroll".into(),
        description: "Scroll the current page up or down.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "direction": { "type": "string", "enum": ["up", "down"], "default": "down" }
            }
        }),
    }
}

pub fn browser_get_dom_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_get_dom".into(),
        description:
            "Retrieve a simplified structure (JSON) of the current page's interactive elements."
                .into(),
        input_schema: serde_json::json!({ "type": "object", "properties": {} }),
    }
}

pub fn browser_screenshot_tool() -> ToolDefinition {
    ToolDefinition {
        name: "browser_screenshot".into(),
        description: "Take a full-page screenshot of a URL. Useful for visual verification or data extraction from charts.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to screenshot." },
                "path": { "type": "string", "description": "The path to save the screenshot to (optional)." }
            },
            "required": ["url"]
        }),
    }
}
