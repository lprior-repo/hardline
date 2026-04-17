//! Queue tests

#![allow(clippy::unwrap_used)]

use chrono::Utc;

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::queue::entry::QueueEntry;
use crate::domain::queue::queue::Queue;
use crate::domain::queue::status::QueueStatus;
use crate::domain::validation::ValidationError;

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
}

#[test]
fn test_dequeue_returns_none_for_nonexistent() {
    let queue = Queue::new();
    let entry = QueueEntry::new("test-1", "test-session", 10).unwrap();
    let queue = queue.enqueue(entry);

    let id = QueueEntryId::new("nonexistent").unwrap();
    let (new_queue, removed) = queue.dequeue(&id);

    assert!(removed.is_none());
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

    let session = SessionName::new("my-session").unwrap();
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
    let entry = QueueEntry::with_timestamp(
        QueueEntryId::new("test-1").unwrap(),
        SessionName::new("session-1").unwrap(),
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
}

#[test]
fn test_with_entry_at_invalid_position_returns_error() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let new_entry = QueueEntry::new("out-of-bounds", "out-of-bounds", 5).unwrap();
    let result = queue.with_entry(5, new_entry);

    assert!(matches!(result, Err(ValidationError::OutOfBounds { .. })));
}

#[test]
fn test_status_pending_to_claimed_transition() {
    let status = QueueStatus::Pending;
    let result = status.transition_to(QueueStatus::Claimed);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), QueueStatus::Claimed);
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

    assert!(queue.all(|e| e.priority <= crate::domain::queue::status::MAX_PRIORITY));
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
fn test_update_status_valid_chain() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());

    let id = QueueEntryId::new("test-1").unwrap();
    let result = queue.update_status(&id, QueueStatus::Claimed);

    assert!(result.is_ok());
    let new_queue = result.unwrap();
    assert_eq!(new_queue.find(&id).unwrap().status, QueueStatus::Claimed);
}

#[test]
fn test_update_status_invalid_entry_returns_error() {
    let queue = Queue::new();
    let id = QueueEntryId::new("nonexistent").unwrap();
    let result = queue.update_status(&id, QueueStatus::Claimed);

    assert!(matches!(result, Err(ValidationError::NotFound { .. })));
}

#[test]
fn test_update_status_invalid_transition_returns_error() {
    let queue = Queue::new();
    let queue = queue.enqueue(QueueEntry::new("test-1", "session-1", 10).unwrap());

    let id = QueueEntryId::new("test-1").unwrap();
    let result = queue.update_status(&id, QueueStatus::Merged);

    assert!(matches!(
        result,
        Err(ValidationError::InvalidStateTransition { .. })
    ));
}

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
fn test_remove_at_valid_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let result = queue.remove_at(0);
    assert!(result.is_ok());
    let (new_queue, removed) = result.unwrap();
    assert_eq!(removed.id.as_str(), "first");
    assert!(new_queue.is_empty());
}

#[test]
fn test_remove_at_invalid_position() {
    let queue = Queue::new();
    let entry = QueueEntry::new("first", "first", 10).unwrap();
    let queue = queue.enqueue(entry);

    let result = queue.remove_at(5);
    assert!(result.is_err());
}

#[test]
fn test_queue_entry_id_valid() {
    assert!(QueueEntryId::new("test-123").is_ok());
}

#[test]
fn test_queue_entry_id_empty() {
    assert!(matches!(
        QueueEntryId::new(""),
        Err(ValidationError::EmptyValue(_))
    ));
}

#[test]
fn test_session_name_valid() {
    assert!(SessionName::new("my-session").is_ok());
    assert!(SessionName::new("session_123").is_ok());
}

#[test]
fn test_session_name_empty() {
    assert!(matches!(
        SessionName::new(""),
        Err(ValidationError::EmptyValue(_))
    ));
}

#[test]
fn test_session_name_rejects_shell_metacharacters() {
    let invalid_chars = ["$", "`", "|", "&", "<", ">", "\n", "\r", "\x00"];
    for c in invalid_chars {
        let test_name = format!("session{}name", c);
        assert!(
            matches!(
                SessionName::new(&test_name),
                Err(ValidationError::InvalidCharacters { .. })
            ),
            "Should reject character: {:?}",
            c
        );
    }
}

#[test]
fn test_queue_entry_new_rejects_high_priority() {
    let result = QueueEntry::new("test-1", "session", 101);
    assert!(matches!(
        result,
        Err(ValidationError::ExceedsMaximum { .. })
    ));
}

#[test]
fn test_queue_entry_new_accepts_max_priority() {
    let result = QueueEntry::new("test-1", "session", 100);
    assert!(result.is_ok());
}

use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

proptest! {
    #[test]
    fn prop_priority_ordering_invariant(
        entries in proptest::collection::vec(
            (
                "[a-zA-Z0-9_-]{1,20}",
                "[a-zA-Z0-9_-]{1,20}",
                0..=crate::domain::queue::status::MAX_PRIORITY
            ),
            0..50
        )
    ) {
        let mut queue = Queue::new();
        for (id, session, priority) in entries {
            if let Ok(entry) = QueueEntry::new(id, session, priority) {
                queue = queue.enqueue(entry);
            }
        }

        let priorities: Vec<_> = queue.entries().iter().map(|e| e.priority).collect();
        for window in priorities.windows(2) {
            prop_assert!(window[0] <= window[1]);
        }
    }

    #[test]
    fn prop_fifo_within_priority(
        priorities in proptest::collection::vec(0..=5u32, 1..50)
    ) {
        let mut queue = Queue::new();
        for (idx, priority) in priorities.iter().enumerate() {
            let id = format!("id-{}", idx);
            let session = format!("sess-{}", idx);
            if let Ok(entry) = QueueEntry::new(id, session, *priority) {
                queue = queue.enqueue(entry);
            }
        }

        for current_priority in 0..=5 {
            let same_priority_entries: Vec<_> = queue.entries()
                .iter()
                .filter(|e| e.priority == current_priority)
                .collect();

            for window in same_priority_entries.windows(2) {
                let idx1 = window[0].id.as_str().strip_prefix("id-").unwrap().parse::<usize>().unwrap();
                let idx2 = window[1].id.as_str().strip_prefix("id-").unwrap().parse::<usize>().unwrap();
                prop_assert!(idx1 < idx2);
            }
        }
    }

    #[test]
    fn prop_non_empty_queue_after_push(
        id in "[a-zA-Z0-9_-]{1,20}",
        session in "[a-zA-Z0-9_-]{1,20}",
        priority in 0..=crate::domain::queue::status::MAX_PRIORITY
    ) {
        let queue = Queue::new();
        if let Ok(entry) = QueueEntry::new(id, session, priority) {
            let new_queue = queue.enqueue(entry);
            prop_assert!(!new_queue.is_empty());
            prop_assert_eq!(new_queue.len(), 1);
        }
    }

    #[test]
    fn prop_size_bounds(
        entries in proptest::collection::vec(
            (
                "[a-zA-Z0-9_-]{1,20}",
                "[a-zA-Z0-9_-]{1,20}",
                0..=crate::domain::queue::status::MAX_PRIORITY
            ),
            0..50
        )
    ) {
        let mut queue = Queue::new();
        let mut count = 0;
        for (id, session, priority) in entries {
            if let Ok(entry) = QueueEntry::new(id, session, priority) {
                queue = queue.enqueue(entry);
                count += 1;
            }
        }
        prop_assert_eq!(queue.len(), count);
        prop_assert_eq!(queue.entries().len(), count);
    }

    #[test]
    fn prop_dequeue_then_find_none(
        id in "[a-zA-Z0-9_-]{1,20}",
        session in "[a-zA-Z0-9_-]{1,20}",
        priority in 0..=crate::domain::queue::status::MAX_PRIORITY
    ) {
        let id_clone = id.clone();
        if let Ok(entry) = QueueEntry::new(id, session, priority) {
            let queue = Queue::new().enqueue(entry);
            let entry_id = QueueEntryId::new(id_clone).unwrap();
            let (new_queue, removed) = queue.dequeue(&entry_id);
            prop_assert!(removed.is_some());
            prop_assert!(new_queue.find(&entry_id).is_none());
        }
    }

    #[test]
    fn prop_enqueue_immutability(
        id in "[a-zA-Z0-9_-]{1,20}",
        session in "[a-zA-Z0-9_-]{1,20}",
        priority in 0..=crate::domain::queue::status::MAX_PRIORITY
    ) {
        let queue = Queue::new();
        if let Ok(entry) = QueueEntry::new(id, session, priority) {
            let _new_queue = queue.enqueue(entry);
            prop_assert!(queue.is_empty());
        }
    }

    #[test]
    fn prop_find_nonexistent(
        id in "[a-zA-Z0-9_-]{1,20}",
        session in "[a-zA-Z0-9_-]{1,20}",
        priority in 0..=crate::domain::queue::status::MAX_PRIORITY,
        lookup_id in "[a-zA-Z0-9_-]{1,20}"
    ) {
        if let Ok(entry) = QueueEntry::new(id.clone(), session, priority) {
            let queue = Queue::new().enqueue(entry);
            if id != lookup_id {
                let lookup = QueueEntryId::new(lookup_id).unwrap();
                prop_assert!(queue.find(&lookup).is_none());
            }
        }
    }

    #[test]
    fn prop_queue_entry_serde_roundtrip(
        id in "[a-zA-Z0-9_-]{1,20}",
        session in "[a-zA-Z0-9_-]{1,20}",
        priority in 0..=crate::domain::queue::status::MAX_PRIORITY
    ) {
        if let Ok(entry) = QueueEntry::new(id, session, priority) {
            let json = serde_json::to_string(&entry).unwrap();
            let back: QueueEntry = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back.id.as_str(), entry.id.as_str());
            prop_assert_eq!(back.session.as_str(), entry.session.as_str());
            prop_assert_eq!(back.priority, entry.priority);
        }
    }

    #[test]
    fn prop_status_transition_validity(
        from_idx in 0u8..10u8,
        to_idx in 0u8..10u8
    ) {
        let all_statuses = [
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
        let from = all_statuses[from_idx as usize];
        let to = all_statuses[to_idx as usize];
        let result = from.transition_to(to);
        if result.is_ok() {
            prop_assert_eq!(result.unwrap(), to);
        } else {
            prop_assert!(result.is_err());
        }
    }
}
