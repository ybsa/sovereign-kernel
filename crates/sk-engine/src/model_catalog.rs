//! Known model registry and capabilities.
//!
//! Defines the capabilities of different LLMs (local and cloud)
//! to inform the routing layer.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelTier {
    /// Fast, local model (e.g. Llama 3 8B, Qwen 1.5B). Free, completely private, but limited reasoning.
    LocalLight = 1,
    /// Fast, cheap cloud model (e.g. Gemini Flash, Claude Haiku). Good for general chat.
    CloudFast = 2,
    /// Powerful cloud model (e.g. Claude 3.5 Sonnet, GPT-4o). Best for complex tool use and code.
    CloudReasoning = 3,
}

#[derive(Debug, Clone)]
pub struct ModelCapability {
    /// The model identifier (e.g. "claude-3-5-sonnet-20241022").
    pub name: String,
    /// The provider (e.g. "anthropic", "gemini", "local").
    pub provider: String,
    /// Capability tier.
    pub tier: ModelTier,
    /// Maximum context window size in tokens.
    pub max_context: usize,
    /// Whether this model supports native tool calling.
    pub supports_tools: bool,
}

impl ModelCapability {
    pub fn new(name: &str, provider: &str, tier: ModelTier, max_context: usize, supports_tools: bool) -> Self {
        Self {
            name: name.to_string(),
            provider: provider.to_string(),
            tier,
            max_context,
            supports_tools,
        }
    }
}

/// A standard catalog of models supported by Sovereign Kernel.
pub fn default_catalog() -> Vec<ModelCapability> {
    vec![
        ModelCapability::new("llama-3-8b-instruct", "local", ModelTier::LocalLight, 8192, false),
        ModelCapability::new("qwen2.5-coder-7b", "local", ModelTier::LocalLight, 32768, true),
        ModelCapability::new("gemini-1.5-flash", "gemini", ModelTier::CloudFast, 1000000, true),
        ModelCapability::new("claude-3-haiku-20240307", "anthropic", ModelTier::CloudFast, 200000, true),
        ModelCapability::new("claude-3-5-sonnet-20241022", "anthropic", ModelTier::CloudReasoning, 200000, true),
        ModelCapability::new("gemini-1.5-pro", "gemini", ModelTier::CloudReasoning, 2000000, true),
    ]
}
