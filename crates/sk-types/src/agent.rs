//! Agent identity and manifest types.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AgentId
// ---------------------------------------------------------------------------

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    /// Create a new random agent ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from a string.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AgentId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

// ---------------------------------------------------------------------------
// AgentManifest
// ---------------------------------------------------------------------------

/// Capability declaration for an agent — what tools it can use, what models
/// it prefers, what permissions it requires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    /// Human-readable agent name.
    pub name: String,
    /// Short description of what this agent does.
    pub description: String,
    /// System prompt (may be overridden by Soul injection).
    pub system_prompt: String,
    /// Preferred LLM provider (e.g. "anthropic", "openai", "local").
    #[serde(default)]
    pub provider: Option<String>,
    /// Preferred model ID (e.g. "claude-sonnet-4-20250514", "gpt-4o").
    #[serde(default)]
    pub model: Option<String>,
    /// Tools this agent is allowed to invoke (empty = all).
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Maximum tokens per response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for LLM sampling.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for AgentManifest {
    fn default() -> Self {
        Self {
            name: "sovereign".into(),
            description: "Default Sovereign Kernel agent".into(),
            system_prompt: String::new(),
            provider: None,
            model: None,
            allowed_tools: Vec::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

// ---------------------------------------------------------------------------
// AgentEntry
// ---------------------------------------------------------------------------

/// Persisted agent record combining identity + manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    /// Unique agent ID.
    pub id: AgentId,
    /// Agent manifest (capabilities, config).
    pub manifest: AgentManifest,
    /// When this agent was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this agent was last active.
    pub last_active: chrono::DateTime<chrono::Utc>,
}

impl AgentEntry {
    /// Create a new agent entry with a fresh ID and current timestamp.
    pub fn new(manifest: AgentManifest) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: AgentId::new(),
            manifest,
            created_at: now,
            last_active: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_roundtrip() {
        let id = AgentId::new();
        let s = id.to_string();
        let parsed = AgentId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn agent_manifest_defaults() {
        let m = AgentManifest::default();
        assert_eq!(m.max_tokens, 4096);
        assert!((m.temperature - 0.7).abs() < f32::EPSILON);
        assert!(m.allowed_tools.is_empty());
    }

    #[test]
    fn agent_entry_json_roundtrip() {
        let entry = AgentEntry::new(AgentManifest::default());
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AgentEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, parsed.id);
    }
}
