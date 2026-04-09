//! Queue module tests

use chrono::Utc;

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::queue::entry::QueueEntry;
use crate::domain::queue::queue_impl::Queue;
use crate::domain::queue::status::{QueueStatus, MAX_PRIORITY};

// =========================================================================
// Happy Path Tests
// =========================================================================

#[test]
fn test_queue_new_is_empty() {
    let queue = Queue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_enqueue_adds_entry() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-session", 10).unwrap();
    let new_queue = queue.enqueue(entry);
    assert_eq!(new_queue.len(), 1);
    // Original queue is unchanged
    assert!(queue.is_empty());
}

#[test]
fn test_enqueue_maintains_priority_order() {
    let queue = Queue::new();

    let queue = queue.enqueue(QueueEntry::new("low", "low-priority", 100).unwrap());
    let queue = queue.enqueue(QueueEntry::new("high", "high-priority", 1).unwrap());
    let queue = queue.enqueue(QueueEntry::new("medium", "medium-priority", 50).unwrap());

    let priorities: Vec<_> = queue.entries().iter().map(|e| e.priority).collect();
    assert_eq!(priorities, vec![1, 50, 100]);
}

#[test]
fn test_dequeue_removes_entry() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-session", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("test-1").unwrap();
    let (new_queue, removed) = queue.dequeue(&id);

    assert!(removed.is_some());
    assert!(new_queue.is_empty());
    // Original queue is unchanged
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_dequeue_returns_none_for_nonexistent() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-session", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("nonexistent").unwrap();
    let (new_queue, removed) = queue.dequeue(&id);

    assert!(removed.is_none());
    // Queue should be unchanged when entry not found
    assert_eq!(new_queue.len(), 1);
}

#[test]
fn test_find_returns_entry() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-session", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("test-1").unwrap();
    let found = queue.find(&id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id.as_str(), "test-1");
}

#[test]
fn test_find_returns_none_for_nonexistent() {
    let queue = Queue::new();
    let id = QueueEntryId::new("nonexistent").unwrap();
    let found = queue.find(&id);
    assert!(found.is_none());
}

#[test]
fn test_find_by_session_returns_entry() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "my-session", 10).unwrap();
    let queue = queue.enqueue(entry);

    let session = SessionName::parse("my-session").unwrap();
    let found = queue.find_by_session(&session);
    assert!(found.is_some());
    assert_eq!(found.unwrap().session.as_str(), "my-session");
}

#[test]
fn test_next_pending_returns_pending_entry() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "session-1", 10).unwrap();
    let queue = queue.enqueue(entry);

    let next = queue.next_pending();
    assert!(next.is_some());
    assert_eq!(next.unwrap().status, QueueStatus::Pending);
}

#[test]
fn test_next_pending_returns_none_when_no_pending() {
    let queue = Queue::new();
    // Create an entry with a terminal status via valid transitions
    let entry = QueueEntry::with_timestamp(
        QueueEntryId::new("test-1").unwrap(),
        SessionName::parse("session-1").unwrap(),
        10,
        Utc::now(),
    )
    .unwrap()
    .transition_status(QueueStatus::Claimed)
    .unwrap()
    .transition_status(QueueStatus::Rebasing)
    .unwrap()
    .transition_status(QueueStatus::Testing)
    .unwrap()
    .transition_status(QueueStatus::ReadyToMerge)
    .unwrap()
    .transition_status(QueueStatus::Merging)
    .unwrap()
    .transition_status(QueueStatus::Merged)
    .unwrap();
    let queue = queue.enqueue(entry);

    let next = queue.next_pending();
    assert!(next.is_none());
}

#[test]
fn test_with_entry_at_valid_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let new_entry = QueueEntry::new("inserted", "inserted", 5).unwrap();
    let result = queue.with_entry(0, new_entry);

    assert!(result.is_ok());
    let new_queue = result.unwrap();
    assert_eq!(new_queue.len(), 2);
    // Original unchanged
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_with_entry_at_end_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let new_entry = QueueEntry::new("second", "second", 5).unwrap();
    let result = queue.with_entry(1, new_entry);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

// =========================================================================
// Error Path Tests
// =========================================================================

#[test]
fn test_with_entry_at_invalid_position_returns_error() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let new_entry = QueueEntry::new("out-of-bounds", "out-of-bounds", 5).unwrap();
    let result = queue.with_entry(5, new_entry);

    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::OutOfBounds { .. })
    ));
}

// =========================================================================
// QueueStatus Transition Tests
// =========================================================================

#[test]
fn test_status_pending_to_claimed_transition() {
    let status = QueueStatus::Pending;
    let result = status.transition_to(QueueStatus::Claimed);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), QueueStatus::Claimed);
}

#[test]
fn test_status_pending_to_cancelled_transition() {
    let status = QueueStatus::Pending;
    let result = status.transition_to(QueueStatus::Cancelled);
    assert!(result.is_ok());
}

#[test]
fn test_status_pending_to_merged_invalid_transition() {
    let status = QueueStatus::Pending;
    let result = status.transition_to(QueueStatus::Merged);
    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::InvalidStateTransition { .. })
    ));
}

#[test]
fn test_status_claimed_to_rebasing_transition() {
    let status = QueueStatus::Claimed;
    let result = status.transition_to(QueueStatus::Rebasing);
    assert!(result.is_ok());
}

#[test]
fn test_status_rebasing_to_testing_transition() {
    let status = QueueStatus::Rebasing;
    let result = status.transition_to(QueueStatus::Testing);
    assert!(result.is_ok());
}

#[test]
fn test_status_testing_to_ready_to_merge_transition() {
    let status = QueueStatus::Testing;
    let result = status.transition_to(QueueStatus::ReadyToMerge);
    assert!(result.is_ok());
}

#[test]
fn test_status_ready_to_merge_to_merging_transition() {
    let status = QueueStatus::ReadyToMerge;
    let result = status.transition_to(QueueStatus::Merging);
    assert!(result.is_ok());
}

#[test]
fn test_status_merging_to_merged_transition() {
    let status = QueueStatus::Merging;
    let result = status.transition_to(QueueStatus::Merged);
    assert!(result.is_ok());
}

#[test]
fn test_status_merged_is_terminal() {
    assert!(QueueStatus::Merged.is_terminal());
}

#[test]
fn test_status_failed_terminal_is_terminal() {
    assert!(QueueStatus::FailedTerminal.is_terminal());
}

#[test]
fn test_status_cancelled_is_terminal() {
    assert!(QueueStatus::Cancelled.is_terminal());
}

#[test]
fn test_status_pending_is_not_terminal() {
    assert!(!QueueStatus::Pending.is_terminal());
}

#[test]
fn test_status_failed_is_failed() {
    assert!(QueueStatus::FailedRetryable.is_failed());
    assert!(QueueStatus::FailedTerminal.is_failed());
}

#[test]
fn test_status_pending_is_not_failed() {
    assert!(!QueueStatus::Pending.is_failed());
}

// =========================================================================
// QueueEntry Tests
// =========================================================================

#[test]
fn test_queue_entry_new_rejects_high_priority() {
    let result = QueueEntry::new("test-1", "session", 101);
    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::ExceedsMaximum { .. })
    ));
}

#[test]
fn test_queue_entry_new_accepts_max_priority() {
    let result = QueueEntry::new("test-1", "session", 100);
    assert!(result.is_ok());
}

#[test]
fn test_queue_entry_new_rejects_empty_id() {
    let result = QueueEntry::new("", "session", 10);
    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::EmptyValue(_))
    ));
}

#[test]
fn test_queue_entry_new_rejects_empty_session() {
    let result = QueueEntry::new("test-1", "", 10);
    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::EmptyValue(_))
    ));
}

#[test]
fn test_queue_entry_transition_status_valid() {
    let entry = QueueEntry::new("test-1", "session", 10).unwrap();
    let updated = entry.transition_status(QueueStatus::Claimed);
    assert!(updated.is_ok());
    assert_eq!(updated.unwrap().status, QueueStatus::Claimed);
}

#[test]
fn test_queue_entry_transition_status_invalid() {
    let entry = QueueEntry::new("test-1", "session", 10).unwrap();
    let updated = entry.transition_status(QueueStatus::Merged);
    assert!(matches!(
        updated,
        Err(crate::domain::validation::ValidationError::InvalidStateTransition { .. })
    ));
}

#[test]
fn test_queue_entry_with_priority_valid() {
    let entry = QueueEntry::new("test-1", "session", 10).unwrap();
    let updated = entry.with_priority(50);
    assert!(updated.is_ok());
    assert_eq!(updated.unwrap().priority, 50);
}

#[test]
fn test_queue_entry_with_priority_invalid() {
    let entry = QueueEntry::new("test-1", "session", 10).unwrap();
    let updated = entry.with_priority(101);
    assert!(matches!(
        updated,
        Err(crate::domain::validation::ValidationError::ExceedsMaximum { .. })
    ));
}

// =========================================================================
// Functional Combinator Tests
// =========================================================================

#[test]
fn test_filter_pending_entries() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let entry2 = QueueEntry::new("test-2", "session-2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap();
    let queue = queue.enqueue(entry2);

    let pending = queue.filter(|e| e.status == QueueStatus::Pending);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id.as_str(), "test-1");
}

#[test]
fn test_map_entry_ids() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "session-2", 20).unwrap());

    let ids: Vec<String> = queue.map(|e| e.id.as_str().to_string());
    assert_eq!(ids, vec!["test-1", "test-2"]);
}

#[test]
fn test_fold_total_priority() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "session-2", 20).unwrap());

    let total = queue.fold(0, |acc, e| acc + e.priority);
    assert_eq!(total, 30);
}

#[test]
fn test_any_has_high_priority() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 5).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "session-2", 20).unwrap());

    assert!(queue.any(|e| e.priority < 10));
    assert!(!queue.any(|e| e.priority > 100));
}

#[test]
fn test_all_have_valid_priority() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "session-2", 20).unwrap());

    assert!(queue.all(|e| e.priority <= MAX_PRIORITY));
}

#[test]
fn test_group_by_status() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let entry2 = QueueEntry::new("test-2", "session-2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap();
    let queue = queue.enqueue(entry2);

    let grouped = queue.group_by_status();
    assert_eq!(grouped.len(), 2);
}

#[test]
fn test_count_active_excludes_terminal() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    // Create an entry with a terminal status via valid transitions: Pending -> Claimed -> Rebasing -> Testing -> ReadyToMerge -> Merging -> Merged
    let entry2 = QueueEntry::new("test-2", "session-2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap()
        .transition_status(QueueStatus::Rebasing)
        .unwrap()
        .transition_status(QueueStatus::Testing)
        .unwrap()
        .transition_status(QueueStatus::ReadyToMerge)
        .unwrap()
        .transition_status(QueueStatus::Merging)
        .unwrap()
        .transition_status(QueueStatus::Merged)
        .unwrap();
    let queue = queue.enqueue(entry2);

    let count = queue.count_active();
    assert_eq!(count, 1);
}

#[test]
fn test_sorted_by_session_name() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "charlie", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "alpha", 20).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-3", "bravo", 15).unwrap());

    let sorted = queue.sorted_by_key(|e| e.session.to_string());
    assert_eq!(sorted[0].session.as_str(), "alpha");
    assert_eq!(sorted[1].session.as_str(), "bravo");
    assert_eq!(sorted[2].session.as_str(), "charlie");
}

#[test]
fn test_partition_by_status() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());
    let entry2 = QueueEntry::new("test-2", "session-2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap();
    let queue = queue.enqueue(entry2);

    let (claimed, pending) = queue.partition(|e| e.status == QueueStatus::Claimed);
    assert_eq!(claimed.len(), 1);
    assert_eq!(pending.len(), 1);
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
fn test_multiple_entries_same_priority() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("a", "a", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("b", "b", 10).unwrap());

    assert_eq!(queue.len(), 2);
}

#[test]
fn test_dequeue_all_entries() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "test-1", 10).unwrap());
    let queue = queue.enqueue(QueueEntry::new("test-2", "test-2", 20).unwrap());

    let id1 = QueueEntryId::new("test-1").unwrap();
    let (queue, _) = queue.dequeue(&id1);

    let id2 = QueueEntryId::new("test-2").unwrap();
    let (queue, _) = queue.dequeue(&id2);

    assert!(queue.is_empty());
}

// =========================================================================
// Immutability Tests
// =========================================================================

#[test]
fn test_enqueue_preserves_original() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "session-1", 10).unwrap();
    let new_queue = queue.enqueue(entry.clone());

    assert_eq!(queue.len(), 0);
    assert_eq!(new_queue.len(), 1);
}

#[test]
fn test_dequeue_preserves_original() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "session-1", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("test-1").unwrap();
    let (new_queue, removed) = queue.dequeue(&id);

    assert_eq!(queue.len(), 1);
    assert_eq!(new_queue.len(), 0);
    assert!(removed.is_some());
}

#[test]
fn test_with_entry_preserves_original() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let new_entry = QueueEntry::new("second", "second", 5).unwrap();
    let new_queue = queue.with_entry(1, new_entry).unwrap();

    assert_eq!(queue.len(), 1);
    assert_eq!(new_queue.len(), 2);
}

// =========================================================================
// Railway-Oriented Programming Tests
// =========================================================================

#[test]
fn test_update_status_valid_chain() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());

    let id = QueueEntryId::new("test-1").unwrap();
    let result = queue.update_status(&id, QueueStatus::Claimed);

    assert!(result.is_ok());
    let new_queue = result.unwrap();
    assert_eq!(new_queue.find(&id).unwrap().status, QueueStatus::Claimed);
    // Original unchanged
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Pending);
}

#[test]
fn test_update_status_invalid_entry_returns_error() {
    let queue = Queue::new();
    let id = QueueEntryId::new("nonexistent").unwrap();
    let result = queue.update_status(&id, QueueStatus::Claimed);

    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::NotFound { .. })
    ));
}

#[test]
fn test_update_status_invalid_transition_returns_error() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());

    let id = QueueEntryId::new("test-1").unwrap();
    let result = queue.update_status(&id, QueueStatus::Merged);

    assert!(matches!(
        result,
        Err(crate::domain::validation::ValidationError::InvalidStateTransition { .. })
    ));
}

// =========================================================================
// Contract Verification Tests
// =========================================================================

#[test]
fn test_invariant_priority_order_maintained_after_enqueues() {
    let queue = Queue::new();
    let queue = (0..10).rev().fold(queue, |acc, i| {
        acc.enqueue(QueueEntry::new(format!("id-{}", i), format!("session-{}", i), i).unwrap())
    });

    let priorities: Vec<_> = queue.entries().iter().map(|e| e.priority).collect();
    for window in priorities.windows(2) {
        assert!(window[0] <= window[1]);
    }
}

#[test]
fn test_invariant_queue_len_matches_entries_count() {
    let queue = Queue::new();
    assert_eq!(queue.len(), queue.entries().len());

    let queue = queue.enqueue(QueueEntry::new("test-1", "test-1", 10).unwrap());
    assert_eq!(queue.len(), queue.entries().len());

    let id = QueueEntryId::new("test-1").unwrap();
    let (queue, _) = queue.dequeue(&id);
    assert_eq!(queue.len(), queue.entries().len());
}

#[test]
fn test_invariant_dequeue_of_nonexistent_preserves_queue() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-1", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("nonexistent").unwrap();
    let (new_queue, removed) = queue.dequeue(&id);

    assert!(removed.is_none());
    // When entry not found, queue should be cloned
    assert_eq!(new_queue.len(), queue.len());
}

// =========================================================================
// remove_at Tests
// =========================================================================

#[test]
fn test_remove_at_valid_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let result = queue.remove_at(0);
    assert!(result.is_ok());
    let (new_queue, removed) = result.unwrap();
    assert_eq!(removed.id.as_str(), "first");
    assert!(new_queue.is_empty());
    // Original unchanged
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_remove_at_invalid_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let result = queue.remove_at(5);
    assert!(result.is_err());
}

// =========================================================================
// QueueEntryId and SessionName Tests (from identifiers.rs)
// =========================================================================

#[test]
fn test_queue_entry_id_valid() {
    assert!(QueueEntryId::new("test-123").is_ok());
    assert!(QueueEntryId::new("  test-123  ").is_ok());
}

#[test]
fn test_queue_entry_id_empty() {
    assert!(matches!(
        QueueEntryId::new(""),
        Err(crate::domain::validation::ValidationError::EmptyValue(_))
    ));
    assert!(matches!(
        QueueEntryId::new("   "),
        Err(crate::domain::validation::ValidationError::EmptyValue(_))
    ));
}

#[test]
fn test_session_name_valid() {
    assert!(SessionName::parse("my-session").is_ok());
    assert!(SessionName::parse("  my-session  ").is_ok());
    assert!(SessionName::parse("session_123").is_ok());
}

#[test]
fn test_session_name_empty() {
    assert!(matches!(
        SessionName::parse(""),
        Err(crate::domain::identifiers::IdentifierError::Empty)
    ));
    assert!(matches!(
        SessionName::parse("   "),
        Err(crate::domain::identifiers::IdentifierError::Empty)
    ));
}

#[test]
fn test_session_name_rejects_shell_metacharacters() {
    let invalid_chars = ["$", "`", "|", "&", "<", ">", "\n", "\r", "\x00"];
    for c in invalid_chars {
        let test_name = format!("session{}name", c);
        assert!(
            matches!(
                SessionName::parse(&test_name),
                Err(crate::domain::identifiers::IdentifierError::InvalidCharacters { .. })
            ),
            "Should reject character: {:?}",
            c
        );
    }
}

#[test]
fn test_session_name_validate_works() {
    assert!(SessionName::parse("valid-name").is_ok());
    assert!(SessionName::parse("invalid$name").is_err());
}

#[test]
fn test_session_name_try_from() {
    assert!(SessionName::try_from("valid".to_string()).is_ok());
    assert!(SessionName::try_from("valid").is_ok());
    assert!(SessionName::try_from("").is_err());
}

// =========================================================================
// from_identifiers Tests
// =========================================================================

#[test]
fn test_queue_entry_from_identifiers() {
    let id = QueueEntryId::new("test-1").unwrap();
    let session = SessionName::parse("session").unwrap();
    let entry = QueueEntry::from_identifiers(id, session, 50).unwrap();
    assert_eq!(entry.id.as_str(), "test-1");
    assert_eq!(entry.session.as_str(), "session");
    assert_eq!(entry.priority, 50);
    assert_eq!(entry.status, QueueStatus::Pending);
}

// =========================================================================
// with_timestamp and with_status Tests
// =========================================================================

#[test]
fn test_queue_entry_with_timestamp() {
    let id = QueueEntryId::new("test-1").unwrap();
    let session = SessionName::parse("session").unwrap();
    let timestamp = Utc::now();
    let entry = QueueEntry::with_timestamp(id, session, 50, timestamp).unwrap();
    assert_eq!(entry.enqueued_at, timestamp);
}

#[test]
fn test_queue_entry_with_status() {
    let id = QueueEntryId::new("test-1").unwrap();
    let session = SessionName::parse("session").unwrap();
    let timestamp = Utc::now();
    let entry = QueueEntry::with_status(id, session, 50, timestamp, QueueStatus::Claimed).unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);
}

// =========================================================================
// Exhaustive QueueStatus state machine tests (ha-oeea)
// =========================================================================

/// All 10 status variants for exhaustive iteration.
const ALL_SM_STATUSES: [QueueStatus; 10] = [
    QueueStatus::Pending,
    QueueStatus::Claimed,
    QueueStatus::Rebasing,
    QueueStatus::Testing,
    QueueStatus::ReadyToMerge,
    QueueStatus::Merging,
    QueueStatus::Merged,
    QueueStatus::FailedRetryable,
    QueueStatus::FailedTerminal,
    QueueStatus::Cancelled,
];

// ── Display ──────────────────────────────────────────────────────────────

#[test]
fn test_sm_display_all_variants() {
    use std::fmt::Write;
    let mut buf = String::new();
    for status in ALL_SM_STATUSES {
        buf.clear();
        write!(&mut buf, "{}", status).unwrap();
        assert!(
            !buf.is_empty(),
            "Display produced empty string for {:?}",
            status
        );
    }
}

#[test]
fn test_sm_display_matches_expected_strings() {
    assert_eq!(format!("{}", QueueStatus::Pending), "pending");
    assert_eq!(format!("{}", QueueStatus::Claimed), "claimed");
    assert_eq!(format!("{}", QueueStatus::Rebasing), "rebasing");
    assert_eq!(format!("{}", QueueStatus::Testing), "testing");
    assert_eq!(format!("{}", QueueStatus::ReadyToMerge), "ready_to_merge");
    assert_eq!(format!("{}", QueueStatus::Merging), "merging");
    assert_eq!(format!("{}", QueueStatus::Merged), "merged");
    assert_eq!(
        format!("{}", QueueStatus::FailedRetryable),
        "failed_retryable"
    );
    assert_eq!(
        format!("{}", QueueStatus::FailedTerminal),
        "failed_terminal"
    );
    assert_eq!(format!("{}", QueueStatus::Cancelled), "cancelled");
}

// ── Predicates ───────────────────────────────────────────────────────────

#[test]
fn test_sm_is_terminal_exhaustive() {
    for status in ALL_SM_STATUSES {
        let expected = matches!(
            status,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        );
        assert_eq!(
            status.is_terminal(),
            expected,
            "is_terminal wrong for {:?}",
            status
        );
    }
}

#[test]
fn test_sm_is_failed_exhaustive() {
    for status in ALL_SM_STATUSES {
        let expected = matches!(
            status,
            QueueStatus::FailedRetryable | QueueStatus::FailedTerminal
        );
        assert_eq!(
            status.is_failed(),
            expected,
            "is_failed wrong for {:?}",
            status
        );
    }
}

#[test]
fn test_sm_terminal_and_failed_disjoint() {
    // FailedRetryable is failed but NOT terminal
    assert!(QueueStatus::FailedRetryable.is_failed());
    assert!(!QueueStatus::FailedRetryable.is_terminal());
    // FailedTerminal is both failed AND terminal
    assert!(QueueStatus::FailedTerminal.is_failed());
    assert!(QueueStatus::FailedTerminal.is_terminal());
}

// ── Transition matrix ────────────────────────────────────────────────────

/// The full happy path: Pending → Claimed → Rebasing → Testing → ReadyToMerge → Merging → Merged
#[test]
fn test_sm_full_happy_path() {
    let chain = [
        (QueueStatus::Pending, QueueStatus::Claimed),
        (QueueStatus::Claimed, QueueStatus::Rebasing),
        (QueueStatus::Rebasing, QueueStatus::Testing),
        (QueueStatus::Testing, QueueStatus::ReadyToMerge),
        (QueueStatus::ReadyToMerge, QueueStatus::Merging),
        (QueueStatus::Merging, QueueStatus::Merged),
    ];
    for (from, to) in &chain {
        let result = from.transition_to(*to);
        assert!(
            result.is_ok(),
            "Transition {:?} → {:?} should succeed",
            from,
            to
        );
        assert_eq!(result.unwrap(), *to);
    }
}

/// Cancel from every state that allows it (based on actual transition rules)
#[test]
fn test_sm_cancel_from_all_cancellable() {
    // From the state machine: Pending, Claimed, FailedRetryable can cancel
    let cancellable = [
        QueueStatus::Pending,
        QueueStatus::Claimed,
        QueueStatus::FailedRetryable,
    ];
    for status in &cancellable {
        let result = status.transition_to(QueueStatus::Cancelled);
        assert!(result.is_ok(), "Cancel from {:?} should succeed", status);
        assert_eq!(result.unwrap(), QueueStatus::Cancelled);
    }
}

/// All transitions from terminal states are rejected
#[test]
fn test_sm_terminal_states_reject_all_transitions() {
    let terminal = [
        QueueStatus::Merged,
        QueueStatus::FailedTerminal,
        QueueStatus::Cancelled,
    ];
    for from in &terminal {
        for to in ALL_SM_STATUSES {
            // Same-state transitions also rejected for terminals
            let result = from.transition_to(to);
            assert!(
                result.is_err(),
                "Transition {:?} → {:?} should be rejected",
                from,
                to
            );
        }
    }
}

/// Self-transitions are rejected for all states
#[test]
fn test_sm_self_transitions_rejected() {
    for status in ALL_SM_STATUSES {
        let result = status.transition_to(status);
        assert!(
            result.is_err(),
            "Self-transition {:?} → {:?} should be rejected",
            status,
            status
        );
    }
}

/// Exhaustive invalid transition matrix: test every (from, to) pair that should fail
#[test]
fn test_sm_exhaustive_invalid_transitions() {
    // Define the valid transitions as a set (from the actual state machine)
    let valid: std::collections::HashSet<(QueueStatus, QueueStatus)> = [
        // Pending
        (QueueStatus::Pending, QueueStatus::Claimed),
        (QueueStatus::Pending, QueueStatus::Cancelled),
        // Claimed
        (QueueStatus::Claimed, QueueStatus::Rebasing),
        (QueueStatus::Claimed, QueueStatus::Cancelled),
        // Rebasing
        (QueueStatus::Rebasing, QueueStatus::Testing),
        (QueueStatus::Rebasing, QueueStatus::FailedRetryable),
        // Testing
        (QueueStatus::Testing, QueueStatus::ReadyToMerge),
        (QueueStatus::Testing, QueueStatus::FailedRetryable),
        (QueueStatus::Testing, QueueStatus::FailedTerminal),
        // ReadyToMerge
        (QueueStatus::ReadyToMerge, QueueStatus::Merging),
        (QueueStatus::ReadyToMerge, QueueStatus::FailedRetryable),
        // Merging
        (QueueStatus::Merging, QueueStatus::Merged),
        (QueueStatus::Merging, QueueStatus::FailedRetryable),
        // FailedRetryable
        (QueueStatus::FailedRetryable, QueueStatus::Pending),
        (QueueStatus::FailedRetryable, QueueStatus::Cancelled),
    ]
    .into_iter()
    .collect();

    let mut invalid_count = 0;
    for from in ALL_SM_STATUSES {
        for to in ALL_SM_STATUSES {
            if from == to || valid.contains(&(from, to)) {
                continue;
            }
            let result = from.transition_to(to);
            assert!(
                result.is_err(),
                "Invalid transition {:?} → {:?} should fail",
                from,
                to
            );
            // Verify error contains correct from/to strings
            if let Err(crate::domain::validation::ValidationError::InvalidStateTransition {
                from: f,
                to: t,
            }) = result
            {
                assert_eq!(f, format!("{}", from));
                assert_eq!(t, format!("{}", to));
            } else {
                panic!(
                    "Expected InvalidStateTransition error for {:?} → {:?}",
                    from, to
                );
            }
            invalid_count += 1;
        }
    }
    // Sanity: should have many invalid transitions
    assert!(
        invalid_count > 50,
        "Expected many invalid transitions, got {}",
        invalid_count
    );
}

// ── Failure paths ────────────────────────────────────────────────────────

/// Testing → FailedRetryable
#[test]
fn test_sm_testing_to_failed_retryable() {
    let result = QueueStatus::Testing.transition_to(QueueStatus::FailedRetryable);
    assert!(result.is_ok());
}

/// Testing → FailedTerminal
#[test]
fn test_sm_testing_to_failed_terminal() {
    let result = QueueStatus::Testing.transition_to(QueueStatus::FailedTerminal);
    assert!(result.is_ok());
}

/// Rebasing → FailedRetryable
#[test]
fn test_sm_rebasing_to_failed_retryable() {
    let result = QueueStatus::Rebasing.transition_to(QueueStatus::FailedRetryable);
    assert!(result.is_ok());
}

/// ReadyToMerge → FailedRetryable
#[test]
fn test_sm_ready_to_merge_to_failed_retryable() {
    let result = QueueStatus::ReadyToMerge.transition_to(QueueStatus::FailedRetryable);
    assert!(result.is_ok());
}

/// Merging → FailedRetryable
#[test]
fn test_sm_merging_to_failed_retryable() {
    let result = QueueStatus::Merging.transition_to(QueueStatus::FailedRetryable);
    assert!(result.is_ok());
}

/// FailedRetryable → Pending (retry)
#[test]
fn test_sm_failed_retryable_to_pending() {
    let result = QueueStatus::FailedRetryable.transition_to(QueueStatus::Pending);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), QueueStatus::Pending);
}

// ── Retry cycle through queue entry ──────────────────────────────────────

/// Full retry cycle: Pending → Claimed → Rebasing → Testing → FailedRetryable → Pending → Claimed → ... → Merged
#[test]
fn test_sm_entry_retry_cycle() {
    let entry = QueueEntry::new("retry-test", "session", 10).unwrap();

    // First attempt: fail at Testing
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry
        .transition_status(QueueStatus::FailedRetryable)
        .unwrap();
    assert!(entry.status.is_failed());
    assert!(!entry.status.is_terminal());

    // Retry: go back to Pending and succeed
    let entry = entry.transition_status(QueueStatus::Pending).unwrap();
    assert_eq!(entry.status, QueueStatus::Pending);

    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert!(entry.status.is_terminal());
}

/// Failure at Rebasing → retry → success
#[test]
fn test_sm_entry_fail_at_rebasing_retry_success() {
    let entry = QueueEntry::new("rebasing-fail", "session", 10).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry
        .transition_status(QueueStatus::FailedRetryable)
        .unwrap();

    // Retry
    let entry = entry.transition_status(QueueStatus::Pending).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert!(entry.status.is_terminal());
}

/// Failure at ReadyToMerge → retry → success
#[test]
fn test_sm_entry_fail_at_ready_to_merge_retry_success() {
    let entry = QueueEntry::new("rtm-fail", "session", 10).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry
        .transition_status(QueueStatus::FailedRetryable)
        .unwrap();

    // Retry
    let entry = entry.transition_status(QueueStatus::Pending).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert!(entry.status.is_terminal());
}

/// Failure at Merging → retry → success
#[test]
fn test_sm_entry_fail_at_merging_retry_success() {
    let entry = QueueEntry::new("merge-fail", "session", 10).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    let entry = entry
        .transition_status(QueueStatus::FailedRetryable)
        .unwrap();

    // Retry
    let entry = entry.transition_status(QueueStatus::Pending).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert!(entry.status.is_terminal());
}

// ── Terminal failure paths ───────────────────────────────────────────────

/// Testing → FailedTerminal (non-recoverable)
#[test]
fn test_sm_entry_terminal_failure_from_testing() {
    let entry = QueueEntry::new("terminal-fail", "session", 10).unwrap();
    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    let entry = entry
        .transition_status(QueueStatus::FailedTerminal)
        .unwrap();

    assert!(entry.status.is_terminal());
    assert!(entry.status.is_failed());

    // No transitions from FailedTerminal
    let result = entry.transition_status(QueueStatus::Pending);
    assert!(result.is_err());
}

// ── Cancel paths ─────────────────────────────────────────────────────────

/// Cancel from each stage that allows cancellation
#[test]
fn test_sm_cancel_at_each_stage() {
    // Only Pending, Claimed, and FailedRetryable can cancel directly
    let cancellable = [
        QueueStatus::Pending,
        QueueStatus::Claimed,
        QueueStatus::FailedRetryable,
    ];

    for cancel_from in cancellable {
        let entry = QueueEntry::new("cancel-test", "session", 10).unwrap();
        // Transition to the target state
        let entry = match cancel_from {
            QueueStatus::Pending => entry,
            QueueStatus::Claimed => entry.transition_status(QueueStatus::Claimed).unwrap(),
            QueueStatus::FailedRetryable => entry
                .transition_status(QueueStatus::Claimed)
                .unwrap()
                .transition_status(QueueStatus::Rebasing)
                .unwrap()
                .transition_status(QueueStatus::Testing)
                .unwrap()
                .transition_status(QueueStatus::FailedRetryable)
                .unwrap(),
            _ => entry,
        };

        let cancelled = entry.transition_status(QueueStatus::Cancelled);
        assert!(
            cancelled.is_ok(),
            "Cancel from {:?} should succeed",
            cancel_from
        );
        assert!(cancelled.unwrap().status.is_terminal());
    }
}

// ── Serde roundtrip for all variants ─────────────────────────────────────

#[test]
fn test_sm_serde_roundtrip_all_variants() {
    for status in ALL_SM_STATUSES {
        let json = serde_json::to_string(&status).unwrap();
        let back: QueueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back, "Serde roundtrip failed for {:?}", status);
    }
}

#[test]
fn test_sm_serde_format_pascal_case() {
    assert_eq!(
        serde_json::to_string(&QueueStatus::Pending).unwrap(),
        "\"Pending\""
    );
    assert_eq!(
        serde_json::to_string(&QueueStatus::FailedRetryable).unwrap(),
        "\"FailedRetryable\""
    );
    assert_eq!(
        serde_json::to_string(&QueueStatus::ReadyToMerge).unwrap(),
        "\"ReadyToMerge\""
    );
}

#[test]
fn test_sm_serde_rejects_invalid() {
    assert!(serde_json::from_str::<QueueStatus>("\"invalid\"").is_err());
    assert!(serde_json::from_str::<QueueStatus>("null").is_err());
    assert!(serde_json::from_str::<QueueStatus>("42").is_err());
    assert!(serde_json::from_str::<QueueStatus>("\"pending\"").is_err()); // lowercase
}

// ── Copy, Clone, Hash ───────────────────────────────────────────────────

#[test]
fn test_sm_copy_semantics() {
    for status in ALL_SM_STATUSES {
        let copied = status;
        assert_eq!(status, copied);
    }
}

#[test]
fn test_sm_clone_semantics() {
    for status in ALL_SM_STATUSES {
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }
}

#[test]
fn test_sm_hash_deduplication() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for status in ALL_SM_STATUSES {
        set.insert(status);
    }
    assert_eq!(set.len(), 10);

    // Insert duplicates
    for status in ALL_SM_STATUSES {
        set.insert(status);
    }
    assert_eq!(set.len(), 10);
}

// ── Queue-level status tracking ──────────────────────────────────────────

#[test]
fn test_sm_queue_status_tracking_through_transitions() {
    let queue = Queue::new();

    // Enqueue pending entry
    let queue = queue.enqueue(QueueEntry::new("track-1", "sess-1", 10).unwrap());
    assert_eq!(
        queue
            .find(&QueueEntryId::new("track-1").unwrap())
            .unwrap()
            .status,
        QueueStatus::Pending
    );

    // Transition to Claimed
    let id = QueueEntryId::new("track-1").unwrap();
    let queue = queue.update_status(&id, QueueStatus::Claimed).unwrap();
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Claimed);

    // Transition through full path
    let queue = queue.update_status(&id, QueueStatus::Rebasing).unwrap();
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Rebasing);

    let queue = queue.update_status(&id, QueueStatus::Testing).unwrap();
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Testing);

    let queue = queue.update_status(&id, QueueStatus::ReadyToMerge).unwrap();
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::ReadyToMerge);

    let queue = queue.update_status(&id, QueueStatus::Merging).unwrap();
    assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Merging);

    let queue = queue.update_status(&id, QueueStatus::Merged).unwrap();
    assert!(queue.find(&id).unwrap().status.is_terminal());

    // count_active should be 0 (all terminal)
    assert_eq!(queue.count_active(), 0);
}

#[test]
fn test_sm_queue_multiple_entries_different_statuses() {
    let queue = Queue::new();

    // Entry 1: pending
    let queue = queue.enqueue(QueueEntry::new("e1", "s1", 10).unwrap());

    // Entry 2: advance to Testing
    let entry2 = QueueEntry::new("e2", "s2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap()
        .transition_status(QueueStatus::Rebasing)
        .unwrap()
        .transition_status(QueueStatus::Testing)
        .unwrap();
    let queue = queue.enqueue(entry2);

    // Entry 3: advance to Merged (terminal)
    let entry3 = QueueEntry::new("e3", "s3", 30)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap()
        .transition_status(QueueStatus::Rebasing)
        .unwrap()
        .transition_status(QueueStatus::Testing)
        .unwrap()
        .transition_status(QueueStatus::ReadyToMerge)
        .unwrap()
        .transition_status(QueueStatus::Merging)
        .unwrap()
        .transition_status(QueueStatus::Merged)
        .unwrap();
    let queue = queue.enqueue(entry3);

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.count_active(), 2); // e1 (Pending) + e2 (Testing)

    let grouped = queue.group_by_status();
    assert_eq!(grouped.len(), 3); // Pending, Testing, Merged
}

#[test]
fn test_sm_queue_filter_by_terminal_vs_active() {
    let queue = Queue::new();

    let queue = queue.enqueue(QueueEntry::new("active-1", "s1", 10).unwrap());

    let claimed = QueueEntry::new("active-2", "s2", 20)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap();
    let queue = queue.enqueue(claimed);

    // Terminal: Merged (full chain)
    let merged = QueueEntry::new("term-1", "s3", 30)
        .unwrap()
        .transition_status(QueueStatus::Claimed)
        .unwrap()
        .transition_status(QueueStatus::Rebasing)
        .unwrap()
        .transition_status(QueueStatus::Testing)
        .unwrap()
        .transition_status(QueueStatus::ReadyToMerge)
        .unwrap()
        .transition_status(QueueStatus::Merging)
        .unwrap()
        .transition_status(QueueStatus::Merged)
        .unwrap();
    let queue = queue.enqueue(merged);

    // Terminal: Cancelled (from Pending)
    let cancelled = QueueEntry::new("term-2", "s4", 40)
        .unwrap()
        .transition_status(QueueStatus::Cancelled)
        .unwrap();
    let queue = queue.enqueue(cancelled);

    let active: Vec<_> = queue
        .entries()
        .iter()
        .filter(|e| !e.status.is_terminal())
        .collect();
    let terminal: Vec<_> = queue
        .entries()
        .iter()
        .filter(|e| e.status.is_terminal())
        .collect();

    assert_eq!(active.len(), 2);
    assert_eq!(terminal.len(), 2);
}

// ── Entry immutability through status transitions ────────────────────────

#[test]
fn test_sm_entry_priority_preserved_through_transitions() {
    let entry = QueueEntry::new("prio-test", "session", 42).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    assert_eq!(entry.priority, 42);

    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert_eq!(entry.priority, 42);
}

#[test]
fn test_sm_entry_id_preserved_through_transitions() {
    let entry = QueueEntry::new("id-test", "session", 10).unwrap();
    let original_id = entry.id.clone();

    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    assert_eq!(entry.id, original_id);

    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    assert_eq!(entry.id, original_id);

    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    assert_eq!(entry.id, original_id);

    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    assert_eq!(entry.id, original_id);

    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    assert_eq!(entry.id, original_id);

    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert_eq!(entry.id, original_id);
}

#[test]
fn test_sm_entry_session_preserved_through_transitions() {
    let entry = QueueEntry::new("sess-test", "my-session", 10).unwrap();
    let original_session = entry.session.clone();

    let entry = entry.transition_status(QueueStatus::Claimed).unwrap();
    assert_eq!(entry.session, original_session);

    let entry = entry.transition_status(QueueStatus::Rebasing).unwrap();
    assert_eq!(entry.session, original_session);

    let entry = entry.transition_status(QueueStatus::Testing).unwrap();
    assert_eq!(entry.session, original_session);

    let entry = entry.transition_status(QueueStatus::ReadyToMerge).unwrap();
    assert_eq!(entry.session, original_session);

    let entry = entry.transition_status(QueueStatus::Merging).unwrap();
    assert_eq!(entry.session, original_session);

    let entry = entry.transition_status(QueueStatus::Merged).unwrap();
    assert_eq!(entry.session, original_session);
}
