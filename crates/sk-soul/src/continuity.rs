//! Cross-session continuity — how the agent persists identity across restarts.
//!
//! "Each session, you wake up fresh. These files _are_ your memory."
//! — OpenClaw SOUL.md

/// Continuity strategy for maintaining identity across sessions.
///
/// The Soul defines *who* the agent is, but Continuity defines *how* the agent
/// remembers being that person across restarts and session boundaries.
#[derive(Debug, Clone)]
pub struct ContinuityConfig {
    /// Whether to auto-load memories at session start.
    pub auto_recall: bool,
    /// Maximum number of memory entries to inject into context.
    pub max_context_memories: usize,
    /// Whether the agent can update its own SOUL.md.
    pub allow_soul_mutation: bool,
}

impl Default for ContinuityConfig {
    fn default() -> Self {
        Self {
            auto_recall: true,
            max_context_memories: 20,
            allow_soul_mutation: false,
        }
    }
}

/// A context block injected at session start to provide continuity.
#[derive(Debug, Clone)]
pub struct ContinuityContext {
    /// Relevant memories recalled for this session.
    pub memories: Vec<RecalledMemory>,
    /// Summary of the agent's recent activity.
    pub recent_summary: Option<String>,
}

/// A single recalled memory entry.
#[derive(Debug, Clone)]
pub struct RecalledMemory {
    /// The memory content.
    pub content: String,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f32,
    /// When this memory was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Source of the memory (e.g. "conversation", "user_note", "learned").
    pub source: String,
}

impl ContinuityContext {
    /// Format memories as a prompt fragment for injection.
    pub fn to_prompt_fragment(&self) -> String {
        if self.memories.is_empty() && self.recent_summary.is_none() {
            return String::new();
        }

        let mut parts = vec!["## Recalled Memories".to_string()];

        if let Some(ref summary) = self.recent_summary {
            parts.push(format!("\n**Recent activity:** {summary}\n"));
        }

        for (i, mem) in self.memories.iter().enumerate() {
            parts.push(format!(
                "{}. [{}] (relevance: {:.0}%) {}",
                i + 1,
                mem.source,
                mem.relevance * 100.0,
                mem.content,
            ));
        }

        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_continuity_context() {
        let ctx = ContinuityContext {
            memories: Vec::new(),
            recent_summary: None,
        };
        assert!(ctx.to_prompt_fragment().is_empty());
    }

    #[test]
    fn continuity_with_memories() {
        let ctx = ContinuityContext {
            memories: vec![RecalledMemory {
                content: "User prefers dark mode".into(),
                relevance: 0.95,
                source: "learned".into(),
                created_at: chrono::Utc::now(),
            }],
            recent_summary: Some("Helped user debug a Rust project".into()),
        };
        let fragment = ctx.to_prompt_fragment();
        assert!(fragment.contains("dark mode"));
        assert!(fragment.contains("95%"));
    }
}
