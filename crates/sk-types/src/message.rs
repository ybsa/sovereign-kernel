//! Message types for LLM conversations.

use serde::{Deserialize, Serialize};

/// Role of a message participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System prompt / instructions.
    System,
    /// Human user input.
    User,
    /// LLM assistant output.
    Assistant,
    /// Tool execution result.
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Who sent this message.
    pub role: Role,
    /// Text content (may be empty for tool-use-only messages).
    #[serde(default)]
    pub content: String,
    /// Tool calls requested by the assistant (only when role = Assistant).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<crate::tool::ToolCall>,
    /// Tool call ID this message is responding to (only when role = Tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Timestamp when this message was created.
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Message {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Estimated token count (rough: chars / 4).
    pub fn estimated_tokens(&self) -> usize {
        self.content.len() / 4
            + self
                .tool_calls
                .iter()
                .map(|tc| tc.arguments.len() / 4 + tc.name.len())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_user() {
        let m = Message::user("hello");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content, "hello");
    }

    #[test]
    fn message_json_roundtrip() {
        let m = Message::assistant("world");
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::Assistant);
        assert_eq!(parsed.content, "world");
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn estimated_tokens_rough() {
        let m = Message::user("a]".repeat(100).as_str());
        assert!(m.estimated_tokens() > 0);
    }
}
