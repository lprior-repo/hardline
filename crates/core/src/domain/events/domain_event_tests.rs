//! Tests for domain events
//!
//! These tests verify the serialization and behavior of domain events.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use crate::domain::events::{
        deserialize_event, deserialize_event_bytes, serialize_event, serialize_event_bytes,
        DomainEvent,
    };
    use crate::domain::identifiers::{BeadId, SessionName, WorkspaceName};

    #[test]
    fn test_session_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_created");
        assert_eq!(event.timestamp(), &timestamp);

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_session_completed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_completed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_completed");
    }

    #[test]
    fn test_session_failed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_failed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            "Out of memory".to_string(),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_failed");

        // Verify the event contains the failure reason
        if let DomainEvent::SessionFailed(e) = &event {
            assert_eq!(e.reason, "Out of memory");
        } else {
            panic!("Expected SessionFailed event");
        }
    }

    #[test]
    fn test_workspace_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_created(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_created");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_workspace_removed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_removed(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_removed");
    }

    #[test]
    fn test_bead_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::bead_created(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            "Fix the bug".to_string(),
            Some("Critical issue".to_string()),
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_created");

        // Verify the event contains the bead data
        if let DomainEvent::BeadCreated(e) = &event {
            assert_eq!(e.title, "Fix the bug");
            assert_eq!(e.description, Some("Critical issue".to_string()));
        } else {
            panic!("Expected BeadCreated event");
        }
    }

    #[test]
    fn test_bead_closed_event() {
        let timestamp = Utc::now();
        let closed_at = timestamp;

        let event = DomainEvent::bead_closed(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            closed_at,
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_closed");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let events = vec![
            DomainEvent::session_created(
                "session-123".to_string(),
                SessionName::parse("my-session").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("my-workspace").expect("valid"),
                PathBuf::from("/tmp/workspace"),
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc123").expect("valid"),
                "Test bead".to_string(),
                None,
                Utc::now(),
            ),
        ];

        for event in events {
            // Test JSON serialization
            let json = serialize_event(&event).expect("serialization failed");
            let deserialized = deserialize_event(&json).expect("deserialization failed");
            assert_eq!(event, deserialized);

            // Test bytes serialization
            let bytes = serialize_event_bytes(&event).expect("serialization failed");
            let deserialized_bytes =
                deserialize_event_bytes(&bytes).expect("deserialization failed");
            assert_eq!(event, deserialized_bytes);
        }
    }

    #[test]
    fn test_all_event_types_have_unique_types() {
        let events = [
            DomainEvent::session_created(
                "s1".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_completed(
                "s2".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_failed(
                "s3".to_string(),
                SessionName::parse("s").expect("valid"),
                "error".to_string(),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::workspace_removed(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc").expect("valid"),
                "t".to_string(),
                None,
                Utc::now(),
            ),
            DomainEvent::bead_closed(
                BeadId::parse("bd-abc").expect("valid"),
                Utc::now(),
                Utc::now(),
            ),
        ];

        let event_types: Vec<&str> = events.iter().map(DomainEvent::event_type).collect();

        // Check that all event types are unique
        let unique_types: std::collections::HashSet<_> = event_types.iter().collect();
        assert_eq!(
            unique_types.len(),
            event_types.len(),
            "Event types should be unique"
        );
    }
}
