//! Error types for the Sovereign Kernel.

use thiserror::Error;

/// Unified error type for the Sovereign Kernel.
#[derive(Debug, Error)]
pub enum SovereignError {
    #[error("LLM driver error: {0}")]
    LlmError(String),

    #[error("Tool execution error: {tool}: {message}")]
    ToolError { tool: String, message: String },

    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Context overflow: {used} tokens used, {limit} limit")]
    ContextOverflow { used: usize, limit: usize },

    #[error("Loop detected: {0}")]
    LoopDetected(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Budget exceeded: {resource} — {message}")]
    BudgetExceeded { resource: String, message: String },

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Approval denied: {tool} — {reason}")]
    ApprovalDenied { tool: String, reason: String },

    #[error("Auth denied: {0}")]
    AuthDenied(String),

    #[error("Approval timeout: {tool}")]
    ApprovalTimeout { tool: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience alias.
pub type SovereignResult<T> = Result<T, SovereignError>;

impl SovereignError {
    /// Whether this error is retryable (rate limit, overload, transient).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SovereignError::RateLimited { .. } | SovereignError::Timeout(_)
        )
    }

    /// Whether this error is a budget/quota violation.
    pub fn is_budget_error(&self) -> bool {
        matches!(
            self,
            SovereignError::QuotaExceeded(_) | SovereignError::BudgetExceeded { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = SovereignError::LlmError("bad request".into());
        assert!(e.to_string().contains("bad request"));
    }

    #[test]
    fn error_retryable() {
        assert!(SovereignError::RateLimited {
            retry_after_ms: 1000
        }
        .is_retryable());
        assert!(SovereignError::Timeout("slow".into()).is_retryable());
        assert!(!SovereignError::ConfigError("bad".into()).is_retryable());
    }
}
