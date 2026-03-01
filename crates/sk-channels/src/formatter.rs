//! Channel-specific message formatting.
//!
//! Converts standard Markdown into platform-specific markup:
//! - Telegram HTML: `**bold**` → `<b>bold</b>`
//! - Slack mrkdwn: `**bold**` → `*bold*`, `[text](url)` → `<url|text>`
//! - Plain text: strips all formatting

/// Format a message for a specific channel type. Currently just passes through text.
pub fn format_for_channel(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_markdown_passthrough() {
        let text = "**bold** and *italic*";
        assert_eq!(format_for_channel(text), text);
    }
}
