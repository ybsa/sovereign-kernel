//! Context budget — manage context window token limits and pressure.

use sk_types::Message;

pub const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// Check if a token count fits within a limit.
pub fn fits_in_context(token_count: usize, limit: usize) -> bool {
    token_count <= limit
}

/// A budget for a single session run.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    pub limit: usize,
    pub safety_margin: f64,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            limit: DEFAULT_CONTEXT_WINDOW,
            safety_margin: 0.9, // 90% target to avoid hard cuts
        }
    }
}

impl ContextBudget {
    /// Calculate the effective token limit after safety margin.
    pub fn effective_limit(&self) -> usize {
        (self.limit as f64 * self.safety_margin) as usize
    }

    /// Prune messages if they exceed the budget (Sliding Window).
    /// Always keeps the system prompt and the most recent N messages.
    pub fn prune_to_fit(&self, messages: &mut Vec<Message>, keep_recent: usize) -> usize {
        let mut total_tokens: usize = messages.iter().map(|m| m.estimated_tokens()).sum();
        let limit = self.effective_limit();

        if total_tokens <= limit {
            return 0;
        }

        let mut removed = 0;
        // Keep the first message (usually system prompt) and the last N
        // Start removing from index 1
        while total_tokens > limit && messages.len() > keep_recent + 1 {
            let removed_msg = messages.remove(1);
            total_tokens -= removed_msg.estimated_tokens();
            removed += 1;
        }

        removed
    }
}
