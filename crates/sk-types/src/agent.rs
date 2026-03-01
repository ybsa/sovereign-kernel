//! Agent identity and manifest types.
//!
//! Ported from OpenFang's openfang-types/src/agent.rs — complete with
//! agent state, resource quotas, autonomous config, scheduling modes,
//! model configs, tool profiles, and permission modes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

use crate::tool::ToolDefinition;

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

impl std::str::FromStr for AgentId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for AgentId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

// ---------------------------------------------------------------------------
// UserId
// ---------------------------------------------------------------------------

/// Unique identifier for a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Generate a new random UserId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UserId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

// ---------------------------------------------------------------------------
// AgentState
// ---------------------------------------------------------------------------

/// The current lifecycle state of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent has been created but not yet started.
    Created,
    /// Agent is actively running and processing events.
    Running,
    /// Agent is paused and not processing events.
    Suspended,
    /// Agent has been terminated and cannot be resumed.
    Terminated,
    /// Agent crashed and is awaiting recovery.
    Crashed,
}

// ---------------------------------------------------------------------------
// AgentMode
// ---------------------------------------------------------------------------

/// Permission-based operational mode for an agent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Read-only: agent can observe but cannot call any tools.
    Observe,
    /// Restricted: agent can only call read-only tools.
    Assist,
    /// Unrestricted: agent can use all granted tools.
    #[default]
    Full,
}

impl AgentMode {
    /// Filter a tool list based on this mode.
    pub fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
        match self {
            Self::Observe => vec![],
            Self::Assist => {
                let read_only = [
                    "file_read",
                    "file_list",
                    "memory_recall",
                    "web_fetch",
                    "web_search",
                    "agent_list",
                ];
                tools
                    .into_iter()
                    .filter(|t| read_only.contains(&t.name.as_str()))
                    .collect()
            }
            Self::Full => tools,
        }
    }
}

// ---------------------------------------------------------------------------
// Priority
// ---------------------------------------------------------------------------

/// Agent priority level for scheduling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority.
    Low = 0,
    /// Normal priority (default).
    #[default]
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority.
    Critical = 3,
}

// ---------------------------------------------------------------------------
// ScheduleMode
// ---------------------------------------------------------------------------

/// How an agent is scheduled to run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    /// Agent wakes up when a message/event arrives (default).
    #[default]
    Reactive,
    /// Agent wakes up on a cron schedule.
    Periodic { cron: String },
    /// Agent monitors conditions and acts when thresholds are met.
    Proactive { conditions: Vec<String> },
    /// Agent runs in a persistent loop.
    Continuous {
        #[serde(default = "default_check_interval")]
        check_interval_secs: u64,
    },
}

fn default_check_interval() -> u64 {
    60
}

// ---------------------------------------------------------------------------
// ResourceQuota
// ---------------------------------------------------------------------------

/// Resource limits for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceQuota {
    /// Maximum WASM memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum CPU time per invocation in milliseconds.
    pub max_cpu_time_ms: u64,
    /// Maximum tool calls per minute.
    pub max_tool_calls_per_minute: u32,
    /// Maximum LLM tokens per hour.
    pub max_llm_tokens_per_hour: u64,
    /// Maximum network bytes per hour.
    pub max_network_bytes_per_hour: u64,
    /// Maximum cost in USD per hour.
    pub max_cost_per_hour_usd: f64,
    /// Maximum cost in USD per day (0.0 = unlimited).
    pub max_cost_per_day_usd: f64,
    /// Maximum cost in USD per month (0.0 = unlimited).
    pub max_cost_per_month_usd: f64,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_cpu_time_ms: 30_000,             // 30 seconds
            max_tool_calls_per_minute: 60,
            max_llm_tokens_per_hour: 1_000_000,
            max_network_bytes_per_hour: 100 * 1024 * 1024, // 100 MB
            max_cost_per_hour_usd: 1.0,
            max_cost_per_day_usd: 0.0,   // unlimited
            max_cost_per_month_usd: 0.0, // unlimited
        }
    }
}

// ---------------------------------------------------------------------------
// AutonomousConfig
// ---------------------------------------------------------------------------

/// Autonomous agent configuration — guardrails for 24/7 agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutonomousConfig {
    /// Cron expression for quiet hours (e.g., "22:00-06:00").
    pub quiet_hours: Option<String>,
    /// Maximum iterations per invocation (overrides global MAX_ITERATIONS).
    pub max_iterations: u32,
    /// Maximum restarts before the agent is permanently stopped.
    pub max_restarts: u32,
    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,
    /// Channel to send heartbeat status to (e.g., "telegram", "discord").
    pub heartbeat_channel: Option<String>,
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            quiet_hours: None,
            max_iterations: 50,
            max_restarts: 10,
            heartbeat_interval_secs: 30,
            heartbeat_channel: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ModelConfig
// ---------------------------------------------------------------------------

/// LLM model configuration for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    /// LLM provider name.
    pub provider: String,
    /// Model identifier.
    pub model: String,
    /// Maximum tokens for completion.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// System prompt for the agent.
    pub system_prompt: String,
    /// Optional API key environment variable name.
    pub api_key_env: Option<String>,
    /// Optional base URL override for the provider.
    pub base_url: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            system_prompt: "You are a helpful AI agent.".to_string(),
            api_key_env: None,
            base_url: None,
        }
    }
}

// ---------------------------------------------------------------------------
// FallbackModel
// ---------------------------------------------------------------------------

/// A fallback model entry in a chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackModel {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

// ---------------------------------------------------------------------------
// ModelRoutingConfig
// ---------------------------------------------------------------------------

/// Model routing configuration — auto-selects cheap/mid/expensive models by complexity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelRoutingConfig {
    /// Model to use for simple queries.
    pub simple_model: String,
    /// Model to use for medium-complexity queries.
    pub medium_model: String,
    /// Model to use for complex queries.
    pub complex_model: String,
    /// Token count threshold: below this = simple.
    pub simple_threshold: u32,
    /// Token count threshold: above this = complex.
    pub complex_threshold: u32,
}

impl Default for ModelRoutingConfig {
    fn default() -> Self {
        Self {
            simple_model: "gemini-2.0-flash".to_string(),
            medium_model: "gemini-2.0-flash".to_string(),
            complex_model: "gemini-2.5-pro-preview-06-05".to_string(),
            simple_threshold: 100,
            complex_threshold: 500,
        }
    }
}

// ---------------------------------------------------------------------------
// HookEvent
// ---------------------------------------------------------------------------

/// Hook event types that can be intercepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Fires before a tool call is executed. Handler can block the call.
    BeforeToolCall,
    /// Fires after a tool call completes.
    AfterToolCall,
    /// Fires before the system prompt is constructed.
    BeforePromptBuild,
    /// Fires after the agent loop completes.
    AgentLoopEnd,
}

// ---------------------------------------------------------------------------
// ToolProfile
// ---------------------------------------------------------------------------

/// Named tool presets — expand to tool lists + derived capabilities.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolProfile {
    Minimal,
    Coding,
    Research,
    Messaging,
    Automation,
    #[default]
    Full,
    Custom,
}

impl ToolProfile {
    /// Expand profile to tool name list.
    pub fn tools(&self) -> Vec<String> {
        match self {
            Self::Minimal => vec!["file_read", "file_list"],
            Self::Coding => vec![
                "file_read",
                "file_write",
                "file_list",
                "shell_exec",
                "web_fetch",
            ],
            Self::Research => vec!["web_fetch", "web_search", "file_read", "file_write"],
            Self::Messaging => vec!["agent_send", "agent_list", "memory_store", "memory_recall"],
            Self::Automation => vec![
                "file_read",
                "file_write",
                "file_list",
                "shell_exec",
                "web_fetch",
                "web_search",
                "agent_send",
                "agent_list",
                "memory_store",
                "memory_recall",
            ],
            Self::Full | Self::Custom => vec!["*"],
        }
        .into_iter()
        .map(String::from)
        .collect()
    }
}

// ---------------------------------------------------------------------------
// ToolConfig
// ---------------------------------------------------------------------------

/// Tool configuration within an agent manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool-specific configuration parameters.
    pub params: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// ManifestCapabilities
// ---------------------------------------------------------------------------

/// Capability declarations in a manifest (human-readable format).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManifestCapabilities {
    /// Allowed network hosts.
    pub network: Vec<String>,
    /// Allowed tool IDs.
    pub tools: Vec<String>,
    /// Memory read scopes.
    pub memory_read: Vec<String>,
    /// Memory write scopes.
    pub memory_write: Vec<String>,
    /// Whether this agent can spawn sub-agents.
    pub agent_spawn: bool,
    /// Agent message patterns.
    pub agent_message: Vec<String>,
    /// Allowed shell commands.
    pub shell: Vec<String>,
}

// ---------------------------------------------------------------------------
// AgentIdentity
// ---------------------------------------------------------------------------

/// Visual identity for an agent — emoji, avatar, color, personality.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentIdentity {
    /// Single emoji character for quick visual identification.
    pub emoji: Option<String>,
    /// Avatar URL (http/https) or data URI.
    pub avatar_url: Option<String>,
    /// Hex color code (e.g., "#FF5C00") for UI accent.
    pub color: Option<String>,
    /// Archetype: "researcher", "coder", "assistant", "writer", etc.
    pub archetype: Option<String>,
    /// Personality vibe: "professional", "friendly", "technical", etc.
    pub vibe: Option<String>,
}

// ---------------------------------------------------------------------------
// AgentManifest
// ---------------------------------------------------------------------------

/// Complete agent manifest — defines everything about an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentManifest {
    /// Human-readable agent name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Description of what this agent does.
    pub description: String,
    /// Author identifier.
    pub author: String,
    /// System prompt (may be overridden by Soul injection).
    pub system_prompt: String,
    /// Scheduling mode.
    pub schedule: ScheduleMode,
    /// LLM model configuration.
    pub model: ModelConfig,
    /// Fallback model chain — tried in order if the primary model fails.
    #[serde(default)]
    pub fallback_models: Vec<FallbackModel>,
    /// Resource quotas.
    pub resources: ResourceQuota,
    /// Priority level.
    pub priority: Priority,
    /// Capability grants.
    pub capabilities: ManifestCapabilities,
    /// Named tool profile — expands to tool list + derived capabilities.
    #[serde(default)]
    pub profile: Option<ToolProfile>,
    /// Preferred LLM provider (e.g. "anthropic", "openai", "local").
    #[serde(default)]
    pub provider: Option<String>,
    /// Tools this agent is allowed to invoke (empty = all).
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Maximum tokens per response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for LLM sampling.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Tool-specific configurations.
    #[serde(default)]
    pub tools: HashMap<String, ToolConfig>,
    /// Installed skill references (empty = all skills available).
    #[serde(default)]
    pub skills: Vec<String>,
    /// MCP server allowlist (empty = all connected MCP servers available).
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    /// Custom metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for agent discovery and categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Model routing configuration — auto-select models by complexity.
    #[serde(default)]
    pub routing: Option<ModelRoutingConfig>,
    /// Autonomous agent configuration — guardrails for 24/7 agents.
    #[serde(default)]
    pub autonomous: Option<AutonomousConfig>,
    /// Agent workspace directory.
    #[serde(default)]
    pub workspace: Option<PathBuf>,
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
            version: "0.1.0".into(),
            description: "Default Sovereign Kernel agent".into(),
            author: String::new(),
            system_prompt: String::new(),
            schedule: ScheduleMode::default(),
            model: ModelConfig::default(),
            fallback_models: Vec::new(),
            resources: ResourceQuota::default(),
            priority: Priority::default(),
            capabilities: ManifestCapabilities::default(),
            profile: None,
            provider: None,
            allowed_tools: Vec::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            tools: HashMap::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            routing: None,
            autonomous: None,
            workspace: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentEntry
// ---------------------------------------------------------------------------

/// A registered agent entry in the kernel's registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    /// Unique agent ID.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// Full manifest.
    pub manifest: AgentManifest,
    /// Current lifecycle state.
    pub state: AgentState,
    /// Permission-based operational mode.
    #[serde(default)]
    pub mode: AgentMode,
    /// When the agent was created.
    pub created_at: DateTime<Utc>,
    /// When the agent was last active.
    pub last_active: DateTime<Utc>,
    /// Parent agent (if spawned by another agent).
    pub parent: Option<AgentId>,
    /// Child agents spawned by this agent.
    #[serde(default)]
    pub children: Vec<AgentId>,
    /// Active session ID.
    pub session_id: crate::session::SessionId,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Visual identity for dashboard display.
    #[serde(default)]
    pub identity: AgentIdentity,
}

impl AgentEntry {
    /// Create a new agent entry with a fresh ID and current timestamp.
    pub fn new(manifest: AgentManifest) -> Self {
        let now = Utc::now();
        let name = manifest.name.clone();
        Self {
            id: AgentId::new(),
            name,
            manifest,
            state: AgentState::Created,
            mode: AgentMode::default(),
            created_at: now,
            last_active: now,
            parent: None,
            children: Vec::new(),
            session_id: crate::session::SessionId::new(),
            tags: Vec::new(),
            identity: AgentIdentity::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// TokenUsage
// ---------------------------------------------------------------------------

/// Token usage for a single LLM call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens consumed.
    pub input_tokens: u64,
    /// Number of output tokens generated.
    pub output_tokens: u64,
}

impl TokenUsage {
    /// Total tokens consumed.
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn agent_id_from_str() {
        let id = AgentId::new();
        let s = id.to_string();
        let parsed: AgentId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn agent_id_uniqueness() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn user_id_uniqueness() {
        let u1 = UserId::new();
        let u2 = UserId::new();
        assert_ne!(u1, u2);
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
        assert_eq!(parsed.state, AgentState::Created);
    }

    #[test]
    fn resource_quota_defaults() {
        let q = ResourceQuota::default();
        assert_eq!(q.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(q.max_cpu_time_ms, 30_000);
        assert_eq!(q.max_llm_tokens_per_hour, 1_000_000);
    }

    #[test]
    fn autonomous_config_defaults() {
        let c = AutonomousConfig::default();
        assert_eq!(c.max_iterations, 50);
        assert_eq!(c.max_restarts, 10);
        assert_eq!(c.heartbeat_interval_secs, 30);
        assert!(c.quiet_hours.is_none());
    }

    #[test]
    fn model_routing_config_defaults() {
        let r = ModelRoutingConfig::default();
        assert!(!r.simple_model.is_empty());
        assert!(r.simple_threshold < r.complex_threshold);
    }

    #[test]
    fn tool_profile_minimal() {
        let tools = ToolProfile::Minimal.tools();
        assert_eq!(tools, vec!["file_read", "file_list"]);
    }

    #[test]
    fn tool_profile_coding() {
        let tools = ToolProfile::Coding.tools();
        assert!(tools.contains(&"shell_exec".to_string()));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn tool_profile_full_wildcard() {
        let tools = ToolProfile::Full.tools();
        assert_eq!(tools, vec!["*"]);
    }

    #[test]
    fn agent_mode_filter_observe() {
        let tools = vec![ToolDefinition {
            name: "shell_exec".into(),
            description: "Execute shell commands".into(),
            parameters: serde_json::json!({}),
            source: "builtin".into(),
            required_capabilities: vec![],
        }];
        let filtered = AgentMode::Observe.filter_tools(tools);
        assert!(filtered.is_empty());
    }

    #[test]
    fn agent_mode_filter_full() {
        let tools = vec![ToolDefinition {
            name: "shell_exec".into(),
            description: "Execute shell commands".into(),
            parameters: serde_json::json!({}),
            source: "builtin".into(),
            required_capabilities: vec![],
        }];
        let filtered = AgentMode::Full.filter_tools(tools);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn agent_state_serde() {
        let state = AgentState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"running\"");
        let back: AgentState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AgentState::Running);
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }
}
