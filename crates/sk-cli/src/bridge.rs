use async_trait::async_trait;
use dashmap::DashMap;
use sk_channels::bridge::ChannelBridgeHandle;
use sk_kernel::SovereignKernel;
use sk_types::AgentId;
use std::sync::Arc;
use tracing::{error, info};

pub struct SovereignBridge {
    kernel: Arc<SovereignKernel>,
    sessions: DashMap<AgentId, Arc<tokio::sync::Mutex<sk_types::Session>>>,
}

impl SovereignBridge {
    pub fn new(kernel: Arc<SovereignKernel>) -> Self {
        Self {
            kernel,
            sessions: DashMap::new(),
        }
    }
}

#[async_trait]
impl ChannelBridgeHandle for SovereignBridge {
    async fn send_message(&self, agent_id: AgentId, message: &str) -> Result<String, String> {
        info!("Channel message received for agent {agent_id}: {message}");

        // Retrieve or create session in memory cache
        let session_mutex = self
            .sessions
            .entry(agent_id.clone())
            .or_insert_with(|| {
                // Try to load from SQLite database first
                if let Ok(entries) = self.kernel.memory.sessions.list_for_agent(agent_id.clone()) {
                    if let Some((latest_id, _, _)) = entries.first() {
                        if let Ok(Some(loaded_session)) =
                            self.kernel.memory.sessions.load(*latest_id)
                        {
                            return Arc::new(tokio::sync::Mutex::new(loaded_session));
                        }
                    }
                }
                // Fallback to new session
                Arc::new(tokio::sync::Mutex::new(sk_types::Session::new(
                    agent_id.clone(),
                )))
            })
            .clone();

        let mut session = session_mutex.lock().await;

        let user_msg_lower = message.trim().to_lowercase();
        if user_msg_lower == "approve" || user_msg_lower == "yes" {
            self.kernel.safety.approve_last_for_agent(&agent_id);
        } else if user_msg_lower == "deny" || user_msg_lower == "no" {
            self.kernel.safety.deny_last_for_agent(&agent_id);
        }

        let result = self.kernel.run_agent(&mut session, message).await;

        match result {
            Ok(result) => Ok(result.response),
            Err(e) => {
                error!("Agent loop failed: {e}");
                Err(e.to_string())
            }
        }
    }

    async fn find_agent_by_name(&self, name: &str) -> Result<Option<AgentId>, String> {
        // Create a deterministic UUID so the same agent name always gets the same ID and session
        let hash = md5::compute(name);
        Ok(Some(AgentId(uuid::Uuid::from_bytes(*hash))))
    }

    async fn list_agents(&self) -> Result<Vec<(AgentId, String)>, String> {
        let name = "Sovereign Default";
        let hash = md5::compute(name);
        Ok(vec![(
            AgentId(uuid::Uuid::from_bytes(*hash)),
            name.to_string(),
        )])
    }

    async fn spawn_agent_by_name(&self, manifest_name: &str) -> Result<AgentId, String> {
        let hash = md5::compute(manifest_name);
        Ok(AgentId(uuid::Uuid::from_bytes(*hash)))
    }

    async fn record_delivery(
        &self,
        agent_id: AgentId,
        channel: &str,
        recipient: &str,
        _success: bool,
        _error: Option<&str>,
    ) {
        let value = serde_json::json!({
            "channel": channel,
            "to": recipient
        });
        if let Err(e) = self
            .kernel
            .memory
            .structured
            .set(agent_id, "last_channel", value)
        {
            error!("Failed to save last_channel for agent {agent_id}: {e}");
        }
    }
}
