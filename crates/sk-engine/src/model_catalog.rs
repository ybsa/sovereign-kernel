//! Known model registry and capabilities.
//!
//! Defines the capabilities of different LLMs (local and cloud)
//! to inform the routing layer.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ModelTier {
    /// Fast, local model (e.g. Llama 3 8B, Qwen 1.5B). Free, completely private, but limited reasoning.
    LocalLight = 1,
    /// Fast, cheap cloud model (e.g. Gemini Flash, Claude Haiku). Good for general chat.
    CloudFast = 2,
    /// Powerful cloud model (e.g. Claude 3.5 Sonnet, GPT-4o). Best for complex tool use and code.
    CloudReasoning = 3,
    /// Top-tier frontier model (e.g. Claude 3 Opus, GPT-4.1).
    CloudFrontier = 4,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Whether this model supports vision/image inputs.
    pub supports_vision: bool,
    /// Whether this model supports "Computer Use" (Anthropic-specific).
    pub supports_computer_use: bool,
}

impl ModelCapability {
    pub fn new(
        name: &str,
        provider: &str,
        tier: ModelTier,
        max_context: usize,
        supports_tools: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            provider: provider.to_string(),
            tier,
            max_context,
            supports_tools,
            supports_vision: false,
            supports_computer_use: false,
        }
    }

    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }

    pub fn with_computer_use(mut self) -> Self {
        self.supports_vision = true;
        self.supports_computer_use = true;
        self
    }
}

/// A standard catalog of models supported by Sovereign Kernel.
pub fn default_catalog() -> Vec<ModelCapability> {
    vec![
        // ── Anthropic ──────────────────────────────────────────────
        ModelCapability::new(
            "claude-3-5-sonnet-20241022",
            "anthropic",
            ModelTier::CloudReasoning,
            200000,
            true,
        )
        .with_computer_use(),
        ModelCapability::new(
            "claude-3-5-haiku-20241022",
            "anthropic",
            ModelTier::CloudFast,
            200000,
            true,
        )
        .with_vision(),
        ModelCapability::new(
            "claude-3-haiku-20240307",
            "anthropic",
            ModelTier::CloudFast,
            200000,
            true,
        )
        .with_vision(),
        ModelCapability::new(
            "claude-3-opus-20240229",
            "anthropic",
            ModelTier::CloudFrontier,
            200000,
            true,
        )
        .with_vision(),
        // ── OpenAI ─────────────────────────────────────────────────
        ModelCapability::new("gpt-4o", "openai", ModelTier::CloudReasoning, 128000, true)
            .with_vision(),
        ModelCapability::new("gpt-4o-mini", "openai", ModelTier::CloudFast, 128000, true)
            .with_vision(),
        ModelCapability::new(
            "o1-preview",
            "openai",
            ModelTier::CloudFrontier,
            128000,
            true,
        ),
        ModelCapability::new("o1-mini", "openai", ModelTier::CloudReasoning, 128000, true),
        ModelCapability::new(
            "gpt-4-turbo",
            "openai",
            ModelTier::CloudReasoning,
            128000,
            true,
        )
        .with_vision(),
        // ── Google Gemini ──────────────────────────────────────────
        ModelCapability::new(
            "gemini-2.0-flash-exp",
            "gemini",
            ModelTier::CloudFast,
            1000000,
            true,
        )
        .with_vision(),
        ModelCapability::new(
            "gemini-1.5-pro",
            "gemini",
            ModelTier::CloudReasoning,
            2000000,
            true,
        )
        .with_vision(),
        ModelCapability::new(
            "gemini-1.5-flash",
            "gemini",
            ModelTier::CloudFast,
            1000000,
            true,
        )
        .with_vision(),
        // ── DeepSeek ───────────────────────────────────────────────
        ModelCapability::new(
            "deepseek-chat",
            "deepseek",
            ModelTier::CloudFast,
            64000,
            true,
        ),
        ModelCapability::new(
            "deepseek-reasoner",
            "deepseek",
            ModelTier::CloudReasoning,
            64000,
            true,
        ),
        // ── Groq (Llama / Mixtral / Qwen) ──────────────────────────
        ModelCapability::new(
            "llama-3.3-70b-versatile",
            "groq",
            ModelTier::CloudReasoning,
            128000,
            true,
        ),
        ModelCapability::new(
            "llama-3.1-8b-instant",
            "groq",
            ModelTier::CloudFast,
            128000,
            true,
        ),
        ModelCapability::new(
            "mixtral-8x7b-32768",
            "groq",
            ModelTier::CloudFast,
            32768,
            true,
        ),
        // ── xAI Grok ───────────────────────────────────────────────
        ModelCapability::new("grok-beta", "xai", ModelTier::CloudFast, 128000, true),
        // ── Local (Ollama / vLLM) ──────────────────────────────────
        ModelCapability::new(
            "llama3.3:latest",
            "local",
            ModelTier::LocalLight,
            8192,
            true,
        ),
        ModelCapability::new(
            "qwen2.5-coder:7b",
            "local",
            ModelTier::LocalLight,
            32768,
            true,
        ),
        ModelCapability::new("mistral:latest", "local", ModelTier::LocalLight, 8192, true),
        ModelCapability::new("phi3:latest", "local", ModelTier::LocalLight, 4096, true),
    ]
}
