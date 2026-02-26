//! Core types for the Sovereign Kernel.
//!
//! Foundation types shared across all crates: agent identities, messages,
//! tool definitions, sessions, errors, config, and security primitives.

pub mod agent;
pub mod config;
pub mod error;
pub mod message;
pub mod security;
pub mod session;
pub mod tool;

pub use agent::{AgentEntry, AgentId, AgentManifest};
pub use config::SovereignConfig;
pub use error::{SovereignError, SovereignResult};
pub use message::{Message, Role};
pub use session::{Session, SessionId};
pub use tool::{ToolCall, ToolDefinition, ToolResult};
