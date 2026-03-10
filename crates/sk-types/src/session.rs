//! Session types for conversation persistence.

use crate::agent::AgentId;
use crate::message::Message;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A conversation session between a user and an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: SessionId,
    /// The agent this session belongs to.
    pub agent_id: AgentId,
    /// Ordered message history.
    pub messages: Vec<Message>,
    /// Optional human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional summary of the conversation history (populated by THE HEALER).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// When this session was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this session was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    /// Create a new empty session for an agent.
    pub fn new(agent_id: AgentId) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: SessionId::new(),
            agent_id,
            messages: Vec::new(),
            label: None,
            summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the session history.
    pub fn push_message(&mut self, message: Message) {
        self.updated_at = chrono::Utc::now();
        self.messages.push(message);
    }

    /// Estimated total tokens across all messages.
    pub fn total_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.estimated_tokens()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_push_updates_timestamp() {
        let mut s = Session::new(AgentId::new());
        let before = s.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        s.push_message(Message::user("hello"));
        assert!(s.updated_at >= before);
        assert_eq!(s.messages.len(), 1);
    }

    #[test]
    fn session_id_roundtrip() {
        let id = SessionId::new();
        let s = id.to_string();
        let parsed = SessionId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }
}
