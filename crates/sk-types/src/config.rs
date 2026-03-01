//! Global configuration types.

use crate::approval::ApprovalPolicy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level configuration for the Sovereign Kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    /// Data directory for databases, sessions, memories.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Home directory.
    #[serde(default = "default_data_dir")]
    pub home_dir: PathBuf,

    /// API listen address.
    #[serde(default = "default_api_listen")]
    pub api_listen: String,

    /// API key.
    #[serde(default)]
    pub api_key: String,

    /// Network enabled.
    #[serde(default)]
    pub network_enabled: bool,

    /// Network configuration.
    #[serde(default)]
    pub network: NetworkConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub vault: VaultConfig,

    #[serde(default)]
    pub channels: ChannelsConfig,

    #[serde(default)]
    pub web: WebConfig,

    #[serde(default)]
    pub approval: ApprovalPolicy,

    #[serde(default)]
    pub webhook_triggers: String,

    #[serde(default)]
    pub extensions: ExtensionsConfig,

    #[serde(default)]
    pub a2a: A2aConfig,

    #[serde(default)]
    pub fallback_providers: Vec<String>,

    #[serde(default)]
    pub log_level: String,

    #[serde(default)]
    pub language: String,

    #[serde(default)]
    pub mode: String,

    #[serde(default)]
    pub usage_footer: String,

    #[serde(default)]
    pub max_cron_jobs: usize,

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

    /// Embedding model settings.
    #[serde(default)]
    pub embeddings: EmbeddingConfig,

    /// Configuration for local inference (mistral.rs).
    #[serde(default)]
    pub local_inference: LocalInferenceConfig,

    /// Execution policy for sandbox tools.
    #[serde(default)]
    pub exec_policy: ExecPolicy,

    /// Docker sandbox configuration.
    #[serde(default)]
    pub docker_sandbox: DockerSandboxConfig,

    /// Browser configuration.
    #[serde(default)]
    pub browser: BrowserConfig,

    /// Media understanding configuration.
    #[serde(default)]
    pub media: crate::media::MediaConfig,

    /// Link extraction configuration.
    #[serde(default)]
    pub link: crate::media::LinkConfig,

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

/// Headless browser configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    /// Run browser in headless mode (no visible window).
    pub headless: bool,
    /// Viewport width in pixels.
    pub viewport_width: u32,
    /// Viewport height in pixels.
    pub viewport_height: u32,
    /// Per-action timeout in seconds.
    pub timeout_secs: u64,
    /// Idle timeout — auto-close session after this many seconds of inactivity.
    pub idle_timeout_secs: u64,
    /// Maximum concurrent browser sessions.
    pub max_sessions: usize,
    /// Python executable path (e.g., "python3" on Unix, "python" on Windows).
    pub python_path: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1280,
            viewport_height: 720,
            timeout_secs: 30,
            idle_timeout_secs: 300,
            max_sessions: 5,
            python_path: if cfg!(windows) {
                "python".to_string()
            } else {
                "python3".to_string()
            },
        }
    }
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

impl Default for LocalInferenceConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            gpu_layers: default_gpu_layers(),
            context_size: default_local_context(),
            threads: default_threads(),
        }
    }
}

fn default_api_listen() -> String {
    "127.0.0.1:8080".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub shared_secret: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelsConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionsConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub channel_bindings: HashMap<String, String>,
    #[serde(default)]
    pub api_key_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadMode {
    Off,
    Restart,
    Hot,
    #[default]
    Hybrid,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            home_dir: default_data_dir(),
            api_listen: default_api_listen(),
            api_key: String::new(),
            network_enabled: false,
            network: NetworkConfig::default(),
            memory: MemoryConfig::default(),
            vault: VaultConfig::default(),
            channels: ChannelsConfig::default(),
            web: WebConfig::default(),
            browser: BrowserConfig::default(),
            approval: ApprovalPolicy::default(),
            webhook_triggers: String::new(),
            extensions: ExtensionsConfig::default(),
            a2a: A2aConfig::default(),
            fallback_providers: Vec::new(),
            log_level: "info".into(),
            language: "en".into(),
            mode: "default".into(),
            usage_footer: "full".into(),
            max_cron_jobs: 1000,
            soul_path: None,
            default_provider: default_provider(),
            default_model: default_model(),
            api_keys: HashMap::new(),
            mcp_servers: HashMap::new(),
            embeddings: EmbeddingConfig::default(),
            local_inference: LocalInferenceConfig::default(),
            exec_policy: ExecPolicy::default(),
            docker_sandbox: DockerSandboxConfig::default(),
            media: crate::media::MediaConfig::default(),
            link: crate::media::LinkConfig::default(),
            memory_decay_rate: default_decay_rate(),
            context_window_tokens: default_context_window(),
        }
    }
}

impl KernelConfig {
    /// Load config from a TOML file path.
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::SovereignError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::SovereignError::ConfigError(format!(
                "Failed to read {}: {e}",
                path.display()
            ))
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::error::SovereignError::ConfigError(format!(
                "Failed to parse {}: {e}",
                path.display()
            ))
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

/// Docker sandbox activation mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerSandboxMode {
    /// Docker sandbox disabled.
    #[default]
    Off,
    /// Only use Docker for non-main agents.
    NonMain,
    /// Use Docker for all agents.
    All,
}

/// Docker container lifecycle scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerScope {
    /// Container per session (destroyed when session ends).
    #[default]
    Session,
    /// Container per agent (reused across sessions).
    Agent,
    /// Shared container pool.
    Shared,
}

/// Docker container sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerSandboxConfig {
    /// Enable Docker sandbox. Default: false.
    pub enabled: bool,
    /// Docker image for exec sandbox. Default: "python:3.12-slim".
    pub image: String,
    /// Container name prefix. Default: "openfang-sandbox".
    pub container_prefix: String,
    /// Working directory inside container. Default: "/workspace".
    pub workdir: String,
    /// Network mode: "none", "bridge", or custom. Default: "none".
    pub network: String,
    /// Memory limit (e.g., "256m", "1g"). Default: "512m".
    pub memory_limit: String,
    /// CPU limit (e.g., 0.5, 1.0, 2.0). Default: 1.0.
    pub cpu_limit: f64,
    /// Max execution time in seconds. Default: 60.
    pub timeout_secs: u64,
    /// Read-only root filesystem. Default: true.
    pub read_only_root: bool,
    /// Additional capabilities to add. Default: empty (drop all).
    pub cap_add: Vec<String>,
    /// tmpfs mounts. Default: ["/tmp:size=64m"].
    pub tmpfs: Vec<String>,
    /// PID limit. Default: 100.
    pub pids_limit: u32,
    /// Docker sandbox mode: off, non_main, all. Default: off.
    #[serde(default)]
    pub mode: DockerSandboxMode,
    /// Container lifecycle scope. Default: session.
    #[serde(default)]
    pub scope: DockerScope,
    /// Cooldown before reusing a released container (seconds). Default: 300.
    #[serde(default = "default_reuse_cool_secs")]
    pub reuse_cool_secs: u64,
    /// Idle timeout — destroy containers after N seconds of inactivity. Default: 86400 (24h).
    #[serde(default = "default_docker_idle_timeout")]
    pub idle_timeout_secs: u64,
    /// Maximum age before forced destruction (seconds). Default: 604800 (7 days).
    #[serde(default = "default_docker_max_age")]
    pub max_age_secs: u64,
    /// Paths blocked from bind mounting.
    #[serde(default)]
    pub blocked_mounts: Vec<String>,
}

fn default_reuse_cool_secs() -> u64 {
    300
}
fn default_docker_idle_timeout() -> u64 {
    86400
}
fn default_docker_max_age() -> u64 {
    604800
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            image: "python:3.12-slim".to_string(),
            container_prefix: "openfang-sandbox".to_string(),
            workdir: "/workspace".to_string(),
            network: "none".to_string(),
            memory_limit: "512m".to_string(),
            cpu_limit: 1.0,
            timeout_secs: 60,
            read_only_root: true,
            cap_add: Vec::new(),
            tmpfs: vec!["/tmp:size=64m".to_string()],
            pids_limit: 100,
            mode: DockerSandboxMode::Off,
            scope: DockerScope::Session,
            reuse_cool_secs: default_reuse_cool_secs(),
            idle_timeout_secs: default_docker_idle_timeout(),
            max_age_secs: default_docker_max_age(),
            blocked_mounts: Vec::new(),
        }
    }
}

/// Shell/exec security mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecSecurityMode {
    /// Block all shell execution.
    Deny,
    /// Only allow commands in safe_bins or allowed_commands.
    #[default]
    Allowlist,
    /// Allow all commands (unsafe, dev only).
    Full,
}

/// Shell/exec security policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecPolicy {
    /// Security mode: "deny" blocks all, "allowlist" only allows listed,
    /// "full" allows all (unsafe, dev only).
    pub mode: ExecSecurityMode,
    /// Commands that bypass allowlist (stdin-only utilities).
    pub safe_bins: Vec<String>,
    /// Global command allowlist (when mode = allowlist).
    pub allowed_commands: Vec<String>,
    /// Max execution timeout in seconds. Default: 30.
    pub timeout_secs: u64,
    /// Max output size in bytes. Default: 100KB.
    pub max_output_bytes: usize,
    /// No-output idle timeout in seconds. When > 0, kills processes that
    /// produce no stdout/stderr output for this duration. Default: 30.
    #[serde(default = "default_no_output_timeout")]
    pub no_output_timeout_secs: u64,
}

fn default_no_output_timeout() -> u64 {
    30
}

impl Default for ExecPolicy {
    fn default() -> Self {
        Self {
            mode: ExecSecurityMode::default(),
            safe_bins: vec![
                "sleep", "true", "false", "cat", "sort", "uniq", "cut", "tr", "head", "tail", "wc",
                "date", "echo", "printf", "basename", "dirname", "pwd", "env",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            allowed_commands: Vec::new(),
            timeout_secs: 30,
            max_output_bytes: 100 * 1024,
            no_output_timeout_secs: default_no_output_timeout(),
        }
    }
}

/// Reason a subprocess was terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    /// Process exited normally.
    Exited(i32),
    /// Absolute timeout exceeded.
    AbsoluteTimeout,
    /// No output timeout exceeded.
    NoOutputTimeout,
}

/// A2A (Agent-to-Agent) protocol configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct A2aConfig {
    /// Whether A2A is enabled.
    pub enabled: bool,
    /// Path to serve A2A endpoints (default: "/a2a").
    #[serde(default = "default_a2a_path")]
    pub listen_path: String,
    /// External A2A agents to connect to.
    #[serde(default)]
    pub external_agents: Vec<ExternalAgent>,
}

fn default_a2a_path() -> String {
    "/a2a".to_string()
}

/// An external A2A agent to discover and interact with.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalAgent {
    /// Display name.
    pub name: String,
    /// Agent endpoint URL.
    pub url: String,
}
