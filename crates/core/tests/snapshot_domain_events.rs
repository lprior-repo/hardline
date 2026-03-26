//! Snapshot tests for domain event JSON serialization.
//!
//! These tests verify that domain events serialize correctly to JSON
//! for event sourcing, logging, and API responses.

use chrono::{DateTime, Utc};
use scp_core::domain::events::{
    BeadEvent, DomainEvent, EventMetadata, SessionEvent, WorkspaceEvent,
};

fn create_test_metadata() -> EventMetadata {
    EventMetadata {
        event_id: "evt-123".into(),
        timestamp: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc),
        correlation_id: Some("corr-456".into()),
        causation_id: Some("cmd-789".into()),
    }
}

#[test]
fn test_session_created_event_json() {
    let event = DomainEvent::Session(SessionEvent::Created {
        metadata: create_test_metadata(),
        session_id: "session-abc".into(),
        session_name: "test-session".into(),
        workspace_path: "/tmp/workspace".into(),
        parent_session_id: None,
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("session_created_event", json);
}

#[test]
fn test_session_focused_event_json() {
    let event = DomainEvent::Session(SessionEvent::Focused {
        metadata: create_test_metadata(),
        session_id: "session-abc".into(),
        previous_session_id: Some("session-xyz".into()),
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("session_focused_event", json);
}

#[test]
fn test_session_paused_event_json() {
    let event = DomainEvent::Session(SessionEvent::Paused {
        metadata: create_test_metadata(),
        session_id: "session-abc".into(),
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("session_paused_event", json);
}

#[test]
fn test_session_completed_event_json() {
    let event = DomainEvent::Session(SessionEvent::Completed {
        metadata: create_test_metadata(),
        session_id: "session-abc".into(),
        duration_ms: 3600000,
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("session_completed_event", json);
}

#[test]
fn test_workspace_created_event_json() {
    let event = DomainEvent::Workspace(WorkspaceEvent::Created {
        metadata: create_test_metadata(),
        workspace_id: "ws-123".into(),
        workspace_name: "main".into(),
        workspace_path: "/home/user/projects/main".into(),
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("workspace_created_event", json);
}

#[test]
fn test_workspace_locked_event_json() {
    let event = DomainEvent::Workspace(WorkspaceEvent::Locked {
        metadata: create_test_metadata(),
        workspace_id: "ws-123".into(),
        locked_by: "agent-456".into(),
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("workspace_locked_event", json);
}

#[test]
fn test_bead_created_event_json() {
    let event = DomainEvent::Bead(BeadEvent::Created {
        metadata: create_test_metadata(),
        bead_id: "bd-789".into(),
        bead_title: "Implement feature X".into(),
        priority: 1,
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("bead_created_event", json);
}

#[test]
fn test_bead_state_changed_event_json() {
    let event = DomainEvent::Bead(BeadEvent::StateChanged {
        metadata: create_test_metadata(),
        bead_id: "bd-789".into(),
        from_state: "open".into(),
        to_state: "in_progress".into(),
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("bead_state_changed_event", json);
}

#[test]
fn test_bead_completed_event_json() {
    let event = DomainEvent::Bead(BeadEvent::Completed {
        metadata: create_test_metadata(),
        bead_id: "bd-789".into(),
        duration_ms: 7200000,
    });
    let json = serde_json::to_string(&event).unwrap();
    insta::assert_snapshot!("bead_completed_event", json);
}

#[test]
fn test_event_metadata_json() {
    let metadata = EventMetadata {
        event_id: "evt-test".into(),
        timestamp: DateTime::parse_from_rfc3339("2024-06-15T14:22:00Z")
            .unwrap()
            .with_timezone(&Utc),
        correlation_id: Some("corr-abc".into()),
        causation_id: None,
    };
    let json = serde_json::to_string(&metadata).unwrap();
    insta::assert_snapshot!("event_metadata", json);
}

#[test]
fn test_event_metadata_without_correlation_json() {
    let metadata = EventMetadata {
        event_id: "evt-test-2".into(),
        timestamp: DateTime::parse_from_rfc3339("2024-06-15T14:22:00Z")
            .unwrap()
            .with_timezone(&Utc),
        correlation_id: None,
        causation_id: None,
    };
    let json = serde_json::to_string(&metadata).unwrap();
    insta::assert_snapshot!("event_metadata_no_correlation", json);
}
