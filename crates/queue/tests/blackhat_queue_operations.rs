//! Black-hat test suite: Queue operations — priority ordering, immutability, state machine
//!
//! Tests the immutable Queue and QueueEntry from domain::queue.
//! Uses scp_queue public API types.

use std::sync::{Arc, Mutex};
use std::thread;

use scp_queue::domain::queue::status::MAX_PRIORITY;
use scp_queue::{Queue, QueueEntry, QueueEntryId, QueueStatus};

// ── Helpers ──

fn make_entry(id: &str, session: &str, priority: u32) -> QueueEntry {
    QueueEntry::new(id, session, priority).unwrap()
}

fn entry_id(id: &str) -> QueueEntryId {
    QueueEntryId::new(id).unwrap()
}

// ── Priority Ordering ──

#[test]
fn enqueue_maintains_priority_order_reversed() {
    let queue = Queue::new()
        .enqueue(make_entry("high", "s1", 100))
        .enqueue(make_entry("low", "s2", 1))
        .enqueue(make_entry("mid", "s3", 50));

    let priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
    assert_eq!(priorities, vec![1, 50, 100]);
}

#[test]
fn enqueue_maintains_priority_order_already_sorted() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 1))
        .enqueue(make_entry("b", "s2", 50))
        .enqueue(make_entry("c", "s3", 100));

    let priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
    assert_eq!(priorities, vec![1, 50, 100]);
}

#[test]
fn enqueue_same_priority_maintains_fifo() {
    let queue = Queue::new()
        .enqueue(make_entry("first", "s1", 10))
        .enqueue(make_entry("second", "s2", 10))
        .enqueue(make_entry("third", "s3", 10));

    let ids: Vec<&str> = queue.entries().iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["first", "second", "third"]);
}

#[test]
fn enqueue_mixed_priority_and_fifo() {
    let queue = Queue::new()
        .enqueue(make_entry("p10-a", "s1", 10))
        .enqueue(make_entry("p5-a", "s2", 5))
        .enqueue(make_entry("p10-b", "s3", 10))
        .enqueue(make_entry("p5-b", "s4", 5));

    let priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
    assert_eq!(priorities, vec![5, 5, 10, 10]);
    assert_eq!(queue.entries()[0].id.as_str(), "p5-a");
    assert_eq!(queue.entries()[1].id.as_str(), "p5-b");
}

#[test]
fn enqueue_priority_zero_comes_first() {
    let queue = Queue::new()
        .enqueue(make_entry("normal", "s1", 50))
        .enqueue(make_entry("urgent", "s2", 0));

    assert_eq!(queue.entries()[0].id.as_str(), "urgent");
}

#[test]
fn enqueue_max_priority_comes_last() {
    let queue = Queue::new()
        .enqueue(make_entry("normal", "s1", 50))
        .enqueue(make_entry("max", "s2", MAX_PRIORITY));

    assert_eq!(queue.entries()[1].id.as_str(), "max");
}

// ── Immutability ──

#[test]
fn enqueue_does_not_modify_original() {
    let original = Queue::new().enqueue(make_entry("a", "s1", 10));
    let _extended = original.enqueue(make_entry("b", "s2", 20));

    assert_eq!(original.len(), 1);
}

#[test]
fn dequeue_does_not_modify_original() {
    let original = Queue::new().enqueue(make_entry("a", "s1", 10));
    let (new_queue, removed) = original.dequeue(&entry_id("a"));

    assert!(removed.is_some());
    assert_eq!(original.len(), 1);
    assert!(new_queue.is_empty());
}

#[test]
fn update_status_does_not_modify_original() {
    let original = Queue::new().enqueue(make_entry("a", "s1", 10));
    let updated = original
        .update_status(&entry_id("a"), QueueStatus::Claimed)
        .unwrap();

    assert!(matches!(
        original.find(&entry_id("a")).unwrap().status,
        QueueStatus::Pending
    ));
    assert!(matches!(
        updated.find(&entry_id("a")).unwrap().status,
        QueueStatus::Claimed
    ));
}

#[test]
fn with_entry_does_not_modify_original() {
    let original = Queue::new().enqueue(make_entry("a", "s1", 10));
    let extended = original.with_entry(1, make_entry("b", "s2", 20)).unwrap();

    assert_eq!(original.len(), 1);
    assert_eq!(extended.len(), 2);
}

// ── Dequeue Operations ──

#[test]
fn dequeue_existing_entry() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    let (new_queue, removed) = queue.dequeue(&entry_id("a"));
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id.as_str(), "a");
    assert_eq!(new_queue.len(), 1);
    assert!(new_queue.find(&entry_id("a")).is_none());
}

#[test]
fn dequeue_nonexistent_entry() {
    let queue = Queue::new().enqueue(make_entry("a", "s1", 10));
    let (new_queue, removed) = queue.dequeue(&entry_id("nonexistent"));

    assert!(removed.is_none());
    assert_eq!(new_queue.len(), 1);
}

#[test]
fn dequeue_from_empty_queue() {
    let queue = Queue::new();
    let (new_queue, removed) = queue.dequeue(&entry_id("anything"));

    assert!(removed.is_none());
    assert!(new_queue.is_empty());
}

#[test]
fn dequeue_all_entries() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    let (queue, _) = queue.dequeue(&entry_id("a"));
    let (queue, _) = queue.dequeue(&entry_id("b"));
    assert!(queue.is_empty());
}

#[test]
fn dequeue_middle_entry_preserves_others() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20))
        .enqueue(make_entry("c", "s3", 30));

    let (new_queue, removed) = queue.dequeue(&entry_id("b"));
    assert!(removed.is_some());
    assert_eq!(new_queue.len(), 2);
    assert!(new_queue.find(&entry_id("a")).is_some());
    assert!(new_queue.find(&entry_id("c")).is_some());
}

// ── State Machine Transitions ──

#[test]
fn full_lifecycle_happy_path() {
    let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
    let id = entry_id("e1");

    let q = queue.update_status(&id, QueueStatus::Claimed).unwrap();
    let q = q.update_status(&id, QueueStatus::Rebasing).unwrap();
    let q = q.update_status(&id, QueueStatus::Testing).unwrap();
    let q = q.update_status(&id, QueueStatus::ReadyToMerge).unwrap();
    let q = q.update_status(&id, QueueStatus::Merging).unwrap();
    let q = q.update_status(&id, QueueStatus::Merged).unwrap();

    assert!(q.find(&id).unwrap().status.is_terminal());
}

#[test]
fn lifecycle_with_retry() {
    let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
    let id = entry_id("e1");

    let q = queue.update_status(&id, QueueStatus::Claimed).unwrap();
    let q = q.update_status(&id, QueueStatus::Rebasing).unwrap();
    let q = q.update_status(&id, QueueStatus::Testing).unwrap();
    let q = q.update_status(&id, QueueStatus::FailedRetryable).unwrap();

    let q = q.update_status(&id, QueueStatus::Pending).unwrap();
    assert!(matches!(q.find(&id).unwrap().status, QueueStatus::Pending));
}

#[test]
fn invalid_transition_pending_to_merged() {
    let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
    let result = queue.update_status(&entry_id("e1"), QueueStatus::Merged);
    assert!(result.is_err());
}

#[test]
fn invalid_transition_merged_to_pending() {
    let result = QueueStatus::Merged.transition_to(QueueStatus::Pending);
    assert!(result.is_err());
}

#[test]
fn invalid_transition_cancelled_to_anything() {
    let result = QueueStatus::Cancelled.transition_to(QueueStatus::Pending);
    assert!(result.is_err());
}

#[test]
fn invalid_transition_failed_terminal_to_pending() {
    let result = QueueStatus::FailedTerminal.transition_to(QueueStatus::Pending);
    assert!(result.is_err());
}

#[test]
fn cancel_from_each_non_terminal_state() {
    // Test cancel from every non-terminal state via entry transitions
    let non_terminal_chain = vec![
        (QueueStatus::Pending, QueueStatus::Cancelled),
        (QueueStatus::Claimed, QueueStatus::Cancelled),
        (QueueStatus::Rebasing, QueueStatus::Cancelled),
        (QueueStatus::Testing, QueueStatus::Cancelled),
        (QueueStatus::ReadyToMerge, QueueStatus::Cancelled),
        (QueueStatus::Merging, QueueStatus::Cancelled),
        (QueueStatus::FailedRetryable, QueueStatus::Cancelled),
    ];

    for (from, to) in non_terminal_chain {
        let result = from.transition_to(to);
        assert!(result.is_ok(), "Cancel from {:?} should succeed", from);
    }
}

#[test]
fn entry_full_lifecycle_to_failed_terminal() {
    let entry = QueueEntry::new("e1", "s1", 10).unwrap();
    let result = entry
        .transition_status(QueueStatus::Claimed)
        .and_then(|e| e.transition_status(QueueStatus::Rebasing))
        .and_then(|e| e.transition_status(QueueStatus::Testing))
        .and_then(|e| e.transition_status(QueueStatus::FailedTerminal));
    assert!(result.is_ok());
    assert!(result.unwrap().status.is_terminal());
}

// ── Functional Operations ──

#[test]
fn filter_by_status() {
    let queue = Queue::new().enqueue(make_entry("a", "s1", 10)).enqueue(
        make_entry("b", "s2", 20)
            .transition_status(QueueStatus::Claimed)
            .unwrap(),
    );

    let pending = queue.filter(|e| e.status == QueueStatus::Pending);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id.as_str(), "a");
}

#[test]
fn map_entry_ids() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    let ids: Vec<String> = queue.map(|e| e.id.as_str().to_string());
    assert_eq!(ids, vec!["a", "b"]);
}

#[test]
fn fold_total_priority() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20))
        .enqueue(make_entry("c", "s3", 30));

    let total = queue.fold(0, |acc, e| acc + e.priority);
    assert_eq!(total, 60);
}

#[test]
fn any_and_all_combinators() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    assert!(queue.any(|e| e.priority < 15));
    assert!(!queue.any(|e| e.priority > 100));
    assert!(queue.all(|e| e.priority <= 20));
    assert!(!queue.all(|e| e.priority < 15));
}

#[test]
fn partition_entries() {
    let queue = Queue::new()
        .enqueue(make_entry("low", "s1", 10))
        .enqueue(make_entry("high", "s2", 80));

    let (low, high) = queue.partition(|e| e.priority < 50);
    assert_eq!(low.len(), 1);
    assert_eq!(high.len(), 1);
}

#[test]
fn group_by_status() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(
            make_entry("b", "s2", 20)
                .transition_status(QueueStatus::Claimed)
                .unwrap(),
        )
        .enqueue(
            make_entry("c", "s3", 30)
                .transition_status(QueueStatus::Cancelled)
                .unwrap(),
        );

    let grouped = queue.group_by_status();
    assert_eq!(grouped.len(), 3);
}

#[test]
fn count_active_excludes_terminal() {
    let queue = Queue::new()
        .enqueue(make_entry("active", "s1", 10))
        .enqueue(
            make_entry("a", "s2", 20)
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
                .unwrap(),
        );

    assert_eq!(queue.count_active(), 1);
}

#[test]
fn sorted_by_custom_key() {
    let queue = Queue::new()
        .enqueue(make_entry("c", "s3", 30))
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    let sorted = queue.sorted_by_key(|e| e.priority);
    let priorities: Vec<u32> = sorted.iter().map(|e| e.priority).collect();
    assert_eq!(priorities, vec![10, 20, 30]);
}

// ── QueueEntry Validation ──

#[test]
fn entry_rejects_empty_id() {
    assert!(QueueEntry::new("", "session", 10).is_err());
}

#[test]
fn entry_rejects_whitespace_id() {
    assert!(QueueEntry::new("   ", "session", 10).is_err());
}

#[test]
fn entry_rejects_empty_session() {
    assert!(QueueEntry::new("id", "", 10).is_err());
}

#[test]
fn entry_rejects_session_with_metacharacters() {
    for c in ['$', '`', '|', '&', '<', '>', '\n', '\r', '\0'] {
        assert!(QueueEntry::new("id", &format!("ses{}sion", c), 10).is_err());
    }
}

#[test]
fn entry_rejects_priority_above_max() {
    assert!(QueueEntry::new("id", "session", MAX_PRIORITY + 1).is_err());
}

#[test]
fn entry_accepts_max_priority() {
    assert!(QueueEntry::new("id", "session", MAX_PRIORITY).is_ok());
}

#[test]
fn entry_accepts_zero_priority() {
    let e = QueueEntry::new("id", "session", 0).unwrap();
    assert_eq!(e.priority, 0);
}

// ── Concurrent Access Safety ──

#[test]
fn concurrent_enqueue_via_mutex() {
    let shared = Arc::new(Mutex::new(Queue::new()));
    let mut handles = vec![];

    for i in 0..100 {
        let q = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let entry =
                QueueEntry::new(format!("id-{i}"), format!("session-{i}"), (i % 100) as u32)
                    .unwrap();
            let mut guard = q.lock().unwrap();
            *guard = guard.clone().enqueue(entry);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(shared.lock().unwrap().len(), 100);
}

#[test]
fn concurrent_enqueue_dequeue_via_mutex() {
    let shared = Arc::new(Mutex::new(Queue::new()));

    // Pre-populate
    {
        let mut guard = shared.lock().unwrap();
        for i in 0..50 {
            *guard = guard.clone().enqueue(
                QueueEntry::new(format!("initial-{i}"), format!("session-{i}"), 10).unwrap(),
            );
        }
    }
    assert_eq!(shared.lock().unwrap().len(), 50);

    let mut handles = vec![];

    for i in 0..50 {
        let q = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let entry = QueueEntry::new(format!("enq-{i}"), format!("sess-{i}"), 20).unwrap();
            let mut guard = q.lock().unwrap();
            *guard = guard.clone().enqueue(entry);
        }));
    }

    for i in 0..25 {
        let q = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let id = QueueEntryId::new(format!("initial-{i}")).unwrap();
            let mut guard = q.lock().unwrap();
            let (new_q, _) = guard.clone().dequeue(&id);
            *guard = new_q;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(shared.lock().unwrap().len(), 75);
}

#[test]
fn concurrent_read_only_safe() {
    let queue = Arc::new(
        Queue::new()
            .enqueue(make_entry("a", "s1", 10))
            .enqueue(make_entry("b", "s2", 20))
            .enqueue(make_entry("c", "s3", 30)),
    );

    let mut handles = vec![];
    for _ in 0..100 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            assert_eq!(q.len(), 3);
            assert!(q.find(&entry_id("a")).is_some());
            assert!(q.find(&entry_id("nonexistent")).is_none());
        }));
    }

    for h in handles {
        assert!(h.join().is_ok());
    }
}

// ── Serde Roundtrip ──

#[test]
fn queue_serde_roundtrip() {
    let queue = Queue::new()
        .enqueue(make_entry("a", "s1", 10))
        .enqueue(make_entry("b", "s2", 20));

    let json = serde_json::to_string(&queue).unwrap();
    let back: Queue = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back.entries()[0].id.as_str(), "a");
    assert_eq!(back.entries()[1].id.as_str(), "b");
}

#[test]
fn queue_entry_serde_roundtrip() {
    let entry = make_entry("serde-id", "serde-session", 42);
    let json = serde_json::to_string(&entry).unwrap();
    let back: QueueEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id.as_str(), "serde-id");
    assert_eq!(back.session.as_str(), "serde-session");
    assert_eq!(back.priority, 42);
}

// ── Proptests ──

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_enqueue_maintains_sorted_order(
        priorities in proptest::collection::vec(0..=MAX_PRIORITY, 0..50)
    ) {
        let mut queue = Queue::new();
        for (i, &p) in priorities.iter().enumerate() {
            if let Ok(e) = QueueEntry::new(format!("id-{i}"), format!("s-{i}"), p) {
                queue = queue.enqueue(e);
            }
        }

        let q_priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
        for window in q_priorities.windows(2) {
            prop_assert!(window[0] <= window[1]);
        }
    }

    #[test]
    fn prop_dequeue_removes_entry(
        id in "[a-zA-Z0-9_-]{2,10}",
        session in "[a-zA-Z0-9_-]{2,10}",
        priority in 0..=MAX_PRIORITY,
    ) {
        if let Ok(e) = QueueEntry::new(id.clone(), session, priority) {
            let queue = Queue::new().enqueue(e);
            let qid = QueueEntryId::new(id).unwrap();
            let (new_q, removed) = queue.dequeue(&qid);
            prop_assert!(removed.is_some());
            prop_assert!(new_q.is_empty());
            prop_assert!(new_q.find(&qid).is_none());
        }
    }

    #[test]
    fn prop_enqueue_immutability(
        entries in proptest::collection::vec(
            ("[a-zA-Z0-9_-]{2,10}", "[a-zA-Z0-9_-]{2,10}", 0..=MAX_PRIORITY),
            1..10
        ),
    ) {
        let queue = Queue::new();
        let original_len = queue.len();

        let mut q = queue.clone();
        for (id, session, priority) in &entries {
            if let Ok(e) = QueueEntry::new(id.clone(), session.clone(), *priority) {
                q = q.enqueue(e);
            }
        }

        prop_assert_eq!(queue.len(), original_len);
    }

    #[test]
    fn prop_status_terminal_cannot_transition(
        status_idx in 0u8..3u8,
        target_idx in 0u8..10u8,
    ) {
        let terminals = [
            QueueStatus::Merged,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        let all = [
            QueueStatus::Pending, QueueStatus::Claimed, QueueStatus::Rebasing,
            QueueStatus::Testing, QueueStatus::ReadyToMerge, QueueStatus::Merging,
            QueueStatus::Merged, QueueStatus::FailedRetryable, QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        let from = terminals[status_idx as usize];
        let to = all[target_idx as usize];
        let result = from.transition_to(to);
        prop_assert!(result.is_err());
    }
}
