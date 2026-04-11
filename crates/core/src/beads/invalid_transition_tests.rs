#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Tests for invalid bead state machine transitions.
//!
//! This module tests that invalid transitions are properly rejected and that
//! state invariants are preserved after failed transition attempts:
//!
//! - `reopen()` from non-closed states must fail
//! - Failed transitions must not mutate state
//! - `transition_to` edge cases (self-transitions, idempotency)
//! - Proptest-based exhaustive invalid transition coverage
//! - Closed timestamp integrity through failed transitions
//! - Rapid transition sequences with invalid intermediate states

use chrono::Utc;
use proptest::prelude::*;

use super::{
    domain::{DomainError, IssueState},
    issue::Issue,
};

// ============================================================================
// reopen() Invalid Transition Tests
// ============================================================================

#[test]
fn reopen_from_open_returns_invalid_transition_error() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    let result = issue.reopen();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::InvalidStateTransition { .. })
    ));
}

#[test]
fn reopen_from_open_preserves_open_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    let _ = issue.reopen();
    assert_eq!(issue.state, IssueState::Open);
    assert!(issue.is_active());
}

#[test]
fn reopen_from_in_progress_returns_invalid_transition_error() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    let result = issue.reopen();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::InvalidStateTransition { .. })
    ));
}

#[test]
fn reopen_from_in_progress_preserves_in_progress_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    let _ = issue.reopen();
    assert_eq!(issue.state, IssueState::InProgress);
    assert!(issue.is_active());
}

#[test]
fn reopen_from_blocked_returns_invalid_transition_error() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    let result = issue.reopen();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::InvalidStateTransition { .. })
    ));
}

#[test]
fn reopen_from_blocked_preserves_blocked_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    let _ = issue.reopen();
    assert_eq!(issue.state, IssueState::Blocked);
    assert!(issue.is_blocked());
    assert!(!issue.is_active());
}

#[test]
fn reopen_from_deferred_returns_invalid_transition_error() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();
    let result = issue.reopen();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::InvalidStateTransition { .. })
    ));
}

#[test]
fn reopen_from_deferred_preserves_deferred_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();
    let _ = issue.reopen();
    assert_eq!(issue.state, IssueState::Deferred);
    assert!(!issue.is_active());
}

// ============================================================================
// reopen() Error Message Content Tests
// ============================================================================

#[test]
fn reopen_from_open_error_contains_correct_from_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    let result = issue.reopen();
    if let Err(DomainError::InvalidStateTransition { from, to }) = result {
        assert_eq!(from, "open");
        assert_eq!(to, "open");
    } else {
        panic!("Expected InvalidStateTransition error");
    }
}

#[test]
fn reopen_from_in_progress_error_contains_correct_from_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    let result = issue.reopen();
    if let Err(DomainError::InvalidStateTransition { from, to }) = result {
        assert_eq!(from, "inprogress");
        assert_eq!(to, "open");
    } else {
        panic!("Expected InvalidStateTransition error");
    }
}

#[test]
fn reopen_from_blocked_error_contains_correct_from_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    let result = issue.reopen();
    if let Err(DomainError::InvalidStateTransition { from, to }) = result {
        assert_eq!(from, "blocked");
        assert_eq!(to, "open");
    } else {
        panic!("Expected InvalidStateTransition error");
    }
}

#[test]
fn reopen_from_deferred_error_contains_correct_from_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();
    let result = issue.reopen();
    if let Err(DomainError::InvalidStateTransition { from, to }) = result {
        assert_eq!(from, "deferred");
        assert_eq!(to, "open");
    } else {
        panic!("Expected InvalidStateTransition error");
    }
}

// ============================================================================
// Closed Timestamp Integrity After Failed Transitions
// ============================================================================

#[test]
fn failed_reopen_does_not_clear_closed_at() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    let closed_time = Utc::now();
    issue.close_with_time(closed_time);
    assert!(issue.is_closed());

    issue.transition_to(IssueState::InProgress).unwrap();
    assert!(!issue.is_closed());

    let result = issue.reopen();
    assert!(result.is_err());
    assert!(!issue.is_closed());
    assert!(issue.closed_at().is_none());
}

#[test]
fn failed_reopen_from_in_progress_does_not_set_closed_at() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    assert!(issue.closed_at().is_none());

    let _ = issue.reopen();
    assert!(issue.closed_at().is_none());
}

#[test]
fn failed_reopen_from_blocked_does_not_set_closed_at() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    assert!(issue.closed_at().is_none());

    let _ = issue.reopen();
    assert!(issue.closed_at().is_none());
}

// ============================================================================
// Idempotent Self-Transition Tests
// ============================================================================

#[test]
fn transition_open_to_open_is_idempotent() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    let result = issue.transition_to(IssueState::Open);
    assert!(result.is_ok());
    assert_eq!(issue.state, IssueState::Open);
}

#[test]
fn transition_in_progress_to_in_progress_is_idempotent() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    let result = issue.transition_to(IssueState::InProgress);
    assert!(result.is_ok());
    assert_eq!(issue.state, IssueState::InProgress);
}

#[test]
fn transition_blocked_to_blocked_is_idempotent() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    let result = issue.transition_to(IssueState::Blocked);
    assert!(result.is_ok());
    assert_eq!(issue.state, IssueState::Blocked);
}

#[test]
fn transition_deferred_to_deferred_is_idempotent() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();
    let result = issue.transition_to(IssueState::Deferred);
    assert!(result.is_ok());
    assert_eq!(issue.state, IssueState::Deferred);
}

#[test]
fn transition_closed_to_closed_replaces_timestamp() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.close_with_time(Utc::now());
    let first_close = issue.closed_at().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(5));

    let new_time = Utc::now();
    issue
        .transition_to(IssueState::Closed {
            closed_at: new_time,
        })
        .unwrap();

    assert!(issue.is_closed());
    assert_eq!(issue.closed_at(), Some(new_time));
    assert_ne!(issue.closed_at(), Some(first_close));
}

// ============================================================================
// Rapid Transition Sequences With Invalid Intermediate States
// ============================================================================

#[test]
fn close_then_reopen_then_reopen_again_fails() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.close();
    issue.reopen().unwrap();
    assert_eq!(issue.state, IssueState::Open);

    let result = issue.reopen();
    assert!(result.is_err());
    assert_eq!(issue.state, IssueState::Open);
}

#[test]
fn open_to_blocked_to_reopen_fails() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();

    let result = issue.reopen();
    assert!(result.is_err());
    assert_eq!(issue.state, IssueState::Blocked);
}

#[test]
fn in_progress_to_deferred_to_reopen_fails() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    issue.transition_to(IssueState::Deferred).unwrap();

    let result = issue.reopen();
    assert!(result.is_err());
    assert_eq!(issue.state, IssueState::Deferred);
}

#[test]
fn multiple_failed_reopens_do_not_corrupt_state() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();

    for _ in 0..10 {
        let result = issue.reopen();
        assert!(result.is_err());
        assert_eq!(issue.state, IssueState::Blocked);
        assert!(issue.is_blocked());
    }
}

#[test]
fn close_reopen_close_reopen_cycle_preserves_invariants() {
    let mut issue = Issue::new("t-1", "Test").unwrap();

    for _ in 0..5 {
        issue.close();
        assert!(issue.is_closed());
        assert!(issue.closed_at().is_some());

        issue.reopen().unwrap();
        assert!(!issue.is_closed());
        assert!(issue.closed_at().is_none());
        assert!(issue.is_active());
        assert_eq!(issue.state, IssueState::Open);
    }
}

#[test]
fn transition_sequence_through_all_states_preserves_integrity() {
    let mut issue = Issue::new("t-1", "Test").unwrap();

    issue.transition_to(IssueState::InProgress).unwrap();
    assert!(issue.is_active());

    issue.transition_to(IssueState::Blocked).unwrap();
    assert!(issue.is_blocked());
    assert!(!issue.is_active());

    issue.transition_to(IssueState::Deferred).unwrap();
    assert!(!issue.is_active());

    issue.transition_to(IssueState::Open).unwrap();
    assert!(issue.is_active());

    issue.close();
    assert!(issue.is_closed());
    assert!(issue.closed_at().is_some());

    issue.reopen().unwrap();
    assert_eq!(issue.state, IssueState::Open);
}

// ============================================================================
// updated_at Integrity After Failed Transitions
// ============================================================================

#[test]
fn failed_reopen_does_not_update_updated_at() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    let before = issue.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(5));

    let _ = issue.reopen();
    assert_eq!(issue.updated_at, before);
}

// ============================================================================
// Proptest: Exhaustive Invalid Reopen Coverage
// ============================================================================

fn non_closed_state_strategy() -> impl Strategy<Value = IssueState> {
    prop_oneof![
        Just(IssueState::Open),
        Just(IssueState::InProgress),
        Just(IssueState::Blocked),
        Just(IssueState::Deferred),
    ]
}

proptest! {
    #[test]
    fn prop_reopen_from_non_closed_always_fails(state in non_closed_state_strategy()) {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        issue.transition_to(state).unwrap();

        let result = issue.reopen();
        prop_assert!(result.is_err());
        let is_correct_error = matches!(
            result,
            Err(DomainError::InvalidStateTransition { .. })
        );
        prop_assert!(is_correct_error);
    }

    #[test]
    fn prop_reopen_from_non_closed_preserves_original_state(state in non_closed_state_strategy()) {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        issue.transition_to(state).unwrap();
        let original_state = issue.state;

        let _ = issue.reopen();
        prop_assert_eq!(issue.state, original_state);
    }

    #[test]
    fn prop_reopen_from_closed_always_succeeds(
        state in non_closed_state_strategy()
    ) {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        issue.transition_to(state).unwrap();
        issue.close();
        assert!(issue.is_closed());

        let result = issue.reopen();
        prop_assert!(result.is_ok());
        prop_assert_eq!(issue.state, IssueState::Open);
        prop_assert!(!issue.is_closed());
        prop_assert!(issue.is_active());
    }

    #[test]
    fn prop_transition_to_any_state_always_succeeds(
        from_state in non_closed_state_strategy(),
        to_state in prop_oneof![
            Just(IssueState::Open),
            Just(IssueState::InProgress),
            Just(IssueState::Blocked),
            Just(IssueState::Deferred),
            Just(IssueState::Closed { closed_at: Utc::now() }),
        ]
    ) {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        issue.transition_to(from_state).unwrap();

        let result = issue.transition_to(to_state);
        prop_assert!(result.is_ok());
        prop_assert_eq!(issue.state, to_state);
    }

    #[test]
    fn prop_closed_state_integrity_through_any_transition(
        to_state in prop_oneof![
            Just(IssueState::Open),
            Just(IssueState::InProgress),
            Just(IssueState::Blocked),
            Just(IssueState::Deferred),
            Just(IssueState::Closed { closed_at: Utc::now() }),
        ]
    ) {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        let closed_time = Utc::now();
        issue.close_with_time(closed_time);

        issue.transition_to(to_state).unwrap();

        match to_state {
            IssueState::Closed { .. } => {
                prop_assert!(issue.is_closed());
                prop_assert!(issue.closed_at().is_some());
                prop_assert_ne!(issue.closed_at(), Some(closed_time));
            }
            _ => {
                prop_assert!(!issue.is_closed());
                prop_assert!(issue.closed_at().is_none());
            }
        }
    }
}

// ============================================================================
// State Query Consistency After Failed Transitions
// ============================================================================

#[test]
fn is_active_consistent_after_failed_reopen_from_in_progress() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::InProgress).unwrap();
    assert!(issue.is_active());

    let _ = issue.reopen();
    assert!(issue.is_active());
    assert!(!issue.is_blocked());
    assert!(!issue.is_closed());
}

#[test]
fn is_blocked_consistent_after_failed_reopen_from_blocked() {
    let mut issue = Issue::new("t-1", "Test").unwrap();
    issue.transition_to(IssueState::Blocked).unwrap();
    assert!(issue.is_blocked());
    assert!(!issue.is_active());

    let _ = issue.reopen();
    assert!(issue.is_blocked());
    assert!(!issue.is_active());
    assert!(!issue.is_closed());
}

#[test]
fn is_active_and_is_blocked_mutually_exclusive_after_failed_transitions() {
    let states = [
        IssueState::Open,
        IssueState::InProgress,
        IssueState::Blocked,
        IssueState::Deferred,
    ];

    for state in states {
        let mut issue = Issue::new("t-1", "Test").unwrap();
        issue.transition_to(state).unwrap();

        let _ = issue.reopen();

        match issue.state {
            IssueState::Open | IssueState::InProgress => {
                assert!(issue.is_active());
                assert!(!issue.is_blocked());
            }
            IssueState::Blocked => {
                assert!(!issue.is_active());
                assert!(issue.is_blocked());
            }
            IssueState::Deferred => {
                assert!(!issue.is_active());
                assert!(!issue.is_blocked());
            }
            IssueState::Closed { .. } => {
                assert!(!issue.is_active());
                assert!(!issue.is_blocked());
            }
        }
    }
}
