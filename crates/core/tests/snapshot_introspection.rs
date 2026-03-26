//! Snapshot tests for introspection and lock type JSON serialization.
//!
//! These tests verify that introspection types and lock types serialize
//! correctly to JSON for monitoring and debugging.

use chrono::{DateTime, Utc};
use scp_core::{
    introspection::types::{IntrospectionResult, QueueMetrics, SessionMetrics},
    lock::{LockHandle, LockId, LockKind},
    Fix, FixStatus,
};

#[test]
fn test_introspection_result_json() {
    let result = IntrospectionResult {
        timestamp: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc),
        sessions: vec![SessionMetrics {
            session_id: "session-1".into(),
            session_name: "main".into(),
            status: "active".into(),
            duration_secs: 3600,
            agent_id: Some("agent-1".into()),
            branch: "main".into(),
        }],
        queue: QueueMetrics {
            size: 5,
            max_size: 100,
            front_id: Some("bead-1".into()),
        },
        system_load: 0.75,
    };
    let json = serde_json::to_string(&result).unwrap();
    insta::assert_snapshot!("introspection_result", json);
}

#[test]
fn test_queue_metrics_json() {
    let metrics = QueueMetrics {
        size: 42,
        max_size: 100,
        front_id: Some("bead-abc".into()),
    };
    let json = serde_json::to_string(&metrics).unwrap();
    insta::assert_snapshot!("queue_metrics", json);
}

#[test]
fn test_queue_metrics_empty_json() {
    let metrics = QueueMetrics {
        size: 0,
        max_size: 100,
        front_id: None,
    };
    let json = serde_json::to_string(&metrics).unwrap();
    insta::assert_snapshot!("queue_metrics_empty", json);
}

#[test]
fn test_session_metrics_json() {
    let metrics = SessionMetrics {
        session_id: "session-xyz".into(),
        session_name: "feature-test".into(),
        status: "paused".into(),
        duration_secs: 7200,
        agent_id: None,
        branch: "feature/test".into(),
    };
    let json = serde_json::to_string(&metrics).unwrap();
    insta::assert_snapshot!("session_metrics", json);
}

#[test]
fn test_lock_id_json() {
    let id = LockId("lock-123".into());
    let json = serde_json::to_string(&id).unwrap();
    insta::assert_snapshot!("lock_id", json);
}

#[test]
fn test_lock_kind_serialization() {
    let kinds = vec![
        (LockKind::Exclusive, "exclusive"),
        (LockKind::Shared, "shared"),
    ];

    for (kind, name) in kinds {
        let json = serde_json::to_string(&kind).unwrap();
        insta::assert_snapshot!(format!("lock_kind_{}", name), json);
    }
}

#[test]
fn test_lock_handle_json() {
    let handle = LockHandle {
        lock_id: "lock-abc".into(),
        holder_id: "agent-456".into(),
        kind: LockKind::Exclusive,
        acquired_at: DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        expires_at: DateTime::parse_from_rfc3339("2024-01-15T11:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    };
    let json = serde_json::to_string(&handle).unwrap();
    insta::assert_snapshot!("lock_handle", json);
}

#[test]
fn test_fix_result_json() {
    let fix = Fix {
        id: "fix-789".into(),
        description: "Remove stale lock".into(),
        target: "lock-abc".into(),
        status: FixStatus::Applied,
        applied_at: Some(
            DateTime::parse_from_rfc3339("2024-01-15T10:05:00Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
        error: None,
    };
    let json = serde_json::to_string(&fix).unwrap();
    insta::assert_snapshot!("fix_result", json);
}

#[test]
fn test_fix_result_with_error_json() {
    let fix = Fix {
        id: "fix-789".into(),
        description: "Remove stale lock".into(),
        target: "lock-abc".into(),
        status: FixStatus::Failed,
        applied_at: None,
        error: Some("Permission denied".into()),
    };
    let json = serde_json::to_string(&fix).unwrap();
    insta::assert_snapshot!("fix_result_error", json);
}
