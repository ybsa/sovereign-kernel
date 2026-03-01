//! Agent router — routes incoming channel messages to the correct agent.

use crate::types::ChannelType;
use dashmap::DashMap;
use sk_types::agent::AgentId;

/// Context for evaluating binding match rules against incoming messages.
#[derive(Debug, Default)]
pub struct BindingContext {
    /// Channel type string (e.g., "telegram", "discord").
    pub channel: String,
    /// Account/bot ID within the channel.
    pub account_id: Option<String>,
    /// Peer/user ID (platform_user_id).
    pub peer_id: String,
    /// Guild/server ID.
    pub guild_id: Option<String>,
    /// User's roles.
    pub roles: Vec<String>,
}

/// Routes incoming messages to the correct agent.
///
/// Routing priority: direct routes > user defaults > system default.
pub struct AgentRouter {
    /// Default agent per user (keyed by openfang_user or platform_id).
    user_defaults: DashMap<String, AgentId>,
    /// Direct routes: (channel_type_key, platform_user_id) -> AgentId.
    direct_routes: DashMap<(String, String), AgentId>,
    /// System-wide default agent.
    default_agent: Option<AgentId>,
}

impl AgentRouter {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            user_defaults: DashMap::new(),
            direct_routes: DashMap::new(),
            default_agent: None,
        }
    }

    /// Set the system-wide default agent.
    pub fn set_default(&mut self, agent_id: AgentId) {
        self.default_agent = Some(agent_id);
    }

    /// Set a user's default agent.
    pub fn set_user_default(&self, user_key: String, agent_id: AgentId) {
        self.user_defaults.insert(user_key, agent_id);
    }

    /// Set a direct route for a specific (channel, user) pair.
    pub fn set_direct_route(
        &self,
        channel_key: String,
        platform_user_id: String,
        agent_id: AgentId,
    ) {
        self.direct_routes
            .insert((channel_key, platform_user_id), agent_id);
    }

    /// Resolve which agent should handle a message.
    ///
    /// Priority: direct route > user default > system default.
    pub fn resolve(
        &self,
        channel_type: &ChannelType,
        platform_user_id: &str,
        user_key: Option<&str>,
    ) -> Option<AgentId> {
        let channel_key = format!("{channel_type:?}");

        // 1. Check direct routes
        if let Some(agent) = self
            .direct_routes
            .get(&(channel_key, platform_user_id.to_string()))
        {
            return Some(*agent);
        }

        // 2. Check user defaults
        if let Some(key) = user_key {
            if let Some(agent) = self.user_defaults.get(key) {
                return Some(*agent);
            }
        }
        // Also check by platform_user_id
        if let Some(agent) = self.user_defaults.get(platform_user_id) {
            return Some(*agent);
        }

        // 3. System default
        self.default_agent
    }

    /// Check if a peer has broadcast routing configured.
    pub fn has_broadcast(&self, _peer_id: &str) -> bool {
        false
    }

    pub fn resolve_broadcast(&self, _peer_id: &str) -> Vec<(String, Option<AgentId>)> {
        Vec::new()
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_priority() {
        let mut router = AgentRouter::new();
        let default_agent = AgentId::new();
        let user_agent = AgentId::new();
        let direct_agent = AgentId::new();

        router.set_default(default_agent);
        router.set_user_default("alice".to_string(), user_agent);
        router.set_direct_route("Telegram".to_string(), "tg_123".to_string(), direct_agent);

        // Direct route wins
        let resolved = router.resolve(&ChannelType::Telegram, "tg_123", Some("alice"));
        assert_eq!(resolved, Some(direct_agent));

        // User default for non-direct-routed user
        let resolved = router.resolve(&ChannelType::WhatsApp, "wa_456", Some("alice"));
        assert_eq!(resolved, Some(user_agent));

        // System default for unknown user
        let resolved = router.resolve(&ChannelType::Discord, "dc_789", None);
        assert_eq!(resolved, Some(default_agent));
    }

    #[test]
    fn test_no_route() {
        let router = AgentRouter::new();
        let resolved = router.resolve(&ChannelType::CLI, "local", None);
        assert_eq!(resolved, None);
    }
}
