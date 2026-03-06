//! Web fetch tool.
use sk_types::ToolDefinition;
use std::time::Duration;

pub fn web_fetch_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_fetch".into(),
        description: "Fetch content from a URL. Returns a clean markdown representation of the page, truncated to safe context limits.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}),
    }
}

pub async fn handle_web_fetch(url: &str) -> Result<String, sk_types::SovereignError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to build HTTP client: {}",
                e
            ))
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("HTTP request failed: {}", e))
    })?;

    let status = response.status();
    let text = response.text().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to read response body: {}", e))
    })?;

    if !status.is_success() {
        let snippet = text.chars().take(500).collect::<String>();
        return Ok(format!("HTTP Error {}: \n{}", status, snippet));
    }

    let clean_text = clean_html(&text);

    // Truncate to save context window (roughly 4000 chars)
    let max_len = 4000;
    let mut truncated: String = clean_text.chars().take(max_len).collect();
    if clean_text.chars().count() > max_len {
        truncated.push_str("\n...[Content truncated for length. If you need more, try using the browser_navigate tool instead.]");
    }

    Ok(truncated)
}

fn clean_html(html: &str) -> String {
    use regex_lite::Regex;
    // Remove script tags and content
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let mut html_clean = re_script.replace_all(html, "").into_owned();

    // Remove style tags and content
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    html_clean = re_style.replace_all(&html_clean, "").into_owned();

    // Extract title (cheaply)
    let re_title = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    let title = if let Some(caps) = re_title.captures(&html_clean) {
        caps.get(1)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "No Title".to_string())
    } else {
        "No Title".to_string()
    };

    // Remove all HTML tags
    let re_tags = Regex::new(r"(?is)<[^>]+>").unwrap();
    let mut text = re_tags.replace_all(&html_clean, " ").into_owned();

    // Collapse whitespace horizontally
    let re_ws = Regex::new(r"[\t ]+").unwrap();
    text = re_ws.replace_all(&text, " ").into_owned();

    // Collapse multiple newlines
    let re_newlines = Regex::new(r"(?m)^\s+").unwrap();
    text = re_newlines.replace_all(&text, "").into_owned();
    let re_newlines_2 = Regex::new(r"\n{3,}").unwrap();
    text = re_newlines_2.replace_all(&text, "\n\n").into_owned();

    format!("# {}\n\n{}", title.trim(), text.trim())
}
