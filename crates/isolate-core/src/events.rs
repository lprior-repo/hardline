//! Event types for coordination

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Entry added to queue
    QueueEntryAdded,
    /// Entry removed from queue
    QueueEntryRemoved,
    /// Entry status changed
    QueueEntryStatusChanged,
    /// Agent registered
    AgentRegistered,
    /// Agent unregistered
    AgentUnregistered,
    /// Agent heartbeat received
    AgentHeartbeat,
    /// Lock acquired
    LockAcquired,
    /// Lock released,
    LockReleased,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// An event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: EventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Related session
    pub session: Option<String>,
    /// Related agent
    pub agent_id: Option<String>,
    /// Event data
    pub data: Option<serde_json::Value>,
    /// Human-readable message
    pub message: String,
}

impl Event {
    /// Create a new event
    #[must_use]
    pub fn new(event_type: EventType, message: String) -> Self {
        Self {
            id: uuid(),
            event_type,
            timestamp: Utc::now(),
            session: None,
            agent_id: None,
            data: None,
            message,
        }
    }

    /// Set the session for this event
    #[must_use]
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }

    /// Set the agent ID for this event
    #[must_use]
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the data for this event
    #[must_use]
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Generate a simple UUID-like string
fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{duration:x}")
}
