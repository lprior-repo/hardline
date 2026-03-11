#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Invariant tests for the beads domain.
//!
//! This module tests the core invariants of the beads issue tracking domain:
//! - Closed beads cannot be reopened (they must use `reopen()` method)
//! - Closed beads must have `closed_at` timestamp
//! - Bead IDs must be valid format
//! - State transitions are validated
//! - Labels and dependencies have limits

use chrono::{Duration, Utc};
use proptest::prelude::*;

use super::{
    domain::{
        Assignee, BlockedBy, DependsOn, Description, DomainError, IssueId, IssueState, Labels,
        Priority, Title,
    },
    issue::{Issue, IssueBuilder},
};

// ============================================================================
// Test Strategy Generators
// ============================================================================

/// Generate valid issue IDs (alphanumeric, hyphens, underscores only)
fn issue_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple alphanumeric IDs
        "[a-z0-9]{1,20}",
        // IDs with hyphens
        "[a-z0-9]{1,10}-[a-z0-9]{1,10}",
        // IDs with underscores
        "[a-z0-9]{1,10}_[a-z0-9]{1,10}",
        // Mixed separators
        "[a-z0-9]{1,5}[-_][a-z0-9]{1,5}[-_][a-z0-9]{1,5}",
    ]
    .prop_filter("valid ID", |s| {
        !s.is_empty() && s.len() <= IssueId::MAX_LENGTH
    })
}

/// Generate valid issue IDs for vector contexts (simpler)
fn simple_issue_id_strategy() -> impl Strategy<Value = String> {
    "[a-z0-9-]{1,20}".prop_filter("valid ID", |s| {
        !s.is_empty() && s.len() <= IssueId::MAX_LENGTH
    })
}

/// Generate invalid issue IDs (for testing validation)
fn invalid_issue_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just(String::new()),
        // Contains spaces
        Just("invalid id".to_string()),
        // Contains dots
        Just("invalid.id".to_string()),
        // Contains special characters
        Just("invalid@id".to_string()),
    ]
}

/// Generate valid titles (non-empty after trim, within max length)
fn title_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple titles
        "[A-Za-z ]{1,50}",
        // Titles with numbers
        "[A-Za-z0-9 ]{1,100}",
    ]
    .prop_filter("valid title", |s| {
        !s.trim().is_empty() && s.trim().len() <= Title::MAX_LENGTH
    })
    .prop_map(|s| s.trim().to_string())
}

/// Generate invalid titles (for testing validation)
fn invalid_title_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just(String::new()),
        // Only whitespace
        Just("   ".to_string()),
        Just("\t\n".to_string()),
    ]
}

/// Generate valid labels
fn label_strategy() -> impl Strategy<Value = String> {
    "[a-z0-9-]{1,20}".prop_filter("valid label", |s| {
        !s.is_empty() && s.len() <= Labels::MAX_LABEL_LENGTH
    })
}

/// Generate a list of labels within limits
fn labels_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(label_strategy(), 0..Labels::MAX_COUNT)
}

/// Generate too many labels (exceeds limit)
fn too_many_labels_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        label_strategy(),
        Labels::MAX_COUNT + 1..=Labels::MAX_COUNT + 5,
    )
}

/// Generate valid issue states
fn issue_state_strategy() -> impl Strategy<Value = IssueState> {
    prop_oneof![
        Just(IssueState::Open),
        Just(IssueState::InProgress),
        Just(IssueState::Blocked),
        Just(IssueState::Deferred),
        // Closed state with timestamp
        Just(IssueState::Closed {
            closed_at: Utc::now(),
        }),
    ]
}

// ============================================================================
// IssueId Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_valid_issue_id_always_passes(id in issue_id_strategy()) {
        let result = IssueId::new(&id);
        prop_assert!(result.is_ok(), "Valid ID should pass: {}", id);
        let issue_id = result.unwrap();
        prop_assert_eq!(issue_id.as_str(), &id);
    }

    #[test]
    fn prop_invalid_issue_id_always_fails(id in invalid_issue_id_strategy()) {
        let result = IssueId::new(&id);
        prop_assert!(result.is_err(), "Invalid ID should fail: {:?}", id);
    }

    #[test]
    fn prop_issue_id_roundtrip(id in issue_id_strategy()) {
        let issue_id = IssueId::new(&id).unwrap();
        prop_assert_eq!(issue_id.as_str(), &id);
        prop_assert_eq!(issue_id.into_inner(), id);
    }

    #[test]
    fn prop_issue_id_try_from_string(id in issue_id_strategy()) {
        let issue_id = IssueId::try_from(id.clone()).unwrap();
        prop_assert_eq!(issue_id.as_str(), &id);
    }

    #[test]
    fn prop_issue_id_try_from_str(id in issue_id_strategy()) {
        let issue_id = IssueId::try_from(id.as_str()).unwrap();
        prop_assert_eq!(issue_id.as_str(), &id);
    }
}

// ============================================================================
// Title Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_valid_title_always_passes(title in title_strategy()) {
        let result = Title::new(&title);
        prop_assert!(result.is_ok(), "Valid title should pass: {}", title);
        let title_obj = result.unwrap();
        prop_assert_eq!(title_obj.as_str(), &title);
    }

    #[test]
    fn prop_invalid_title_always_fails(title in invalid_title_strategy()) {
        let result = Title::new(&title);
        prop_assert!(result.is_err(), "Invalid title should fail: {:?}", title);
    }

    #[test]
    fn prop_title_roundtrip(title in title_strategy()) {
        let title_obj = Title::new(&title).unwrap();
        prop_assert_eq!(title_obj.as_str(), &title);
        prop_assert_eq!(title_obj.into_inner(), title);
    }

    #[test]
    fn prop_title_trims_whitespace(title in "[A-Za-z]{1,20}") {
        let with_spaces = format!("  {title}  ");
        let title_obj = Title::new(&with_spaces).unwrap();
        prop_assert_eq!(title_obj.as_str(), &title);
        prop_assert!(title_obj.as_str().len() < with_spaces.len());
    }
}

// ============================================================================
// Description Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_valid_description_always_passes(desc in "[A-Za-z0-9 ]{0,5000}") {
        let result = Description::new(&desc);
        prop_assert!(result.is_ok());
    }
}

#[test]
fn prop_too_long_description_fails() {
    let too_long = "a".repeat(Description::MAX_LENGTH + 1);
    let result = Description::new(&too_long);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(DomainError::DescriptionTooLong { .. })
    ));
}

#[test]
fn prop_description_at_max_length() {
    let max_len = "a".repeat(Description::MAX_LENGTH);
    let result = Description::new(&max_len);
    assert!(result.is_ok());
}

// ============================================================================
// Labels Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_valid_labels_always_passes(labels in labels_strategy()) {
        let result = Labels::new(labels.clone());
        prop_assert!(result.is_ok());
        let labels_obj = result.unwrap();
        prop_assert_eq!(labels_obj.len(), labels.len());
    }

    #[test]
    fn prop_too_many_labels_fails(labels in too_many_labels_strategy()) {
        let result = Labels::new(labels);
        prop_assert!(result.is_err());
    }

    #[test]
    fn prop_labels_add_within_limit(label in label_strategy()) {
        let labels = vec![];
        let labels_obj = Labels::new(labels).unwrap();

        // Add one label should succeed
        let result = labels_obj.add(label);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn prop_labels_remove_preserves_validity(label in label_strategy()) {
        let labels = vec![label.clone()];
        let labels_obj = Labels::new(labels).unwrap();
        let new_labels = labels_obj.remove(&label);
        prop_assert!(new_labels.len() <= labels_obj.len());
    }

    #[test]
    fn prop_labels_contains(label in label_strategy()) {
        let labels = vec![label.clone()];
        let labels_obj = Labels::new(labels).unwrap();
        prop_assert!(labels_obj.contains(&label));
    }

    #[test]
    fn prop_labels_iteration(labels in labels_strategy()) {
        let labels_obj = Labels::new(labels.clone()).unwrap();
        prop_assert_eq!(labels_obj.iter().count(), labels.len());
    }
}

// ============================================================================
// IssueState Invariant Tests
// ============================================================================

#[test]
fn prop_closed_state_always_has_timestamp() {
    let dt = Utc::now();
    let state = IssueState::Closed { closed_at: dt };
    assert!(state.is_closed());
    assert_eq!(state.closed_at(), Some(dt));
}

proptest! {
    #[test]
    fn prop_non_closed_states_have_no_timestamp(state in prop_oneof![
        Just(IssueState::Open),
        Just(IssueState::InProgress),
        Just(IssueState::Blocked),
        Just(IssueState::Deferred),
    ]) {
        prop_assert!(!state.is_closed());
        prop_assert!(state.closed_at().is_none());
    }

    #[test]
    fn prop_active_states(state in prop_oneof![
        Just(IssueState::Open),
        Just(IssueState::InProgress),
    ]) {
        prop_assert!(state.is_active());
    }

    #[test]
    fn prop_non_active_states(state in prop_oneof![
        Just(IssueState::Blocked),
        Just(IssueState::Deferred),
        Just(IssueState::Closed { closed_at: Utc::now() }),
    ]) {
        prop_assert!(!state.is_active());
    }

    #[test]
    fn prop_non_blocked_states(state in prop_oneof![
        Just(IssueState::Open),
        Just(IssueState::InProgress),
        Just(IssueState::Deferred),
        Just(IssueState::Closed { closed_at: Utc::now() }),
    ]) {
        prop_assert!(!state.is_blocked());
    }
}

#[test]
fn prop_blocked_state() {
    let state = IssueState::Blocked;
    assert!(state.is_blocked());
}

// ============================================================================
// Issue Aggregate Root Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_issue_creation_valid(id in issue_id_strategy(), title in title_strategy()) {
        let result = Issue::new(&id, &title);
        prop_assert!(result.is_ok());

        let issue = result.unwrap();
        prop_assert_eq!(issue.id.as_str(), &id);
        prop_assert_eq!(issue.title.as_str(), &title);
        prop_assert!(issue.is_active());
        prop_assert!(!issue.is_closed());
        prop_assert!(!issue.is_blocked());
    }

    #[test]
    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn prop_issue_close_sets_timestamp(id in issue_id_strategy(), title in title_strategy()) {
        let mut issue = Issue::new(&id, &title).unwrap();
        prop_assert!(!issue.is_closed());
        prop_assert!(issue.closed_at().is_none());

        issue.close();
        prop_assert!(issue.is_closed());
        prop_assert!(issue.closed_at().is_some());

        let closed_at = issue.closed_at().unwrap();
        let now = Utc::now();
        let diff = (now - closed_at).num_seconds().abs();
        prop_assert!(diff <= 1);
    }

    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn prop_issue_close_with_time(id in issue_id_strategy(), title in title_strategy()) {
        let specific_time = Utc::now() - Duration::days(7);
        let mut issue = Issue::new(&id, &title).unwrap();

        issue.close_with_time(specific_time);
        prop_assert!(issue.is_closed());
        prop_assert_eq!(issue.closed_at(), Some(specific_time));
    }

    #[test]
    fn prop_issue_reopen_from_closed_only(id in issue_id_strategy(), title in title_strategy()) {
        let mut issue = Issue::new(&id, &title).unwrap();

        let result = issue.reopen();
        prop_assert!(result.is_err());

        issue.close();

        let result = issue.reopen();
        prop_assert!(result.is_ok());
        prop_assert!(!issue.is_closed());
        prop_assert!(issue.is_active());
    }

    #[test]
    fn prop_issue_state_transitions(
        id in issue_id_strategy(),
        title in title_strategy(),
        new_state in issue_state_strategy()
    ) {
        let mut issue = Issue::new(&id, &title).unwrap();

        let result = issue.transition_to(new_state);
        prop_assert!(result.is_ok());
        prop_assert_eq!(issue.state, new_state);

        if matches!(new_state, IssueState::Closed { .. }) {
            prop_assert!(issue.is_closed());
            prop_assert!(issue.closed_at().is_some());
        }
    }

    #[test]
    fn prop_cannot_modify_closed_issue_state(id in issue_id_strategy(), title in title_strategy()) {
        let mut issue = Issue::new(&id, &title).unwrap();
        issue.close();

        let closed_at = issue.closed_at();
        prop_assert!(closed_at.is_some());

        let result = issue.transition_to(IssueState::Open);
        prop_assert!(result.is_ok());
        prop_assert!(!issue.is_closed());
    }
}

// ============================================================================
// DependsOn/BlockedBy Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_depends_on_valid_ids(ids in prop::collection::vec("[a-z0-9-]{1,20}", 0..50)) {
        let result = DependsOn::new(ids.clone());
        prop_assert!(result.is_ok());
        let depends_on = result.unwrap();
        prop_assert_eq!(depends_on.len(), ids.len());
    }

    #[test]
    fn prop_blocked_by_valid_ids(ids in prop::collection::vec("[a-z0-9-]{1,20}", 0..50)) {
        let result = BlockedBy::new(ids.clone());
        prop_assert!(result.is_ok());
        let blocked_by = result.unwrap();
        prop_assert_eq!(blocked_by.len(), ids.len());
    }

    #[test]
    fn prop_issue_blocked_by_setters(
        id in issue_id_strategy(),
        title in title_strategy(),
        blockers in prop::collection::vec("[a-z0-9-]{1,20}", 1..10)
    ) {
        let mut issue = Issue::new(&id, &title).unwrap();

        let result = issue.set_blocked_by(blockers.clone());
        prop_assert!(result.is_ok());
        prop_assert!(issue.is_blocked());
        prop_assert_eq!(issue.blocked_by.len(), blockers.len());
    }

    #[test]
    fn prop_issue_depends_on_setters(
        id in issue_id_strategy(),
        title in title_strategy(),
        deps in prop::collection::vec("[a-z0-9-]{1,20}", 1..10)
    ) {
        let mut issue = Issue::new(&id, &title).unwrap();

        let result = issue.set_depends_on(deps.clone());
        prop_assert!(result.is_ok());
        prop_assert_eq!(issue.depends_on.len(), deps.len());
    }
}

#[test]
fn prop_depends_on_too_many_fails() {
    let too_many: Vec<String> = (0..=DependsOn::MAX_COUNT)
        .map(|i| format!("dep-{i}"))
        .collect();
    let result = DependsOn::new(too_many);
    assert!(result.is_err());
}

#[test]
fn prop_blocked_by_too_many_fails() {
    let too_many: Vec<String> = (0..=BlockedBy::MAX_COUNT)
        .map(|i| format!("block-{i}"))
        .collect();
    let result = BlockedBy::new(too_many);
    assert!(result.is_err());
}

// ============================================================================
// Priority Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_priority_roundtrip(n in 0u32..4) {
        let priority = Priority::from_u32(n);
        prop_assert!(priority.is_some());
        let priority = priority.unwrap();
        prop_assert_eq!(priority.to_u32(), n);
    }

    #[test]
    fn prop_priority_invalid_n(n in 5u32..100) {
        let priority = Priority::from_u32(n);
        prop_assert!(priority.is_none());
    }
}

// ============================================================================
// Assignee Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_valid_assignee(name in "[a-zA-Z0-9._%+-]{1,50}") {
        let result = Assignee::new(&name);
        prop_assert!(result.is_ok());
        let assignee = result.unwrap();
        prop_assert_eq!(assignee.as_str(), &name);
    }
}

#[test]
fn prop_empty_assignee_fails() {
    let result = Assignee::new("");
    assert!(result.is_err());
}

#[test]
fn prop_assignee_too_long() {
    let too_long = "a".repeat(Assignee::MAX_LENGTH + 1);
    let result = Assignee::new(&too_long);
    assert!(result.is_err());
}

// ============================================================================
// IssueBuilder Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_builder_with_valid_fields_succeeds(
        id in issue_id_strategy(),
        title in title_strategy(),
        labels in labels_strategy()
    ) {
        let result = IssueBuilder::new()
            .id(&id)
            .title(&title)
            .labels(labels)
            .build();

        prop_assert!(result.is_ok());
        let issue = result.unwrap();
        prop_assert_eq!(issue.id.as_str(), &id);
        prop_assert_eq!(issue.title.as_str(), &title);
    }

    #[test]
    fn prop_builder_requires_id(title in title_strategy()) {
        let result = IssueBuilder::new()
            .title(&title)
            .build();

        prop_assert!(result.is_err());
        prop_assert!(matches!(result, Err(DomainError::EmptyId)));
    }

    #[test]
    fn prop_builder_requires_title(id in issue_id_strategy()) {
        let result = IssueBuilder::new()
            .id(&id)
            .build();

        prop_assert!(result.is_err());
        prop_assert!(matches!(result, Err(DomainError::EmptyTitle)));
    }

    #[test]
    fn prop_builder_closed_state_requires_timestamp(id in issue_id_strategy(), title in title_strategy()) {
        let closed_at = Utc::now();
        let result = IssueBuilder::new()
            .id(&id)
            .title(&title)
            .state(IssueState::Closed { closed_at })
            .build();

        prop_assert!(result.is_ok());
        let issue = result.unwrap();
        prop_assert!(issue.is_closed());
        prop_assert_eq!(issue.closed_at(), Some(closed_at));
    }
}

// ============================================================================
// Timestamp Invariant Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_created_at_before_or_equal_updated_at(id in issue_id_strategy(), title in title_strategy()) {
        let issue = Issue::new(&id, &title).unwrap();
        prop_assert!(issue.created_at <= issue.updated_at);
    }

    #[test]
    fn prop_update_increases_updated_at(id in issue_id_strategy(), title in title_strategy()) {
        let mut issue = Issue::new(&id, &title).unwrap();
        let initial_updated = issue.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        issue.update_title("New title").unwrap();

        prop_assert!(issue.updated_at > initial_updated);
    }

    #[test]
    fn prop_close_updates_updated_at(id in issue_id_strategy(), title in title_strategy()) {
        let mut issue = Issue::new(&id, &title).unwrap();
        let initial_updated = issue.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        issue.close();

        prop_assert!(issue.updated_at > initial_updated);
    }
}
