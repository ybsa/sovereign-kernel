//! Model routing and selection.
//!
//! Intelligently analyzes conversation context and available tools
//! to dynamically switch between local and cloud models.

use crate::model_catalog::{default_catalog, ModelCapability, ModelTier};
use sk_types::{Message, ToolDefinition};

/// Analyzed complexity of a given task or conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    /// Simple chitchat or summarization — suitable for LocalLight.
    SimpleChat,
    /// Fast tool calling or data extraction — suitable for CloudFast.
    ToolExecution,
    /// Complex reasoning, coding, or long context — needs CloudReasoning.
    DeepReasoning,
}

/// Router to select the best model for a given task.
pub struct ModelRouter {
    catalog: Vec<ModelCapability>,
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new(default_catalog())
    }
}

impl ModelRouter {
    /// Create a new router with a specific model catalog.
    pub fn new(catalog: Vec<ModelCapability>) -> Self {
        Self { catalog }
    }

    /// Evaluates the context to determine the minimum required capability tier.
    pub fn evaluate_task(messages: &[Message], tools: &[ToolDefinition]) -> TaskComplexity {
        // If there are many tools, we probably need a model that's highly reliable at tool use
        if tools.len() > 3 {
            return TaskComplexity::DeepReasoning;
        }

        if !tools.is_empty() {
            return TaskComplexity::ToolExecution;
        }

        // Check context length based on estimated tokens
        let total_tokens: usize = messages.iter().map(|m| m.estimated_tokens()).sum();
        if total_tokens > 8192 {
            return TaskComplexity::ToolExecution; // Maps to CloudFast, Local might OOM or be too slow
        }

        // Keyword heuristics for complex tasks
        if let Some(last_msg) = messages.last() {
            let lower = last_msg.content.text_content().to_lowercase();
            if lower.contains("code")
                || lower.contains("architect")
                || lower.contains("debug")
                || lower.contains("analyze")
            {
                return TaskComplexity::DeepReasoning;
            }
        }

        TaskComplexity::SimpleChat
    }

    /// Returns a list of models capable of handling the task, sorted by preference.
    /// The first model is the primary choice; the rest are fallbacks.
    pub fn route(&self, complexity: TaskComplexity) -> Vec<ModelCapability> {
        let min_tier = match complexity {
            TaskComplexity::SimpleChat => ModelTier::LocalLight,
            TaskComplexity::ToolExecution => ModelTier::CloudFast,
            TaskComplexity::DeepReasoning => ModelTier::CloudReasoning,
        };

        let mut candidates: Vec<ModelCapability> = self
            .catalog
            .iter()
            .filter(|m| m.tier >= min_tier)
            .cloned()
            .collect();

        // Sort by tier ascending (cheapest/fastest capable model first)
        candidates.sort_by(|a, b| a.tier.cmp(&b.tier));

        candidates
    }
}
