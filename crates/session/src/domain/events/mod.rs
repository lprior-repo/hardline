use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::value_objects::SessionName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEvent {
    Activated,
    CommittingEffect,
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
            Self::CommittingEffect => "committing_effect",
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
        // Just verify the timestamp is recent (within the last second)
        let now = Utc::now();
        let diff = (now - event.timestamp).num_nanoseconds().unwrap_or(0);
        assert!(diff.abs() < 1_000_000_000, "timestamp should be recent");
    }

    #[test]
    fn test_event_serialization() {
        let event = SessionEvent::Activated;
        let json = serialize_event(&event).expect("serialize");
        let parsed = deserialize_event(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }

    // =========================================================================
    // SessionEvent Extended Tests
    // =========================================================================

    #[test]
    fn session_event_all_variants_serialize_and_deserialize() {
            let events = [
                SessionEvent::Activated,
                SessionEvent::CommittingEffect,
                SessionEvent::Syncing,
                SessionEvent::Synced,
                SessionEvent::Paused,
                SessionEvent::Completed,
                SessionEvent::Failed,
            ];
            for event in &events {
                let json = serialize_event(event).expect("serialize");
                let parsed = deserialize_event(&json).expect("deserialize");
                assert_eq!(event, &parsed);
            }
    }

    #[test]
    fn session_event_as_str() {
        assert_eq!(SessionEvent::Activated.as_str(), "activated");
        assert_eq!(SessionEvent::CommittingEffect.as_str(), "committing_effect");
        assert_eq!(SessionEvent::Syncing.as_str(), "syncing");
        assert_eq!(SessionEvent::Synced.as_str(), "synced");
        assert_eq!(SessionEvent::Paused.as_str(), "paused");
        assert_eq!(SessionEvent::Completed.as_str(), "completed");
        assert_eq!(SessionEvent::Failed.as_str(), "failed");
    }

    #[test]
    fn session_event_serde_roundtrip_all_variants() {
        let events = [
            SessionEvent::Activated,
            SessionEvent::CommittingEffect,
            SessionEvent::Syncing,
            SessionEvent::Synced,
            SessionEvent::Paused,
            SessionEvent::Completed,
            SessionEvent::Failed,
        ];
        for event in &events {
            let json = serde_json::to_string(event).expect("serialize");
            let parsed: SessionEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(event, &parsed);
        }
    }

    // =========================================================================
    // SessionCreatedEvent Tests
    // =========================================================================

    #[test]
    fn session_created_event_fields() {
        let name = SessionName::parse("test-event").expect("valid");
        let event = SessionCreatedEvent::new("session-abc".into(), name);
        assert_eq!(event.session_id, "session-abc");
        assert_eq!(event.session_name.as_str(), "test-event");
    }

    #[test]
    fn session_created_event_serde_roundtrip() {
        let name = SessionName::parse("serde-test").expect("valid");
        let event = SessionCreatedEvent::new("s-1".into(), name);
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: SessionCreatedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }

    // =========================================================================
    // SessionCompletedEvent Tests
    // =========================================================================

    #[test]
    fn session_completed_event_fields() {
        let name = SessionName::parse("done").expect("valid");
        let event = SessionCompletedEvent::new("s-done".into(), name);
        assert_eq!(event.session_id, "s-done");
    }

    #[test]
    fn session_completed_event_serde_roundtrip() {
        let name = SessionName::parse("completed").expect("valid");
        let event = SessionCompletedEvent::new("s-2".into(), name);
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: SessionCompletedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }

    // =========================================================================
    // SessionFailedEvent Tests
    // =========================================================================

    #[test]
    fn session_failed_event_fields() {
        let name = SessionName::parse("fail-event").expect("valid");
        let event = SessionFailedEvent::new("s-fail".into(), name, "timeout".into());
        assert_eq!(event.session_id, "s-fail");
        assert_eq!(event.reason, "timeout");
    }

    #[test]
    fn session_failed_event_serde_roundtrip() {
        let name = SessionName::parse("failed").expect("valid");
        let event = SessionFailedEvent::new("s-3".into(), name, "crash".into());
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: SessionFailedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }

    // =========================================================================
    // Event Property Tests
    // =========================================================================

    mod event_proptests {
        use super::*;
        use proptest::proptest;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            /// SessionEvent serialize/deserialize roundtrip for all variants
            #[test]
            fn prop_session_event_serde_roundtrip(variant in 0u8..7u8) {
                let events = [
                    SessionEvent::Activated,
                    SessionEvent::CommittingEffect,
                    SessionEvent::Syncing,
                    SessionEvent::Synced,
                    SessionEvent::Paused,
                    SessionEvent::Completed,
                    SessionEvent::Failed,
                ];
                let event = events[variant as usize];
                let json = serde_json::to_string(&event).unwrap();
                let parsed: SessionEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, parsed);
            }

            /// SessionEvent as_str returns non-empty string for all variants
            #[test]
            fn prop_session_event_as_str_non_empty(variant in 0u8..7u8) {
                let events = [
                    SessionEvent::Activated,
                    SessionEvent::CommittingEffect,
                    SessionEvent::Syncing,
                    SessionEvent::Synced,
                    SessionEvent::Paused,
                    SessionEvent::Completed,
                    SessionEvent::Failed,
                ];
                let event = events[variant as usize];
                let s = event.as_str();
                prop_assert!(!s.is_empty());
                prop_assert!(s.is_ascii());
            }

            /// SessionCreatedEvent serde roundtrip with various session IDs
            #[test]
            fn prop_created_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,30}",
                name_str in "[a-z][a-z0-9_-]{1,20}"
            ) {
                let name = SessionName::parse(name_str).unwrap();
                let event = SessionCreatedEvent::new(id, name);
                let json = serde_json::to_string(&event).unwrap();
                let parsed: SessionCreatedEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, parsed);
            }

            /// SessionCompletedEvent serde roundtrip with various session IDs
            #[test]
            fn prop_completed_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,30}",
                name_str in "[a-z][a-z0-9_-]{1,20}"
            ) {
                let name = SessionName::parse(name_str).unwrap();
                let event = SessionCompletedEvent::new(id, name);
                let json = serde_json::to_string(&event).unwrap();
                let parsed: SessionCompletedEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, parsed);
            }

            /// SessionFailedEvent serde roundtrip with various data
            #[test]
            fn prop_failed_event_roundtrip(
                id in "[a-zA-Z0-9_-]{1,30}",
                name_str in "[a-z][a-z0-9_-]{1,20}",
                reason in "[a-zA-Z ]{1,50}"
            ) {
                let name = SessionName::parse(name_str).unwrap();
                let event = SessionFailedEvent::new(id, name, reason);
                let json = serde_json::to_string(&event).unwrap();
                let parsed: SessionFailedEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(event, parsed);
            }
        }
    }

    // =========================================================================
    // Event Edge Cases
    // =========================================================================

    mod event_edge_tests {
        use super::*;

        #[test]
        fn session_created_event_with_empty_id() {
            let name = SessionName::parse("test").expect("valid");
            let event = SessionCreatedEvent::new(String::new(), name);
            assert!(event.session_id.is_empty());
        }

        #[test]
        fn session_failed_event_with_empty_reason() {
            let name = SessionName::parse("test").expect("valid");
            let event = SessionFailedEvent::new("s-1".into(), name, String::new());
            assert!(event.reason.is_empty());
        }

        #[test]
        fn session_failed_event_with_long_reason() {
            let name = SessionName::parse("test").expect("valid");
            let long_reason = "x".repeat(1000);
            let event = SessionFailedEvent::new("s-1".into(), name, long_reason.clone());
            assert_eq!(event.reason.len(), 1000);
        }

        #[test]
        fn session_event_all_variants_distinct_as_str() {
            let events = [
                (SessionEvent::Activated, "activated"),
                (SessionEvent::CommittingEffect, "committing_effect"),
                (SessionEvent::Syncing, "syncing"),
                (SessionEvent::Synced, "synced"),
                (SessionEvent::Paused, "paused"),
                (SessionEvent::Completed, "completed"),
                (SessionEvent::Failed, "failed"),
            ];
            for (event, expected) in events {
                assert_eq!(event.as_str(), expected);
            }
        }

        #[test]
        fn session_event_all_variants_distinct_json() {
            let events = [
                SessionEvent::Activated,
                SessionEvent::CommittingEffect,
                SessionEvent::Syncing,
                SessionEvent::Synced,
                SessionEvent::Paused,
                SessionEvent::Completed,
                SessionEvent::Failed,
            ];
            let mut jsons: Vec<String> = events
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect();
            jsons.sort();
            jsons.dedup();
            assert_eq!(
                jsons.len(),
                7,
                "All events should have distinct JSON representations"
            );
        }

        #[test]
        fn session_created_event_timestamp_within_bounds() {
            let name = SessionName::parse("bounds").expect("valid");
            let before = chrono::Utc::now();
            let event = SessionCreatedEvent::new("s-bounds".into(), name);
            let after = chrono::Utc::now();
            assert!(event.timestamp >= before);
            assert!(event.timestamp <= after);
        }

        #[test]
        fn session_completed_event_timestamp_within_bounds() {
            let name = SessionName::parse("bounds-c").expect("valid");
            let before = chrono::Utc::now();
            let event = SessionCompletedEvent::new("s-c".into(), name);
            let after = chrono::Utc::now();
            assert!(event.timestamp >= before);
            assert!(event.timestamp <= after);
        }

        #[test]
        fn session_failed_event_timestamp_within_bounds() {
            let name = SessionName::parse("bounds-f").expect("valid");
            let before = chrono::Utc::now();
            let event = SessionFailedEvent::new("s-f".into(), name, "err".into());
            let after = chrono::Utc::now();
            assert!(event.timestamp >= before);
            assert!(event.timestamp <= after);
        }

        #[test]
        fn deserialize_invalid_event_json_fails() {
            let result: Result<SessionEvent, _> = serde_json::from_str("\"invalid\"");
            assert!(result.is_err());
        }

        #[test]
        fn serialize_event_returns_valid_json() {
            for event in [
                SessionEvent::Activated,
                SessionEvent::CommittingEffect,
                SessionEvent::Syncing,
                SessionEvent::Synced,
                SessionEvent::Paused,
                SessionEvent::Completed,
                SessionEvent::Failed,
            ] {
                let json = serialize_event(&event).expect("serialize");
                assert!(json.starts_with('"'));
                assert!(json.ends_with('"'));
            }
        }
    }
}
