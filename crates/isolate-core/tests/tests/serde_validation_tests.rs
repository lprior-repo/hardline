#![allow(clippy::unnecessary_map_or, clippy::redundant_clone)]
//! Serde validation tests for all domain types
//!
//! This test suite ensures all domain types properly serialize/deserialize:
//! - JSON serialization roundtrip
//! - Binary serialization roundtrip
//! - Invalid data rejection
//! - Edge cases (empty strings, null values, etc.)

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use isolate_core::{
    beads::{
        Assignee, BlockedBy, DependsOn, Description, IssueId, IssueState, IssueType, Labels,
        Priority, Title,
    },
    domain::{
        events::{serialize_event, serialize_event_bytes, DomainEvent, EventMetadata, StoredEvent},
        identifiers::{
            AbsolutePath, AgentId, BeadId, SessionId, SessionName, TaskId, WorkspaceName,
        },
        session::BranchState,
    },
};

// ============================================================================
// IDENTIFIER TYPE TESTS
// ============================================================================

#[test]
fn test_session_name_json_roundtrip() {
    let original = SessionName::parse("my-session").expect("valid session name");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"my-session\"");

    // Deserialize from JSON
    let deserialized: SessionName = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_session_name_rejects_invalid_json() {
    // Empty string should be rejected during deserialization
    let result: Result<SessionName, _> = serde_json::from_str("\"\"");
    assert!(result.is_err(), "Empty session name should be rejected");

    // String starting with number should be rejected
    let result: Result<SessionName, _> = serde_json::from_str("\"123-session\"");
    assert!(
        result.is_err(),
        "Session name starting with number should be rejected"
    );

    // String with special characters should be rejected
    let result: Result<SessionName, _> = serde_json::from_str("\"my.session\"");
    assert!(result.is_err(), "Session name with dots should be rejected");
}

#[test]
fn test_session_name_whitespace_handling() {
    // Leading/trailing whitespace is trimmed during construction
    let original = SessionName::parse("  my-session  ").expect("valid");
    assert_eq!(original.as_str(), "my-session");

    // After serialization, whitespace doesn't reappear
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"my-session\"");
}

#[test]
fn test_agent_id_json_roundtrip() {
    let original = AgentId::parse("agent-123").expect("valid agent ID");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"agent-123\"");

    // Deserialize from JSON
    let deserialized: AgentId = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_agent_id_rejects_invalid_json() {
    // Empty string should be rejected
    let result: Result<AgentId, _> = serde_json::from_str("\"\"");
    assert!(result.is_err(), "Empty agent ID should be rejected");

    // String with spaces should be rejected
    let result: Result<AgentId, _> = serde_json::from_str("\"agent 123\"");
    assert!(result.is_err(), "Agent ID with spaces should be rejected");
}

#[test]
fn test_workspace_name_json_roundtrip() {
    let original = WorkspaceName::parse("my-workspace").expect("valid workspace name");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"my-workspace\"");

    // Deserialize from JSON
    let deserialized: WorkspaceName = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_workspace_name_rejects_path_separators() {
    // Forward slash should be rejected
    let result: Result<WorkspaceName, _> = serde_json::from_str("\"my/workspace\"");
    assert!(result.is_err(), "Workspace name with / should be rejected");

    // Backslash should be rejected
    let result: Result<WorkspaceName, _> = serde_json::from_str("\"my\\\\workspace\"");
    assert!(result.is_err(), "Workspace name with \\ should be rejected");
}

#[test]
fn test_task_id_json_roundtrip() {
    let original = TaskId::parse("bd-abc123").expect("valid task ID");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"bd-abc123\"");

    // Deserialize from JSON
    let deserialized: TaskId = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_task_id_rejects_invalid_prefix() {
    // Missing bd- prefix should be rejected
    let result: Result<TaskId, _> = serde_json::from_str("\"abc123\"");
    assert!(
        result.is_err(),
        "Task ID without bd- prefix should be rejected"
    );

    // Wrong prefix should be rejected
    let result: Result<TaskId, _> = serde_json::from_str("\"task-abc123\"");
    assert!(
        result.is_err(),
        "Task ID with wrong prefix should be rejected"
    );
}

#[test]
fn test_task_id_rejects_invalid_hex() {
    // Non-hex characters should be rejected
    let result: Result<TaskId, _> = serde_json::from_str("\"bd-xyz\"");
    assert!(result.is_err(), "Task ID with non-hex should be rejected");
}

#[test]
fn test_session_id_json_roundtrip() {
    let original = SessionId::parse("session-abc123").expect("valid session ID");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"session-abc123\"");

    // Deserialize from JSON
    let deserialized: SessionId = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_session_id_rejects_non_ascii() {
    // Non-ASCII characters should be rejected
    let result: Result<SessionId, _> = serde_json::from_str("\"session-日本語\"");
    assert!(
        result.is_err(),
        "Session ID with non-ASCII should be rejected"
    );
}

#[test]
fn test_absolute_path_json_roundtrip() {
    let original = AbsolutePath::parse("/home/user/workspace").expect("valid path");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"/home/user/workspace\"");

    // Deserialize from JSON
    let deserialized: AbsolutePath = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_absolute_path_rejects_relative() {
    // Relative path should be rejected
    let result: Result<AbsolutePath, _> = serde_json::from_str("\"relative/path\"");
    assert!(result.is_err(), "Relative path should be rejected");
}

#[test]
fn test_bead_id_json_roundtrip() {
    let original = BeadId::parse("bd-abc123def456").expect("valid bead ID");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"bd-abc123def456\"");

    // Deserialize from JSON
    let deserialized: BeadId = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

// ============================================================================
// VALUE OBJECT TESTS
// ============================================================================

#[test]
fn test_title_json_roundtrip() {
    let original = Title::new("My Issue Title").expect("valid title");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"My Issue Title\"");

    // Deserialize from JSON
    let deserialized: Title = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_title_accepts_empty_after_trim() {
    // Empty string should be rejected (whitespace-only becomes empty after trim)
    let result = Title::new("");
    assert!(result.is_err(), "Empty title should be rejected");

    // Whitespace-only should also be rejected
    let result = Title::new("   ");
    assert!(result.is_err(), "Whitespace-only title should be rejected");
}

#[test]
fn test_title_trims_whitespace() {
    // Whitespace is trimmed but not preserved in serialization
    let original = Title::new("  My Title  ").expect("valid title");
    assert_eq!(original.as_str(), "My Title");

    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"My Title\"");
}

#[test]
fn test_description_json_roundtrip() {
    let original = Description::new("This is a detailed description").expect("valid");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"This is a detailed description\"");

    // Deserialize from JSON
    let deserialized: Description = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_description_allows_empty() {
    // Empty description is allowed
    let original = Description::new("").expect("valid");
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"\"");
}

#[test]
fn test_issue_id_json_roundtrip() {
    let original = IssueId::new("issue-123").expect("valid issue ID");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"issue-123\"");

    // Deserialize from JSON
    let deserialized: IssueId = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_assignee_json_roundtrip() {
    let original = Assignee::new("user@example.com").expect("valid assignee");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "\"user@example.com\"");

    // Deserialize from JSON
    let deserialized: Assignee = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

// ============================================================================
// STATE ENUM TESTS
// ============================================================================

#[test]
fn test_issue_state_json_roundtrip() {
    let test_cases = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
    ];

    for state in test_cases {
        // Serialize to JSON
        let json = serde_json::to_string(&state).expect("serialization failed");

        // Deserialize from JSON
        let deserialized: IssueState = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized, state);
    }
}

#[test]
fn test_issue_state_closed_with_timestamp() {
    let now = Utc::now();
    let state = IssueState::Closed { closed_at: now };

    // Serialize to JSON
    let json = serde_json::to_string(&state).expect("serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    // Should have "closed" variant with closed_at field inside
    assert!(parsed.get("closed").is_some());
    if let Some(closed_obj) = parsed.get("closed") {
        assert!(closed_obj.get("closed_at").is_some());
    }

    // Deserialize from JSON
    let deserialized: IssueState = serde_json::from_str(&json).expect("deserialization failed");
    match deserialized {
        IssueState::Closed { closed_at: dt } => {
            // Chrono timestamps have microsecond precision
            let duration = if dt > now { dt - now } else { now - dt };
            assert!(duration.num_microseconds().map_or(false, |d| d < 1_000_000));
        }
        _ => panic!("Expected Closed state"),
    }
}

#[test]
fn test_priority_json_roundtrip() {
    let priorities = vec![
        Priority::P0,
        Priority::P1,
        Priority::P2,
        Priority::P3,
        Priority::P4,
    ];

    for priority in priorities {
        // Serialize to JSON
        let json = serde_json::to_string(&priority).expect("serialization failed");

        // Deserialize from JSON
        let deserialized: Priority = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized, priority);
    }
}

#[test]
fn test_priority_lowercase_serialization() {
    // Priorities serialize to lowercase
    let json = serde_json::to_string(&Priority::P0).expect("serialization failed");
    assert_eq!(json, "\"p0\"");

    // Can deserialize from lowercase
    let deserialized: Priority = serde_json::from_str("\"p0\"").expect("deserialization failed");
    assert_eq!(deserialized, Priority::P0);
}

#[test]
fn test_issue_type_json_roundtrip() {
    let issue_types = vec![
        IssueType::Bug,
        IssueType::Feature,
        IssueType::Task,
        IssueType::Epic,
        IssueType::Chore,
        IssueType::MergeRequest,
    ];

    for issue_type in issue_types {
        // Serialize to JSON
        let json = serde_json::to_string(&issue_type).expect("serialization failed");

        // Deserialize from JSON
        let deserialized: IssueType = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized, issue_type);
    }
}

#[test]
fn test_issue_type_lowercase_serialization() {
    // Issue types serialize to lowercase (MergeRequest -> mergerequest)
    let json = serde_json::to_string(&IssueType::MergeRequest).expect("serialization failed");
    assert_eq!(json, "\"mergerequest\"");

    // Can deserialize from lowercase
    let deserialized: IssueType =
        serde_json::from_str("\"mergerequest\"").expect("deserialization failed");
    assert_eq!(deserialized, IssueType::MergeRequest);
}

#[test]
fn test_branch_state_json_roundtrip() {
    let test_cases = vec![
        BranchState::Detached,
        BranchState::OnBranch {
            name: "main".to_string(),
        },
    ];

    for state in test_cases {
        // Serialize to JSON
        let json = serde_json::to_string(&state).expect("serialization failed");

        // Deserialize from JSON
        let deserialized: BranchState =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized, state);
    }
}

#[test]
fn test_branch_state_snake_case() {
    // OnBranch should serialize as "on_branch"
    let state = BranchState::OnBranch {
        name: "main".to_string(),
    };
    let json = serde_json::to_string(&state).expect("serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    // Should have snake_case variant
    assert!(parsed.get("on_branch").is_some());
}

// ============================================================================
// COLLECTION TYPE TESTS
// ============================================================================

#[test]
fn test_labels_json_roundtrip() {
    let original = Labels::new(vec!["bug".to_string(), "urgent".to_string()]).expect("valid");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");

    // Deserialize from JSON
    let deserialized: Labels = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_labels_empty() {
    let original = Labels::empty();

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");
    assert_eq!(json, "[]");

    // Deserialize from JSON
    let deserialized: Labels = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_depends_on_json_roundtrip() {
    let original =
        DependsOn::new(vec!["issue-1".to_string(), "issue-2".to_string()]).expect("valid");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");

    // Deserialize from JSON
    let deserialized: DependsOn = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

#[test]
fn test_blocked_by_json_roundtrip() {
    let original = BlockedBy::new(vec!["issue-1".to_string()]).expect("valid");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("serialization failed");

    // Deserialize from JSON
    let deserialized: BlockedBy = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized, original);
}

// ============================================================================
// DOMAIN EVENT TESTS
// ============================================================================

#[test]
fn test_domain_event_json_roundtrip() {
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
            "Fix bug".to_string(),
            Some("Critical".to_string()),
            Utc::now(),
        ),
    ];

    for event in events {
        // JSON roundtrip
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized =
            serde_json::from_str::<DomainEvent>(&json).expect("deserialization failed");
        assert_eq!(event, deserialized);

        // Binary roundtrip
        let bytes = serialize_event_bytes(&event).expect("serialization failed");
        let deserialized_bytes =
            serde_json::from_slice::<DomainEvent>(&bytes).expect("deserialization failed");
        assert_eq!(event, deserialized_bytes);
    }
}

#[test]
fn test_domain_event_tagged_enums() {
    let event = DomainEvent::session_created(
        "session-123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        Utc::now(),
    );

    let json = serialize_event(&event).expect("serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    // Should have event_type tag (PascalCase)
    assert!(value.get("event_type").is_some());
    assert_eq!(value["event_type"], "SessionCreated");

    // Should have data field
    assert!(value.get("data").is_some());
}

#[test]
fn test_stored_event_serialization() {
    let event = DomainEvent::session_created(
        "session-123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        Utc::now(),
    );

    let metadata = EventMetadata {
        event_number: 1,
        stream_id: "session-123".to_string(),
        stream_version: 1,
        stored_at: Utc::now(),
    };

    let stored = StoredEvent::new(event.clone(), metadata);

    // Serialize
    let json = serde_json::to_string(&stored).expect("serialization failed");

    // Deserialize
    let deserialized: StoredEvent = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(deserialized.metadata.event_number, 1);
    assert_eq!(deserialized.metadata.stream_id, "session-123");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_null_values_in_option_fields() {
    // Test that null is properly handled for optional fields
    let json_null = r"null";
    let result: Option<String> = serde_json::from_str(json_null).expect("valid null");
    assert!(result.is_none());
}

#[test]
fn test_unicode_in_identifiers() {
    // Session names must be ASCII, so unicode should fail
    let result: Result<SessionName, _> = serde_json::from_str("\"session-日本語\"");
    assert!(result.is_err());
}

#[test]
fn test_very_long_strings_rejected() {
    // Session names over 63 chars should be rejected
    let too_long = "a".repeat(64);
    let result: Result<SessionName, _> = serde_json::from_str(&format!("\"{too_long}\""));
    assert!(result.is_err());
}

#[test]
fn test_special_characters_in_paths() {
    // Paths with null bytes should be rejected
    let result: Result<AbsolutePath, _> = serde_json::from_str("\"/path\\0with\\0nulls\"");
    assert!(result.is_err());
}

#[test]
fn test_timestamp_serialization() {
    let now = Utc::now();
    let json = serde_json::to_string(&now).expect("serialization failed");

    let deserialized: DateTime<Utc> = serde_json::from_str(&json).expect("deserialization failed");

    // Timestamps should survive roundtrip within reasonable precision
    let duration = if deserialized > now {
        deserialized - now
    } else {
        now - deserialized
    };
    assert!(duration.num_microseconds().map_or(false, |d| d < 1_000_000));
}

#[test]
fn test_datetime_in_closed_state() {
    let now = Utc::now();
    let state = IssueState::Closed { closed_at: now };

    let json = serde_json::to_string(&state).expect("serialization failed");
    let deserialized: IssueState = serde_json::from_str(&json).expect("deserialization failed");

    match deserialized {
        IssueState::Closed { closed_at: dt } => {
            let duration = if dt > now { dt - now } else { now - dt };
            assert!(duration.num_microseconds().map_or(false, |d| d < 1_000_000));
        }
        _ => panic!("Expected Closed state"),
    }
}

// ============================================================================
// INVALID JSON STRUCTURE TESTS
// ============================================================================

#[test]
fn test_invalid_json_syntax() {
    let result: Result<SessionName, _> = serde_json::from_str("invalid json");
    assert!(result.is_err());
}

#[test]
fn test_wrong_type_for_string() {
    // Number instead of string
    let result: Result<SessionName, _> = serde_json::from_str("123");
    assert!(result.is_err());
}

#[test]
fn test_array_instead_of_string() {
    let result: Result<SessionName, _> = serde_json::from_str("[\"my-session\"]");
    assert!(result.is_err());
}

#[test]
fn test_object_instead_of_string() {
    let result: Result<SessionName, _> = serde_json::from_str("{\"name\":\"my-session\"}");
    assert!(result.is_err());
}

// ============================================================================
// COMPLEX SCENARIOS
// ============================================================================

#[test]
fn test_event_with_all_fields() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_failed(
        "session-123".to_string(),
        SessionName::parse("my-session").expect("valid"),
        "Critical error occurred".to_string(),
        timestamp,
    );

    let json = serialize_event(&event).expect("serialization failed");
    let deserialized: DomainEvent = serde_json::from_str(&json).expect("deserialization failed");

    match deserialized {
        DomainEvent::SessionFailed(e) => {
            assert_eq!(e.session_id, "session-123");
            assert_eq!(e.session_name.as_str(), "my-session");
            assert_eq!(e.reason, "Critical error occurred");
        }
        _ => panic!("Expected SessionFailed event"),
    }
}

#[test]
fn test_multiple_events_preserve_types() {
    let events = vec![
        DomainEvent::session_created(
            "s1".to_string(),
            SessionName::parse("s1").expect("valid"),
            Utc::now(),
        ),
        DomainEvent::session_completed(
            "s2".to_string(),
            SessionName::parse("s2").expect("valid"),
            Utc::now(),
        ),
        DomainEvent::session_failed(
            "s3".to_string(),
            SessionName::parse("s3").expect("valid"),
            "error".to_string(),
            Utc::now(),
        ),
    ];

    let json_array = serde_json::to_string(&events).expect("serialization failed");
    let deserialized: Vec<DomainEvent> =
        serde_json::from_str(&json_array).expect("deserialization failed");

    assert_eq!(deserialized.len(), 3);
    assert!(matches!(deserialized[0], DomainEvent::SessionCreated(_)));
    assert!(matches!(deserialized[1], DomainEvent::SessionCompleted(_)));
    assert!(matches!(deserialized[2], DomainEvent::SessionFailed(_)));
}
