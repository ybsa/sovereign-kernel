//! Heartbeat monitor — detects unresponsive agents for 24/7 autonomous operation.
//!
//! Ported from OpenFang's openfang-kernel/src/heartbeat.rs — complete with
//! quiet hours support, configurable timeouts, per-agent heartbeat intervals,
//! and aggregate health summaries.

use chrono::Utc;
use sk_types::agent::{AgentId, AgentState};
use tracing::{debug, warn};

/// Default heartbeat check interval (seconds).
const DEFAULT_CHECK_INTERVAL_SECS: u64 = 30;

/// Multiplier: agent is considered unresponsive if inactive for this many
/// multiples of its heartbeat interval.
const UNRESPONSIVE_MULTIPLIER: u64 = 2;

/// Result of a heartbeat check.
#[derive(Debug, Clone)]
pub struct HeartbeatStatus {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Agent name.
    pub name: String,
    /// Seconds since last activity.
    pub inactive_secs: i64,
    /// Whether the agent is considered unresponsive.
    pub unresponsive: bool,
}

/// Heartbeat monitor configuration.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// How often to run the heartbeat check (seconds).
    pub check_interval_secs: u64,
    /// Default threshold for unresponsiveness (seconds).
    /// Overridden per-agent by AutonomousConfig.heartbeat_interval_secs.
    pub default_timeout_secs: u64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: DEFAULT_CHECK_INTERVAL_SECS,
            default_timeout_secs: DEFAULT_CHECK_INTERVAL_SECS * UNRESPONSIVE_MULTIPLIER,
        }
    }
}

/// Agent info needed for heartbeat checks.
/// This is a lightweight struct that the registry should provide.
pub struct AgentHeartbeatInfo {
    pub id: AgentId,
    pub name: String,
    pub state: AgentState,
    pub last_active: chrono::DateTime<Utc>,
    pub heartbeat_interval_secs: Option<u64>,
}

/// Check a list of agents and return their heartbeat status.
///
/// This is a pure function — it doesn't start a background task.
/// The caller (kernel) can run this periodically or in a background task.
pub fn check_agents(
    agents: &[AgentHeartbeatInfo],
    config: &HeartbeatConfig,
) -> Vec<HeartbeatStatus> {
    let now = Utc::now();
    let mut statuses = Vec::new();

    for agent in agents {
        // Only check running agents
        if agent.state != AgentState::Running {
            continue;
        }

        let inactive_secs = (now - agent.last_active).num_seconds();

        // Determine timeout: use agent's autonomous config if set, else default
        let timeout_secs = agent
            .heartbeat_interval_secs
            .map(|i| i * UNRESPONSIVE_MULTIPLIER)
            .unwrap_or(config.default_timeout_secs) as i64;

        let unresponsive = inactive_secs > timeout_secs;

        if unresponsive {
            warn!(
                agent = %agent.name,
                inactive_secs,
                timeout_secs,
                "Agent is unresponsive"
            );
        } else {
            debug!(
                agent = %agent.name,
                inactive_secs,
                "Agent heartbeat OK"
            );
        }

        statuses.push(HeartbeatStatus {
            agent_id: agent.id,
            name: agent.name.clone(),
            inactive_secs,
            unresponsive,
        });
    }

    statuses
}

/// Check if an agent is currently within its quiet hours.
///
/// Quiet hours format: "HH:MM-HH:MM" (24-hour format, UTC).
/// Returns true if the current time falls within the quiet period.
pub fn is_quiet_hours(quiet_hours: &str) -> bool {
    let parts: Vec<&str> = quiet_hours.split('-').collect();
    if parts.len() != 2 {
        return false;
    }

    let now = Utc::now();
    let current_minutes = now.format("%H").to_string().parse::<u32>().unwrap_or(0) * 60
        + now.format("%M").to_string().parse::<u32>().unwrap_or(0);

    let parse_time = |s: &str| -> Option<u32> {
        let hm: Vec<&str> = s.trim().split(':').collect();
        if hm.len() != 2 {
            return None;
        }
        let h = hm[0].parse::<u32>().ok()?;
        let m = hm[1].parse::<u32>().ok()?;
        if h > 23 || m > 59 {
            return None;
        }
        Some(h * 60 + m)
    };

    let start = match parse_time(parts[0]) {
        Some(v) => v,
        None => return false,
    };
    let end = match parse_time(parts[1]) {
        Some(v) => v,
        None => return false,
    };

    if start <= end {
        // Same-day range: e.g., 09:00-17:00
        current_minutes >= start && current_minutes < end
    } else {
        // Cross-midnight: e.g., 22:00-06:00
        current_minutes >= start || current_minutes < end
    }
}

/// Aggregate heartbeat summary.
#[derive(Debug, Clone, Default)]
pub struct HeartbeatSummary {
    /// Total agents checked.
    pub total_checked: usize,
    /// Number of responsive agents.
    pub responsive: usize,
    /// Number of unresponsive agents.
    pub unresponsive: usize,
    /// Details of unresponsive agents.
    pub unresponsive_agents: Vec<HeartbeatStatus>,
}

/// Produce a summary from heartbeat statuses.
pub fn summarize(statuses: &[HeartbeatStatus]) -> HeartbeatSummary {
    let unresponsive_agents: Vec<HeartbeatStatus> = statuses
        .iter()
        .filter(|s| s.unresponsive)
        .cloned()
        .collect();

    HeartbeatSummary {
        total_checked: statuses.len(),
        responsive: statuses.len() - unresponsive_agents.len(),
        unresponsive: unresponsive_agents.len(),
        unresponsive_agents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_hours_parsing() {
        assert!(!is_quiet_hours("invalid"));
        assert!(!is_quiet_hours(""));
        assert!(!is_quiet_hours("25:00-06:00"));
    }

    #[test]
    fn test_quiet_hours_format_valid() {
        let _ = is_quiet_hours("22:00-06:00");
        let _ = is_quiet_hours("00:00-23:59");
        let _ = is_quiet_hours("09:00-17:00");
    }

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.check_interval_secs, 30);
        assert_eq!(config.default_timeout_secs, 60);
    }

    #[test]
    fn test_summarize_empty() {
        let summary = summarize(&[]);
        assert_eq!(summary.total_checked, 0);
        assert_eq!(summary.responsive, 0);
        assert_eq!(summary.unresponsive, 0);
    }

    #[test]
    fn test_summarize_mixed() {
        let statuses = vec![
            HeartbeatStatus {
                agent_id: AgentId::new(),
                name: "agent-1".to_string(),
                inactive_secs: 10,
                unresponsive: false,
            },
            HeartbeatStatus {
                agent_id: AgentId::new(),
                name: "agent-2".to_string(),
                inactive_secs: 120,
                unresponsive: true,
            },
            HeartbeatStatus {
                agent_id: AgentId::new(),
                name: "agent-3".to_string(),
                inactive_secs: 5,
                unresponsive: false,
            },
        ];

        let summary = summarize(&statuses);
        assert_eq!(summary.total_checked, 3);
        assert_eq!(summary.responsive, 2);
        assert_eq!(summary.unresponsive, 1);
        assert_eq!(summary.unresponsive_agents.len(), 1);
        assert_eq!(summary.unresponsive_agents[0].name, "agent-2");
    }

    #[test]
    fn test_check_agents_with_running_agents() {
        let config = HeartbeatConfig::default();
        let agents = vec![
            AgentHeartbeatInfo {
                id: AgentId::new(),
                name: "agent-active".to_string(),
                state: AgentState::Running,
                last_active: Utc::now(),
                heartbeat_interval_secs: None,
            },
            AgentHeartbeatInfo {
                id: AgentId::new(),
                name: "agent-suspended".to_string(),
                state: AgentState::Suspended,
                last_active: Utc::now() - chrono::Duration::hours(1),
                heartbeat_interval_secs: None,
            },
        ];

        let statuses = check_agents(&agents, &config);
        // Only the running agent should be checked
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "agent-active");
        assert!(!statuses[0].unresponsive);
    }
}
