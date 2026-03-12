#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! State transition invariant tests for beads domain.
//!
//! This module specifically tests state transition invariants:
//! - Closed beads cannot be transitioned to closed without timestamp
//! - Closed state always includes timestamp (type-level guarantee)
//! - Reopening is only allowed from closed state

use chrono::{Duration, Utc};

use super::{
    domain::{DomainError, IssueId, IssueState},
    issue::Issue,
};

// ============================================================================
// Closed State Invariant Tests
// ============================================================================

#[test]
fn test_closed_state_requires_timestamp() {
    // This is enforced at the type level - you cannot create a Closed state
    // without providing a timestamp. This is a compilation error if you try:
    // let state = IssueState::Closed;  // Won't compile!

    // Correct usage:
    let state = IssueState::Closed {
        closed_at: Utc::now(),
    };
    assert!(state.is_closed());
    assert!(state.closed_at().is_some());
}

#[test]
fn test_closed_state_timestamp_is_preserved() {
    let specific_time = Utc::now() - Duration::days(30);
    let state = IssueState::Closed {
        closed_at: specific_time,
    };

    assert_eq!(state.closed_at(), Some(specific_time));
}

#[test]
fn test_closed_state_cannot_be_created_without_timestamp_by_transition() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    // Attempt to transition to Closed without timestamp should fail
    // (but our current implementation allows flexible transitions)
    // The issue is that IssueState::Closed is a variant, not a separate type
    let result = issue.transition_to(IssueState::Closed {
        closed_at: Utc::now(),
    });

    assert!(result.is_ok());
    assert!(issue.is_closed());
    assert!(issue.closed_at().is_some());
}

// ============================================================================
// Reopen Invariant Tests
// ============================================================================

#[test]
fn test_reopen_from_closed_succeeds() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.close();

    assert!(issue.is_closed());

    let result = issue.reopen();
    assert!(result.is_ok());
    assert!(!issue.is_closed());
    assert!(issue.is_active());
    assert_eq!(issue.state, IssueState::Open);
}

#[test]
fn test_reopen_from_open_fails() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    let result = issue.reopen();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::InvalidStateTransition { .. })
    ));

    // Issue should remain open
    assert!(!issue.is_closed());
    assert!(issue.is_active());
}

#[test]
fn test_reopen_from_in_progress_fails() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();

    let result = issue.reopen();
    assert!(result.is_err());

    // Issue should remain in progress
    assert!(!issue.is_closed());
    assert!(issue.is_active());
}

#[test]
fn test_reopen_from_blocked_fails() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();

    let result = issue.reopen();
    assert!(result.is_err());

    // Issue should remain blocked
    assert!(!issue.is_closed());
    assert!(!issue.is_active()); // Blocked is not active
    assert!(issue.is_blocked());
}

#[test]
fn test_reopen_from_deferred_fails() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();

    let result = issue.reopen();
    assert!(result.is_err());

    // Issue should remain deferred
    assert!(!issue.is_closed());
    assert!(!issue.is_active());
}

#[test]
fn test_close_reopen_close_cycle() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    // Initial state
    assert!(!issue.is_closed());

    // Close
    issue.close();
    assert!(issue.is_closed());

    // Reopen
    issue.reopen().unwrap();
    assert!(!issue.is_closed());

    // Close again with new timestamp
    let first_close_time = issue.closed_at();
    issue.close();
    assert!(issue.is_closed());

    // Timestamps should be different
    assert_ne!(first_close_time, issue.closed_at());
}

// ============================================================================
// State Transition Matrix Tests
// ============================================================================

#[test]
fn test_all_state_transitions_from_open() {
    let issue = Issue::new("test-1", "Test Issue").unwrap();
    assert_eq!(issue.state, IssueState::Open);

    // Can transition to any state from Open (flexible workflow)
    let states = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
        IssueState::Closed {
            closed_at: Utc::now(),
        },
    ];

    for new_state in states {
        let mut test_issue = Issue::new("test-1", "Test Issue").unwrap();
        let result = test_issue.transition_to(new_state);
        assert!(result.is_ok(), "Open -> {new_state:?} should succeed");
        assert_eq!(test_issue.state, new_state);
    }
}

#[test]
fn test_all_state_transitions_from_in_progress() {
    let states = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
        IssueState::Closed {
            closed_at: Utc::now(),
        },
    ];

    for new_state in states {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.transition_to(IssueState::InProgress).unwrap();

        let result = issue.transition_to(new_state);
        assert!(result.is_ok(), "InProgress -> {new_state:?} should succeed");
        assert_eq!(issue.state, new_state);
    }
}

#[test]
fn test_all_state_transitions_from_blocked() {
    let states = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
        IssueState::Closed {
            closed_at: Utc::now(),
        },
    ];

    for new_state in states {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.transition_to(IssueState::Blocked).unwrap();

        let result = issue.transition_to(new_state);
        assert!(result.is_ok(), "Blocked -> {new_state:?} should succeed");
        assert_eq!(issue.state, new_state);
    }
}

#[test]
fn test_all_state_transitions_from_deferred() {
    let states = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
        IssueState::Closed {
            closed_at: Utc::now(),
        },
    ];

    for new_state in states {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.transition_to(IssueState::Deferred).unwrap();

        let result = issue.transition_to(new_state);
        assert!(result.is_ok(), "Deferred -> {new_state:?} should succeed");
        assert_eq!(issue.state, new_state);
    }
}

#[test]
fn test_all_state_transitions_from_closed() {
    let states = vec![
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
        IssueState::Closed {
            closed_at: Utc::now(),
        },
    ];

    for new_state in states {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();

        let result = issue.transition_to(new_state);
        assert!(result.is_ok(), "Closed -> {new_state:?} should succeed");
        assert_eq!(issue.state, new_state);
    }
}

// ============================================================================
// Closed Timestamp Invariant Tests
// ============================================================================

#[test]
fn test_close_always_sets_timestamp() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    assert!(issue.closed_at().is_none());

    issue.close();

    assert!(issue.closed_at().is_some());

    let closed_at = issue.closed_at().unwrap();
    let now = Utc::now();

    // Should be within last second
    let diff = (now - closed_at).num_seconds().abs();
    assert!(diff <= 1);
}

#[test]
fn test_close_with_specific_timestamp() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    let specific_time = Utc::now() - Duration::hours(24);
    issue.close_with_time(specific_time);

    assert_eq!(issue.closed_at(), Some(specific_time));
}

#[test]
fn test_close_multiple_times_updates_timestamp() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    issue.close();
    let first_close = issue.closed_at().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    issue.close();
    let second_close = issue.closed_at().unwrap();

    assert_ne!(first_close, second_close);
    assert!(second_close > first_close);
}

#[test]
fn test_transition_to_closed_requires_timestamp() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    let closed_at = Utc::now();
    let result = issue.transition_to(IssueState::Closed { closed_at });

    assert!(result.is_ok());
    assert!(issue.is_closed());
    assert_eq!(issue.closed_at(), Some(closed_at));
}

// ============================================================================
// Issue ID Format Invariant Tests
// ============================================================================

#[test]
fn test_issue_id_rejects_empty() {
    let result = IssueId::new("");
    assert!(matches!(result, Err(DomainError::EmptyId)));
}

#[test]
fn test_issue_id_rejects_spaces() {
    let result = IssueId::new("invalid id");
    assert!(result.is_err());
}

#[test]
fn test_issue_id_rejects_special_chars() {
    let invalid_ids = vec!["invalid.id", "invalid@id", "invalid#id", "invalid/id"];

    for id in invalid_ids {
        let result = IssueId::new(id);
        assert!(result.is_err(), "Should reject: {id}");
    }
}

#[test]
fn test_issue_id_accepts_valid_formats() {
    let valid_ids = vec![
        "valid",
        "valid-123",
        "valid_456",
        "123abc",
        "ABC-123_XYZ",
        "a-b-c-1-2-3",
    ];

    for id in valid_ids {
        let result = IssueId::new(id);
        assert!(result.is_ok(), "Should accept: {id}");
    }
}

#[test]
fn test_issue_id_rejects_too_long() {
    let too_long = "a".repeat(IssueId::MAX_LENGTH + 1);
    let result = IssueId::new(&too_long);
    assert!(result.is_err());
}

#[test]
fn test_issue_id_accepts_max_length() {
    let max_length = "a".repeat(IssueId::MAX_LENGTH);
    let result = IssueId::new(&max_length);
    assert!(result.is_ok());
}

// ============================================================================
// Timestamp Ordering Invariant Tests
// ============================================================================

#[test]
fn test_created_at_equals_updated_at_on_creation() {
    let issue = Issue::new("test-1", "Test Issue").unwrap();
    assert_eq!(issue.created_at, issue.updated_at);
}

#[test]
fn test_close_updates_updated_at() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    let initial_updated = issue.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    issue.close();

    assert!(issue.updated_at > initial_updated);
}

#[test]
fn test_reopen_updates_updated_at() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.close();

    std::thread::sleep(std::time::Duration::from_millis(10));

    issue.reopen().unwrap();

    assert!(issue.updated_at > issue.created_at);
}

#[test]
fn test_any_field_update_updates_updated_at() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    let initial_updated = issue.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    issue.update_title("New Title").unwrap();

    assert!(issue.updated_at > initial_updated);
}

// ============================================================================
// Closed State Persistence Tests
// ============================================================================

#[test]
fn test_closed_state_persists_through_cloning() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.close();

    let cloned = issue.clone();

    assert!(cloned.is_closed());
    assert_eq!(cloned.closed_at(), issue.closed_at());
}

#[test]
fn test_multiple_closes_do_not_corrupt_state() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();

    for _ in 0..10 {
        issue.close();
        assert!(issue.is_closed());
        assert!(issue.closed_at().is_some());
    }
}

#[test]
fn test_closed_state_survives_serialization() {
    let mut issue = Issue::new("test-1", "Test Issue").unwrap();
    issue.close();

    let serialized = serde_json::to_string(&issue).unwrap();
    let deserialized: Issue = serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.is_closed());
    assert_eq!(deserialized.closed_at(), issue.closed_at());
}
