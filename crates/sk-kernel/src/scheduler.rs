//! Agent scheduler — manages agent execution and resource tracking.
//!
//! Ported from OpenFang's openfang-kernel/src/scheduler.rs — complete with
//! rolling hourly usage windows, resource quota enforcement, and task abort.

use dashmap::DashMap;
use sk_types::agent::{AgentId, ResourceQuota};
use sk_types::TokenUsage;
use sk_types::{SovereignError, SovereignResult};
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::debug;

/// Tracks resource usage for an agent with a rolling hourly window.
#[derive(Debug)]
pub struct UsageTracker {
    /// Total tokens consumed within the current window.
    pub total_tokens: u64,
    /// Total tool calls made within the current window.
    pub tool_calls: u64,
    /// Start of the current usage window.
    pub window_start: Instant,
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            tool_calls: 0,
            window_start: Instant::now(),
        }
    }
}

impl UsageTracker {
    /// Reset counters if the current window has expired (1 hour).
    fn reset_if_expired(&mut self) {
        if self.window_start.elapsed() >= std::time::Duration::from_secs(3600) {
            self.total_tokens = 0;
            self.tool_calls = 0;
            self.window_start = Instant::now();
        }
    }
}

/// The agent scheduler manages execution ordering and resource quotas.
pub struct AgentScheduler {
    /// Resource quotas per agent.
    quotas: DashMap<AgentId, ResourceQuota>,
    /// Usage tracking per agent.
    usage: DashMap<AgentId, UsageTracker>,
    /// Active task handles per agent.
    tasks: DashMap<AgentId, JoinHandle<()>>,
}

impl AgentScheduler {
    /// Create a new scheduler.
    pub fn new() -> Self {
        Self {
            quotas: DashMap::new(),
            usage: DashMap::new(),
            tasks: DashMap::new(),
        }
    }

    /// Register an agent with its resource quota.
    pub fn register(&self, agent_id: AgentId, quota: ResourceQuota) {
        self.quotas.insert(agent_id, quota);
        self.usage.insert(agent_id, UsageTracker::default());
    }

    /// Record token usage for an agent.
    pub fn record_usage(&self, agent_id: AgentId, usage: &TokenUsage) {
        if let Some(mut tracker) = self.usage.get_mut(&agent_id) {
            tracker.reset_if_expired();
            tracker.total_tokens += usage.total();
        }
    }

    /// Record a tool call for an agent.
    pub fn record_tool_call(&self, agent_id: AgentId) {
        if let Some(mut tracker) = self.usage.get_mut(&agent_id) {
            tracker.reset_if_expired();
            tracker.tool_calls += 1;
        }
    }

    /// Check if an agent has exceeded its quota.
    pub fn check_quota(&self, agent_id: AgentId) -> SovereignResult<()> {
        let quota = match self.quotas.get(&agent_id) {
            Some(q) => q.clone(),
            None => return Ok(()), // No quota = no limit
        };
        let mut tracker = match self.usage.get_mut(&agent_id) {
            Some(t) => t,
            None => return Ok(()),
        };

        // Reset the window if an hour has passed
        tracker.reset_if_expired();

        if tracker.total_tokens > quota.max_llm_tokens_per_hour {
            return Err(SovereignError::QuotaExceeded(format!(
                "Token limit exceeded: {} / {}",
                tracker.total_tokens, quota.max_llm_tokens_per_hour
            )));
        }

        if tracker.tool_calls > quota.max_tool_calls_per_minute as u64 {
            return Err(SovereignError::QuotaExceeded(format!(
                "Tool call limit exceeded: {} / {}",
                tracker.tool_calls, quota.max_tool_calls_per_minute
            )));
        }

        Ok(())
    }

    /// Store a task handle for an agent.
    pub fn register_task(&self, agent_id: AgentId, handle: JoinHandle<()>) {
        self.tasks.insert(agent_id, handle);
    }

    /// Abort an agent's active task.
    pub fn abort_task(&self, agent_id: AgentId) {
        if let Some((_, handle)) = self.tasks.remove(&agent_id) {
            handle.abort();
            debug!(agent = %agent_id, "Aborted agent task");
        }
    }

    /// Remove an agent from the scheduler.
    pub fn unregister(&self, agent_id: AgentId) {
        self.abort_task(agent_id);
        self.quotas.remove(&agent_id);
        self.usage.remove(&agent_id);
    }

    /// Get usage stats for an agent.
    pub fn get_usage(&self, agent_id: AgentId) -> Option<(u64, u64)> {
        self.usage
            .get(&agent_id)
            .map(|t| (t.total_tokens, t.tool_calls))
    }

    /// Get all registered agent IDs.
    pub fn registered_agents(&self) -> Vec<AgentId> {
        self.quotas.iter().map(|r| *r.key()).collect()
    }
}

impl Default for AgentScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_usage() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        scheduler.register(id, ResourceQuota::default());
        scheduler.record_usage(
            id,
            &TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
        );
        let (tokens, _) = scheduler.get_usage(id).unwrap();
        assert_eq!(tokens, 150);
    }

    #[test]
    fn test_tool_call_tracking() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        scheduler.register(id, ResourceQuota::default());
        scheduler.record_tool_call(id);
        scheduler.record_tool_call(id);
        let (_, calls) = scheduler.get_usage(id).unwrap();
        assert_eq!(calls, 2);
    }

    #[test]
    fn test_quota_check_pass() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        scheduler.register(id, ResourceQuota::default());
        scheduler.record_usage(
            id,
            &TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
        );
        assert!(scheduler.check_quota(id).is_ok());
    }

    #[test]
    fn test_quota_check_exceeded() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        let quota = ResourceQuota {
            max_llm_tokens_per_hour: 100,
            ..Default::default()
        };
        scheduler.register(id, quota);
        scheduler.record_usage(
            id,
            &TokenUsage {
                input_tokens: 60,
                output_tokens: 50,
            },
        );
        assert!(scheduler.check_quota(id).is_err());
    }

    #[test]
    fn test_unregister() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        scheduler.register(id, ResourceQuota::default());
        assert!(scheduler.get_usage(id).is_some());
        scheduler.unregister(id);
        assert!(scheduler.get_usage(id).is_none());
    }

    #[test]
    fn test_no_quota_no_limit() {
        let scheduler = AgentScheduler::new();
        let id = AgentId::new();
        // Not registered — should pass
        assert!(scheduler.check_quota(id).is_ok());
    }

    #[test]
    fn test_registered_agents() {
        let scheduler = AgentScheduler::new();
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        scheduler.register(id1, ResourceQuota::default());
        scheduler.register(id2, ResourceQuota::default());
        let agents = scheduler.registered_agents();
        assert_eq!(agents.len(), 2);
    }
}
