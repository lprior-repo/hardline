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
            fn prop_session_event_serde_roundtrip(variant in 0u8..6u8) {
                let events = [
                    SessionEvent::Activated,
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
            fn prop_session_event_as_str_non_empty(variant in 0u8..6u8) {
                let events = [
                    SessionEvent::Activated,
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
                6,
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

    // =========================================================================
    // Event Ordering During Lifecycle Transitions (ha-weh)
    // =========================================================================

    mod event_ordering_tests {
        use super::*;
        use crate::domain::entities::session::{Active, Completed, Created, Failed, Session};

        /// SessionCreatedEvent captures the session identity at creation time.
        /// Verify it matches the session's id and name.
        #[test]
        fn created_event_matches_session_identity_at_creation() {
            let name = SessionName::parse("ordering-create").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let event = SessionCreatedEvent::new(session.id.as_str().to_string(), name);

            assert_eq!(event.session_id, session.id.as_str());
            assert_eq!(event.session_name.as_str(), session.name.as_str());
            assert!(event.timestamp >= session.created_at);
        }

        /// Happy path: Created → Active → Completed.
        /// SessionCreatedEvent fires before SessionCompletedEvent.
        /// Both carry the same session identity; timestamps are ordered.
        #[test]
        fn created_then_completed_event_ordering() {
            let name = SessionName::parse("order-complete").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let active: Session<Active> = session.activate().expect("activate");
            let completed: Session<Completed> = active.complete().expect("complete");

            let completed_event =
                SessionCompletedEvent::new(completed.id.as_str().to_string(), name);

            // Identity preserved through transitions
            assert_eq!(created_event.session_id, completed_event.session_id);
            assert_eq!(
                created_event.session_name.as_str(),
                completed_event.session_name.as_str()
            );

            // Timestamp ordering: created ≤ completed
            assert!(
                created_event.timestamp <= completed_event.timestamp,
                "created event must fire at or before completed event"
            );
        }

        /// Failure path: Created → Active → Failed.
        /// SessionCreatedEvent fires before SessionFailedEvent.
        /// Failed event includes a reason; timestamps are ordered.
        #[test]
        fn created_then_failed_event_ordering() {
            let name = SessionName::parse("order-fail").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let active: Session<Active> = session.activate().expect("activate");
            let failed: Session<Failed> = active.fail().expect("fail");

            let failed_event = SessionFailedEvent::new(
                failed.id.as_str().to_string(),
                name,
                "sync timeout".into(),
            );

            // Identity preserved through transitions
            assert_eq!(created_event.session_id, failed_event.session_id);
            assert_eq!(
                created_event.session_name.as_str(),
                failed_event.session_name.as_str()
            );

            // Failed event carries reason
            assert_eq!(failed_event.reason, "sync timeout");

            // Timestamp ordering: created ≤ failed
            assert!(
                created_event.timestamp <= failed_event.timestamp,
                "created event must fire at or before failed event"
            );
        }

        /// Failure path from Created state directly (no activation).
        /// SessionCreatedEvent → SessionFailedEvent with identity preservation.
        #[test]
        fn created_then_failed_without_activation() {
            let name = SessionName::parse("order-fail-direct").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let failed: Session<Failed> = session.fail().expect("fail from created");

            let failed_event = SessionFailedEvent::new(
                failed.id.as_str().to_string(),
                name,
                "initialization failed".into(),
            );

            assert_eq!(created_event.session_id, failed_event.session_id);
            assert!(
                created_event.timestamp <= failed_event.timestamp,
                "created event must fire at or before failed event"
            );
        }

        /// Full sync lifecycle: Created → Active → Syncing → Synced → Completed.
        /// Verify SessionCreatedEvent and SessionCompletedEvent bracket the lifecycle.
        #[test]
        fn full_sync_lifecycle_event_ordering() {
            let name = SessionName::parse("order-sync-full").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");

            let completed_event =
                SessionCompletedEvent::new(completed.id.as_str().to_string(), name);

            assert_eq!(created_event.session_id, completed_event.session_id);
            assert!(
                created_event.timestamp <= completed_event.timestamp,
                "created must precede completed in full sync path"
            );
        }

        /// Pause/resume path: Created → Active → Paused → Active → Completed.
        /// Verify events bracket the paused lifecycle correctly.
        #[test]
        fn pause_resume_lifecycle_event_ordering() {
            let name = SessionName::parse("order-pause").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let active = session.activate().expect("activate");
            let paused = active.pause().expect("pause");
            let resumed = paused.resume().expect("resume");
            let completed = resumed.complete().expect("complete");

            let completed_event =
                SessionCompletedEvent::new(completed.id.as_str().to_string(), name);

            assert_eq!(created_event.session_id, completed_event.session_id);
            assert!(
                created_event.timestamp <= completed_event.timestamp,
                "created must precede completed after pause/resume"
            );
        }

        /// Multiple sessions: verify events carry distinct identities.
        /// Each session's events reference only its own id/name.
        #[test]
        fn concurrent_sessions_have_distinct_events() {
            let name_a = SessionName::parse("session-a").expect("valid");
            let name_b = SessionName::parse("session-b").expect("valid");

            let session_a = Session::<Created>::create(name_a.clone()).expect("created a");
            let session_b = Session::<Created>::create(name_b.clone()).expect("created b");

            let event_a =
                SessionCreatedEvent::new(session_a.id.as_str().to_string(), name_a.clone());
            let event_b =
                SessionCreatedEvent::new(session_b.id.as_str().to_string(), name_b.clone());

            // Distinct session IDs
            assert_ne!(event_a.session_id, event_b.session_id);

            // Events reference correct sessions
            assert_eq!(event_a.session_name.as_str(), "session-a");
            assert_eq!(event_b.session_name.as_str(), "session-b");

            // Complete session A, fail session B
            let completed_a = session_a
                .activate()
                .expect("activate a")
                .complete()
                .expect("complete a");
            let failed_b = session_b.fail().expect("fail b");

            let completed_event =
                SessionCompletedEvent::new(completed_a.id.as_str().to_string(), name_a);
            let failed_event = SessionFailedEvent::new(
                failed_b.id.as_str().to_string(),
                SessionName::parse("session-b").expect("valid"),
                "error".into(),
            );

            // Completed event matches session A only
            assert_eq!(completed_event.session_id, event_a.session_id);
            assert_ne!(completed_event.session_id, event_b.session_id);

            // Failed event matches session B only
            assert_eq!(failed_event.session_id, event_b.session_id);
            assert_ne!(failed_event.session_id, event_a.session_id);
        }

        /// SessionCompletedEvent data matches the session at completion time.
        /// Verify the event captures the session identity (id, name) exactly.
        #[test]
        fn completed_event_data_matches_session_at_completion() {
            let name = SessionName::parse("data-match-complete").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");
            let session_id = session.id.as_str().to_string();

            let completed = session
                .activate()
                .expect("activate")
                .complete()
                .expect("complete");

            let event = SessionCompletedEvent::new(completed.id.as_str().to_string(), name);

            assert_eq!(event.session_id, session_id);
            assert_eq!(event.session_id, completed.id.as_str());
            assert_eq!(event.session_name.as_str(), completed.name.as_str());
        }

        /// SessionFailedEvent data matches the session at failure time.
        /// Verify the event captures id, name, and reason exactly.
        #[test]
        fn failed_event_data_matches_session_at_failure() {
            let name = SessionName::parse("data-match-fail").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");
            let session_id = session.id.as_str().to_string();

            let failed = session.activate().expect("activate").fail().expect("fail");

            let event = SessionFailedEvent::new(
                failed.id.as_str().to_string(),
                name,
                "database connection lost".into(),
            );

            assert_eq!(event.session_id, session_id);
            assert_eq!(event.session_id, failed.id.as_str());
            assert_eq!(event.session_name.as_str(), failed.name.as_str());
            assert_eq!(event.reason, "database connection lost");
        }

        /// After restart from Completed, a new CreatedEvent can be constructed.
        /// The restarted session retains the same identity for a new lifecycle.
        #[test]
        fn restarted_session_supports_new_created_event() {
            let name = SessionName::parse("restart-events").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let first_created =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let completed = session
                .activate()
                .expect("activate")
                .complete()
                .expect("complete");
            let restarted = completed.restart().expect("restart");

            let second_created = SessionCreatedEvent::new(restarted.id.as_str().to_string(), name);

            // Same session identity across restart
            assert_eq!(first_created.session_id, second_created.session_id);
            assert_eq!(
                first_created.session_name.as_str(),
                second_created.session_name.as_str()
            );

            // Second created event is at or after first
            assert!(
                second_created.timestamp >= first_created.timestamp,
                "second created event must be at or after first"
            );
        }

        /// After retry from Failed, a new CreatedEvent can be constructed.
        /// The retried session retains identity for a new lifecycle.
        #[test]
        fn retried_session_supports_new_created_event() {
            let name = SessionName::parse("retry-events").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let first_created =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let failed = session.fail().expect("fail");
            let retried = failed.retry().expect("retry");

            let second_created = SessionCreatedEvent::new(retried.id.as_str().to_string(), name);

            assert_eq!(first_created.session_id, second_created.session_id);
            assert!(
                second_created.timestamp >= first_created.timestamp,
                "retry created event must be at or after initial created"
            );
        }

        /// Full lifecycle with both terminal paths:
        /// Session A completes, Session B fails — verify events don't cross-contaminate.
        #[test]
        fn two_sessions_divergent_paths_no_event_cross_contamination() {
            let name_c = SessionName::parse("path-complete").expect("valid");
            let name_f = SessionName::parse("path-fail").expect("valid");

            let session_c = Session::<Created>::create(name_c.clone()).expect("created c");
            let session_f = Session::<Created>::create(name_f.clone()).expect("created f");

            let id_c = session_c.id.as_str().to_string();
            let id_f = session_f.id.as_str().to_string();

            let completed = session_c
                .activate()
                .expect("activate c")
                .complete()
                .expect("complete c");

            let failed = session_f
                .activate()
                .expect("activate f")
                .fail()
                .expect("fail f");

            let completed_evt =
                SessionCompletedEvent::new(completed.id.as_str().to_string(), name_c);
            let failed_evt = SessionFailedEvent::new(
                failed.id.as_str().to_string(),
                name_f,
                "divergent failure".into(),
            );

            // No cross-contamination
            assert_eq!(completed_evt.session_id, id_c);
            assert_eq!(failed_evt.session_id, id_f);
            assert_ne!(completed_evt.session_id, failed_evt.session_id);
        }

        /// Event ordering with sync lifecycle ending in failure.
        /// Created → Active → Syncing → Failed.
        #[test]
        fn sync_then_fail_event_ordering() {
            let name = SessionName::parse("sync-fail-order").expect("valid");
            let session = Session::<Created>::create(name.clone()).expect("created");

            let created_event =
                SessionCreatedEvent::new(session.id.as_str().to_string(), name.clone());

            let active = session.activate().expect("activate");
            let syncing = active.sync().expect("sync");
            let failed = syncing.fail().expect("fail from syncing");

            let failed_event = SessionFailedEvent::new(
                failed.id.as_str().to_string(),
                name,
                "sync corruption".into(),
            );

            assert_eq!(created_event.session_id, failed_event.session_id);
            assert!(
                created_event.timestamp <= failed_event.timestamp,
                "created event must precede failed event even after sync attempt"
            );
        }
    }
}
