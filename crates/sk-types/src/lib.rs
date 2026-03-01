//! Core types for the Sovereign Kernel.
//!
//! Foundation types shared across all crates: agent identities, messages,
//! tool definitions, sessions, errors, config, security primitives,
//! approval policies, and scheduler types.

pub mod agent;
pub mod approval;
pub mod config;
pub mod error;
pub mod event;
pub mod media;
pub mod message;
pub mod scheduler;
pub mod security;
pub mod session;
pub mod tool;

pub use agent::{
    AgentEntry, AgentId, AgentManifest, AgentMode, AgentState, AutonomousConfig, HookEvent,
    ModelConfig, ModelRoutingConfig, Priority, ResourceQuota, ScheduleMode, TokenUsage,
    ToolProfile, UserId,
};
pub use approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse, RiskLevel,
};
pub use config::KernelConfig;
pub use error::{SovereignError, SovereignResult};
pub use message::{Message, Role};
pub use scheduler::{CronAction, CronDelivery, CronJob, CronJobId, CronSchedule};
pub use session::{Session, SessionId};
pub use tool::{ToolCall, ToolDefinition, ToolResult};
