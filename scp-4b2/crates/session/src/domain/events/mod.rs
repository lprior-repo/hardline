use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::value_objects::SessionName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEvent {
    Activated,
    Syncing,
    Synced,
    Paused,
    Completed,
    Failed,
}

impl SessionEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Activated => "activated",
            Self::Syncing => "syncing",
            Self::Synced => "synced",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCreatedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub timestamp: DateTime<Utc>,
}

impl SessionCreatedEvent {
    pub fn new(session_id: String, session_name: SessionName) -> Self {
        Self {
            session_id,
            session_name,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCompletedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub timestamp: DateTime<Utc>,
}

impl SessionCompletedEvent {
    pub fn new(session_id: String, session_name: SessionName) -> Self {
        Self {
            session_id,
            session_name,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFailedEvent {
    pub session_id: String,
    pub session_name: SessionName,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

impl SessionFailedEvent {
    pub fn new(session_id: String, session_name: SessionName, reason: String) -> Self {
        Self {
            session_id,
            session_name,
            reason,
            timestamp: Utc::now(),
        }
    }
}

pub fn serialize_event(event: &SessionEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(event)
}

pub fn deserialize_event(json: &str) -> Result<SessionEvent, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_created_event() {
        let name = SessionName::parse("test").expect("valid");
        let event = SessionCreatedEvent::new("session-1".into(), name);
        assert_eq!(event.timestamp, Utc::now());
    }

    #[test]
    fn test_event_serialization() {
        let event = SessionEvent::Activated;
        let json = serialize_event(&event).expect("serialize");
        let parsed = deserialize_event(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }
}
