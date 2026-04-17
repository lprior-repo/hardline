//! Events emitted by the agent registry

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AgentId, Capability};

/// Events emitted by the agent registry for observability.
///
/// These events enable monitoring, logging, and auditing of agent lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum AgentEvent {
    Registered {
        agent_id: AgentId,
        name: String,
    },
    BecameActive {
        agent_id: AgentId,
    },
    BecameIdle {
        agent_id: AgentId,
    },
    Disconnected {
        agent_id: AgentId,
    },
    TimedOut {
        agent_id: AgentId,
    },
    HeartbeatReceived {
        agent_id: AgentId,
    },
    CapabilitiesUpdated {
        agent_id: AgentId,
        capabilities: Vec<Capability>,
    },
}

impl AgentEvent {
    /// Get the agent ID for this event
    #[must_use]
    pub fn agent_id(&self) -> &AgentId {
        match self {
            Self::Registered { agent_id, .. } => agent_id,
            Self::BecameActive { agent_id } => agent_id,
            Self::BecameIdle { agent_id } => agent_id,
            Self::Disconnected { agent_id } => agent_id,
            Self::TimedOut { agent_id } => agent_id,
            Self::HeartbeatReceived { agent_id } => agent_id,
            Self::CapabilitiesUpdated { agent_id, .. } => agent_id,
        }
    }
}
