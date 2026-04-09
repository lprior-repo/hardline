//! Data types for the events command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the events command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct EventsOptions {
    /// Filter by session name.
    pub session: Option<String>,
    /// Filter by event type.
    pub event_type: Option<String>,
    /// Follow mode (stream events).
    pub follow: bool,
    /// Maximum number of events to return.
    pub limit: Option<usize>,
    /// Only show events after this timestamp (ISO 8601).
    pub since: Option<String>,
}

/// Event types in the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    SessionCreated,
    SessionRemoved,
    SessionFocused,
    SessionMerged,
    SessionAborted,
    SessionSynced,
    AgentRegistered,
    AgentUnregistered,
    AgentHeartbeat,
    LockAcquired,
    LockReleased,
    CheckpointCreated,
    CheckpointRestored,
    BeadStatusChanged,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::SessionCreated => "session_created",
            Self::SessionRemoved => "session_removed",
            Self::SessionFocused => "session_focused",
            Self::SessionMerged => "session_merged",
            Self::SessionAborted => "session_aborted",
            Self::SessionSynced => "session_synced",
            Self::AgentRegistered => "agent_registered",
            Self::AgentUnregistered => "agent_unregistered",
            Self::AgentHeartbeat => "agent_heartbeat",
            Self::LockAcquired => "lock_acquired",
            Self::LockReleased => "lock_released",
            Self::CheckpointCreated => "checkpoint_created",
            Self::CheckpointRestored => "checkpoint_restored",
            Self::BeadStatusChanged => "bead_status_changed",
        };
        write!(f, "{s}")
    }
}

/// A single event entry from the event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    /// Event ID.
    pub id: String,
    /// Event type.
    #[allow(clippy::struct_field_names)]
    pub event_type: EventType,
    /// Timestamp in ISO 8601 format (e.g. `"2025-01-15T12:00:00Z"`).
    /// Used as a bare string for serialization compatibility with the event log;
    /// consumers should parse into a datetime type if arithmetic is needed.
    pub timestamp: String,
    /// Session name if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Agent ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Event-specific data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Human-readable message.
    pub message: String,
}

/// Output of the events command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsOutput {
    /// List of events.
    pub events: Vec<EventEntry>,
    /// Total count (may be more than returned).
    pub total: usize,
    /// Whether there are more events.
    pub has_more: bool,
    /// Cursor for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Pure helper: check if a given event type string matches a filter.
///
/// Supports exact match (e.g. "session_created") and category prefix
/// match (e.g. "session" matches "session_created", "session_removed", etc.).
#[must_use]
pub fn event_type_matches(filter: Option<&str>, event_type: &EventType) -> bool {
    let Some(raw_filter) = filter else {
        return true;
    };

    let normalized_filter = raw_filter.trim().to_lowercase().replace('-', "_");
    let canonical = event_type.to_string();

    if canonical == normalized_filter {
        return true;
    }

    matches!(
        normalized_filter.as_str(),
        "session" | "agent" | "lock" | "checkpoint" | "bead"
    ) && canonical.starts_with(&format!("{normalized_filter}_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_display_is_snake_case() {
        assert_eq!(EventType::SessionCreated.to_string(), "session_created");
        assert_eq!(EventType::AgentHeartbeat.to_string(), "agent_heartbeat");
        assert_eq!(
            EventType::BeadStatusChanged.to_string(),
            "bead_status_changed"
        );
    }

    #[test]
    fn event_entry_serialization_roundtrip() {
        let entry = EventEntry {
            id: "evt-1".to_string(),
            event_type: EventType::SessionCreated,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: Some("test".to_string()),
            agent_id: None,
            data: None,
            message: "Created session".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let deserialized: EventEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, "evt-1");
        assert_eq!(deserialized.event_type, EventType::SessionCreated);
    }

    #[test]
    fn event_entry_optional_fields_omitted() {
        let entry = EventEntry {
            id: "evt-2".to_string(),
            event_type: EventType::AgentHeartbeat,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: None,
            agent_id: None,
            data: None,
            message: "Heartbeat".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(!json.contains("session"));
        assert!(!json.contains("agent_id"));
        assert!(!json.contains("data"));
    }

    #[test]
    fn events_output_serialization() {
        let output = EventsOutput {
            events: vec![],
            total: 0,
            has_more: false,
            cursor: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn events_output_with_cursor() {
        let output = EventsOutput {
            events: vec![],
            total: 100,
            has_more: true,
            cursor: Some("cursor-abc".to_string()),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("cursor-abc"));
        assert!(json.contains("\"has_more\":true"));
    }

    #[test]
    fn event_type_matches_none_filter_matches_all() {
        assert!(event_type_matches(None, &EventType::SessionCreated));
        assert!(event_type_matches(None, &EventType::LockReleased));
    }

    #[test]
    fn event_type_matches_exact_match() {
        assert!(event_type_matches(
            Some("session_created"),
            &EventType::SessionCreated
        ));
        assert!(!event_type_matches(
            Some("session_created"),
            &EventType::SessionRemoved
        ));
    }

    #[test]
    fn event_type_matches_category_prefix() {
        assert!(event_type_matches(
            Some("session"),
            &EventType::SessionCreated
        ));
        assert!(event_type_matches(
            Some("session"),
            &EventType::SessionRemoved
        ));
        assert!(event_type_matches(
            Some("agent"),
            &EventType::AgentHeartbeat
        ));
        assert!(event_type_matches(Some("lock"), &EventType::LockAcquired));
        assert!(event_type_matches(
            Some("checkpoint"),
            &EventType::CheckpointCreated
        ));
        assert!(event_type_matches(
            Some("bead"),
            &EventType::BeadStatusChanged
        ));
    }

    #[test]
    fn event_type_matches_case_insensitive() {
        assert!(event_type_matches(
            Some("SESSION_CREATED"),
            &EventType::SessionCreated
        ));
        assert!(event_type_matches(
            Some("Session-Created"),
            &EventType::SessionCreated
        ));
    }

    #[test]
    fn event_type_matches_no_false_positives() {
        assert!(!event_type_matches(
            Some("nonexistent"),
            &EventType::SessionCreated
        ));
    }

    #[test]
    fn event_entry_with_data() {
        use serde_json::json;

        let entry = EventEntry {
            id: "evt-3".to_string(),
            event_type: EventType::BeadStatusChanged,
            timestamp: "2025-01-15T12:00:00Z".to_string(),
            session: Some("task".to_string()),
            agent_id: None,
            data: Some(json!({
                "old_status": "in_progress",
                "new_status": "completed"
            })),
            message: "Status changed".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("old_status"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn event_type_enum_coverage() {
        let all_types = [
            EventType::SessionCreated,
            EventType::SessionRemoved,
            EventType::SessionFocused,
            EventType::SessionMerged,
            EventType::SessionAborted,
            EventType::SessionSynced,
            EventType::AgentRegistered,
            EventType::AgentUnregistered,
            EventType::AgentHeartbeat,
            EventType::LockAcquired,
            EventType::LockReleased,
            EventType::CheckpointCreated,
            EventType::CheckpointRestored,
            EventType::BeadStatusChanged,
        ];
        assert_eq!(all_types.len(), 14, "All event types should be covered");

        for et in &all_types {
            let name = et.to_string();
            assert!(!name.is_empty());
            assert!(name.contains('_'), "{} should be snake_case", name);
            assert_eq!(name, name.to_lowercase(), "{} should be lowercase", name);
        }
    }
}
