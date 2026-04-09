//! Internal pub/sub event bus for kernel-internal communication.

use tokio::sync::broadcast;

/// Event types that can be published on the bus.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    AgentStarted { agent_id: String },
    AgentStopped { agent_id: String },
    ToolCalled { agent_id: String, tool_name: String },
    MemoryStored { agent_id: String, memory_id: String },
    McpServerConnected { server_name: String },
    McpServerDisconnected { server_name: String },
    Presence { active_agents: Vec<String> },
    Error { message: String },
    Broadcast { from: String, message: String },
}

/// Kernel event bus for internal pub/sub.
pub struct EventBus {
    sender: broadcast::Sender<KernelEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: KernelEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<KernelEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
