use sk_memory::MemorySubstrate;
use sk_types::agent::AgentId;
use sk_types::Message;
use std::sync::Arc;
use tracing::debug;

/// Manages message routing between agents by directly modifying their persistent sessions.
pub struct InterAgentBus {
    memory: Arc<MemorySubstrate>,
}

impl InterAgentBus {
    pub fn new(memory: Arc<MemorySubstrate>) -> Self {
        Self { memory }
    }

    /// Send a message from one agent to another.
    /// This loads the target agent's session, appends the message, and saves it.
    /// The next time the target agent is awoken, it will see this message.
    pub fn send(
        &self,
        from: Option<&AgentId>,
        to: &AgentId,
        message: String,
    ) -> Result<(), String> {
        let sender_name = match from {
            Some(id) => id.to_string(),
            None => "System".to_string(),
        };

        let formatted_message = format!("[Incoming Message from {}]:\n{}", sender_name, message);

        // Load the latest session for the target agent from memory
        let mut session = if let Ok(sessions) = self.memory.sessions.list_for_agent(*to) {
            if let Some((session_id, _, _)) = sessions.first() {
                self.memory
                    .sessions
                    .load(*session_id)
                    .map_err(|e| format!("Failed to load target session: {}", e))?
                    .unwrap_or_else(|| sk_types::Session::new(*to))
            } else {
                debug!(
                    target = "inter-agent",
                    "Target agent {} has no sessions yet. Creating.", to
                );
                sk_types::Session::new(*to)
            }
        } else {
            sk_types::Session::new(*to)
        };

        // Append the message as a user message (so the agent responds to it next time it acts)
        session.push_message(Message::user(formatted_message));

        // Save back to memory
        self.memory
            .sessions
            .save(&session)
            .map_err(|e| format!("Failed to save target session {}: {}", to, e))?;

        debug!(
            target = "inter-agent",
            "Message from {} routed to {}", sender_name, to
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sk_memory::MemorySubstrate;
    use sk_types::AgentId;

    #[test]
    fn test_bus_routing() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let bus = InterAgentBus::new(Arc::new(substrate));

        let agent_a = AgentId::new();
        let agent_b = AgentId::new();

        // Agent A sends message to Agent B
        bus.send(Some(&agent_a), &agent_b, "Hello from A".to_string())
            .unwrap();

        // Verify Agent B's session contains the message
        let sessions = bus.memory.sessions.list_for_agent(agent_b).unwrap();
        assert!(!sessions.is_empty());
        let (session_id, _, _) = &sessions[0];
        let session = bus.memory.sessions.load(*session_id).unwrap().unwrap();

        let last_msg = session.messages.last().unwrap();
        assert!(format!("{:?}", last_msg.content).contains("Hello from A"));
    }
}
