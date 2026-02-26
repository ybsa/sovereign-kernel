//! Global configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level configuration for the Sovereign Kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignConfig {
    /// Data directory for databases, sessions, memories.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Path to SOUL.md identity file.
    #[serde(default)]
    pub soul_path: Option<PathBuf>,

    /// Default LLM provider.
    #[serde(default = "default_provider")]
    pub default_provider: String,

    /// Default model.
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Provider API keys (provider_name → key).
    #[serde(default)]
    pub api_keys: HashMap<String, String>,

    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerEntry>,

    /// Embedding configuration.
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// Local inference configuration.
    #[serde(default)]
    pub local_inference: Option<LocalInferenceConfig>,

    /// Memory decay rate (0.0 = no decay, 1.0 = aggressive decay).
    #[serde(default = "default_decay_rate")]
    pub memory_decay_rate: f32,

    /// Maximum context window tokens.
    #[serde(default = "default_context_window")]
    pub context_window_tokens: usize,
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sovereign-kernel")
}

fn default_provider() -> String {
    "anthropic".into()
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".into()
}

fn default_decay_rate() -> f32 {
    0.1
}

fn default_context_window() -> usize {
    128_000
}

/// MCP server connection entry in config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Transport type: "stdio" or "sse".
    pub transport: String,
    /// For stdio: command to execute.
    #[serde(default)]
    pub command: Option<String>,
    /// For stdio: command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// For SSE: URL to connect to.
    #[serde(default)]
    pub url: Option<String>,
    /// Environment variables to pass.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Embedding model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider (e.g. "openai", "local").
    #[serde(default = "default_embed_provider")]
    pub provider: String,
    /// Model name.
    #[serde(default = "default_embed_model")]
    pub model: String,
    /// Dimensions (auto-inferred if 0).
    #[serde(default)]
    pub dimensions: usize,
}

fn default_embed_provider() -> String {
    "openai".into()
}

fn default_embed_model() -> String {
    "text-embedding-3-small".into()
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_embed_provider(),
            model: default_embed_model(),
            dimensions: 0,
        }
    }
}

/// Local inference configuration (mistral.rs / llama-cpp-rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInferenceConfig {
    /// Path to the GGUF model file.
    pub model_path: PathBuf,
    /// Number of GPU layers to offload (-1 = all).
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: i32,
    /// Context size.
    #[serde(default = "default_local_context")]
    pub context_size: usize,
    /// Number of threads for CPU inference.
    #[serde(default = "default_threads")]
    pub threads: usize,
}

fn default_gpu_layers() -> i32 {
    -1
}

fn default_local_context() -> usize {
    8192
}

fn default_threads() -> usize {
    4
}

impl Default for SovereignConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            soul_path: None,
            default_provider: default_provider(),
            default_model: default_model(),
            api_keys: HashMap::new(),
            mcp_servers: HashMap::new(),
            embedding: EmbeddingConfig::default(),
            local_inference: None,
            memory_decay_rate: default_decay_rate(),
            context_window_tokens: default_context_window(),
        }
    }
}

impl SovereignConfig {
    /// Load config from a TOML file path.
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::SovereignError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::SovereignError::ConfigError(format!("Failed to read {}: {e}", path.display()))
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::error::SovereignError::ConfigError(format!("Failed to parse {}: {e}", path.display()))
        })
    }

    /// Get the database file path.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("sovereign.db")
    }

    /// Get the memories directory path.
    pub fn memories_dir(&self) -> PathBuf {
        self.data_dir.join("memories")
    }
}
