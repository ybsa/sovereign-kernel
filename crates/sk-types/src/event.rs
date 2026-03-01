//! Event types for the internal event bus.
//!
//! All inter-agent and system communication flows through events.

use crate::agent::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

mod duration_ms {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(dur: &Option<Duration>, s: S) -> Result<S::Ok, S::Error> {
        match dur {
            Some(d) => d.as_millis().serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
        let opt: Option<u64> = Option::deserialize(d)?;
        Ok(opt.map(Duration::from_millis))
    }
}

/// Unique identifier for an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Where an event is directed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EventTarget {
    Agent(AgentId),
    Broadcast,
    Pattern(String),
    System,
}

/// The payload of an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    Message(AgentMessage),
    ToolResult(ToolOutput),
    MemoryUpdate(MemoryDelta),
    Lifecycle(LifecycleEvent),
    Network(NetworkEvent),
    System(SystemEvent),
    Custom(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub role: MessageRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Agent,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_id: String,
    pub tool_use_id: String,
    pub content: String,
    pub success: bool,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDelta {
    pub operation: MemoryOperation,
    pub key: String,
    pub agent_id: AgentId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOperation {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum LifecycleEvent {
    Spawned { agent_id: AgentId, name: String },
    Started { agent_id: AgentId },
    Suspended { agent_id: AgentId },
    Resumed { agent_id: AgentId },
    Terminated { agent_id: AgentId, reason: String },
    Crashed { agent_id: AgentId, error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum NetworkEvent {
    PeerConnected {
        peer_id: String,
    },
    PeerDisconnected {
        peer_id: String,
    },
    MessageReceived {
        from_peer: String,
        from_agent: String,
    },
    DiscoveryResult {
        service: String,
        providers: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum SystemEvent {
    KernelStarted,
    KernelStopping,
    QuotaWarning {
        agent_id: AgentId,
        resource: String,
        usage_percent: f32,
    },
    HealthCheck {
        status: String,
    },
    QuotaEnforced {
        agent_id: AgentId,
        spent: f64,
        limit: f64,
    },
    ModelRouted {
        agent_id: AgentId,
        complexity: String,
        model: String,
    },
    UserAction {
        user_id: String,
        action: String,
        result: String,
    },
    HealthCheckFailed {
        agent_id: AgentId,
        unresponsive_secs: u64,
    },
}

/// A complete event in the event system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub source: AgentId,
    pub target: EventTarget,
    pub payload: EventPayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<EventId>,
    #[serde(with = "duration_ms")]
    pub ttl: Option<Duration>,
}

impl Event {
    pub fn new(source: AgentId, target: EventTarget, payload: EventPayload) -> Self {
        Self {
            id: EventId::new(),
            source,
            target,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
            ttl: None,
        }
    }

    pub fn with_correlation(mut self, correlation_id: EventId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }
}
