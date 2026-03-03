//! Core types and traits for the Sovereign Kernel.
//!
//! Foundation types shared across all crates: agent identities, messages,
//! tool definitions, sessions, errors, config, security primitives,
//! approval policies, and scheduler types.

pub mod agent;
pub mod approval;
pub mod capability;
pub mod config;
pub mod error;
pub mod event;
pub mod manifest_signing;
pub mod media;
pub mod memory;
pub mod message;
pub mod model_catalog;
pub mod scheduler;
pub mod serde_compat;
pub mod taint;
pub mod tool;
pub mod tool_compat;
pub mod webhook;

// Sovereign Kernel specific
pub mod security;
pub mod session;

pub use agent::{
    AgentEntry, AgentId, AgentManifest, AgentMode, AgentState, AutonomousConfig, HookEvent,
    ModelConfig, ModelRoutingConfig, Priority, ResourceQuota, ScheduleMode, ToolProfile, UserId,
};
pub use approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse, RiskLevel,
};
pub use config::{KernelConfig, McpServerEntry};
pub use error::{SovereignError, SovereignResult};
pub use message::{ContentBlock, Message, MessageContent, Role, TokenUsage};
pub use scheduler::{CronAction, CronDelivery, CronJob, CronJobId, CronSchedule};
pub use session::{Session, SessionId};
pub use tool::{ToolCall, ToolDefinition, ToolResult};
