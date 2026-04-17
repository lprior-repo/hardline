//! Tests for the queue repository ports.
//! These tests verify the in-memory repository implementation.

use crate::domain::identifiers::QueueEntryId;
use crate::domain::ports::{InMemoryQueueRepository, QueueRepository};
use crate::domain::queue::entry::QueueEntry;
use crate::domain::queue::status::QueueStatus;

/// Helper to create a test entry
fn create_test_entry(session: &str) -> QueueEntry {
    QueueEntry::new(format!("queue-{}", session), session, 50).unwrap()
}

#[test]
fn in_memory_repo_enqueue_and_dequeue() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    let enqueued_result = repo.enqueue(entry);
    assert!(enqueued_result.is_ok(), "Enqueue should succeed");

    let dequeued_result = repo.dequeue();
    assert!(dequeued_result.is_ok(), "Dequeue should succeed");
    let dequeued = dequeued_result.unwrap();
    assert!(dequeued.is_some(), "Dequeued entry should exist");
}

#[test]
fn in_memory_repo_get_returns_entry() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    let enqueued = repo.enqueue(entry).unwrap();
    let get_result = repo.get(&enqueued.id);

    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_some(), "Entry should be found");
}

#[test]
fn in_memory_repo_remove_deletes_entry() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    let enqueued = repo.enqueue(entry).unwrap();
    let remove_result = repo.remove(&enqueued.id);
    assert!(remove_result.is_ok(), "Remove should succeed");

    let found = repo.get(&enqueued.id).unwrap();
    assert!(found.is_none(), "Entry should be removed");
}

#[test]
fn in_memory_repo_update_replaces_entry() {
    let repo = InMemoryQueueRepository::new();
    let entry1 = create_test_entry("session-1");

    let enqueued = repo.enqueue(entry1).unwrap();
    let update_result = repo.update(enqueued);
    assert!(update_result.is_ok(), "Update should succeed");
}

#[test]
fn in_memory_repo_list_pending_returns_all_pending_entries() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();
    repo.enqueue(create_test_entry("session-2")).unwrap();

    let pending = repo.list_pending().unwrap();
    assert_eq!(pending.len(), 2, "Should have 2 pending entries");
}

#[test]
fn in_memory_repo_dequeue_empty_queue_returns_none() {
    let repo = InMemoryQueueRepository::new();
    let dequeued = repo.dequeue().unwrap();
    assert!(dequeued.is_none(), "Empty queue should return None");
}

#[test]
fn in_memory_repo_get_nonexistent_returns_none() {
    let repo = InMemoryQueueRepository::new();
    let fake_id = QueueEntryId::new("nonexistent-id").unwrap();

    let found = repo.get(&fake_id).unwrap();
    assert!(found.is_none(), "Nonexistent entry should return None");
}

#[test]
fn in_memory_repo_clone_creates_independent_copy() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();
    let cloned_repo = repo.clone();

    assert_eq!(repo.list_all().unwrap().len(), 1);
    assert_eq!(cloned_repo.list_all().unwrap().len(), 1);
}

#[test]
fn in_memory_repo_dequeue_removes_entry_from_queue() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();

    let dequeued = repo.dequeue().unwrap();
    assert!(dequeued.is_some());

    let pending = repo.list_pending().unwrap();
    assert!(pending.is_empty());

    let dequeued_again = repo.dequeue().unwrap();
    assert!(dequeued_again.is_none());
}

#[test]
fn in_memory_repo_list_all_returns_all_entries() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("s1")).unwrap();
    repo.enqueue(create_test_entry("s2")).unwrap();
    repo.enqueue(create_test_entry("s3")).unwrap();

    let all = repo.list_all().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn in_memory_repo_list_all_empty() {
    let repo = InMemoryQueueRepository::new();
    let all = repo.list_all().unwrap();
    assert!(all.is_empty());
}

#[test]
fn in_memory_repo_dequeue_skips_non_pending() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    let enqueued = repo.enqueue(entry).unwrap();

    // Update to Claimed status
    let claimed = enqueued.transition_status(QueueStatus::Claimed).unwrap();
    repo.update(claimed).unwrap();

    // Dequeue should skip non-pending
    let dequeued = repo.dequeue().unwrap();
    assert!(dequeued.is_none());
}

#[test]
fn in_memory_repo_remove_nonexistent_returns_error() {
    let repo = InMemoryQueueRepository::new();
    let fake_id = QueueEntryId::new("nonexistent").unwrap();
    let result = repo.remove(&fake_id);
    assert!(result.is_err());
}

#[test]
fn in_memory_repo_update_nonexistent_returns_error() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    let result = repo.update(entry);
    assert!(result.is_err());
}

#[test]
fn in_memory_repo_enqueue_returns_same_entry() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    let original_id = entry.id.as_str().to_string();

    let enqueued = repo.enqueue(entry).unwrap();
    assert_eq!(enqueued.id.as_str(), original_id);
}

#[test]
fn in_memory_repo_fifo_order() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("s1")).unwrap();
    repo.enqueue(create_test_entry("s2")).unwrap();
    repo.enqueue(create_test_entry("s3")).unwrap();

    let d1 = repo.dequeue().unwrap();
    assert_eq!(d1.unwrap().session.as_str(), "s1");

    let d2 = repo.dequeue().unwrap();
    assert_eq!(d2.unwrap().session.as_str(), "s2");

    let d3 = repo.dequeue().unwrap();
    assert_eq!(d3.unwrap().session.as_str(), "s3");
}

#[test]
fn in_memory_repo_get_after_dequeue_returns_none() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    let enqueued = repo.enqueue(entry).unwrap();
    let id = enqueued.id.clone();

    repo.dequeue().unwrap();

    let result = repo.get(&id).unwrap();
    assert!(result.is_none());
}

#[test]
fn in_memory_repo_default_creates_empty_repo() {
    let repo = InMemoryQueueRepository::default();
    let all = repo.list_all().unwrap();
    assert!(all.is_empty());
}

#[test]
fn in_memory_repo_clone_independence() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();
    let cloned = repo.clone();

    cloned.enqueue(create_test_entry("session-2")).unwrap();

    let original_all = repo.list_all().unwrap();
    assert_eq!(original_all.len(), 1);
    assert_eq!(original_all[0].session.as_str(), "session-1");

    let cloned_all = cloned.list_all().unwrap();
    assert_eq!(cloned_all.len(), 2);
}

#[test]
fn in_memory_repo_multiple_enqueue_dequeue_cycles() {
    let repo = InMemoryQueueRepository::new();

    repo.enqueue(create_test_entry("s1")).unwrap();
    let d1 = repo.dequeue().unwrap();
    assert_eq!(d1.unwrap().session.as_str(), "s1");

    repo.enqueue(create_test_entry("s2")).unwrap();
    let d2 = repo.dequeue().unwrap();
    assert_eq!(d2.unwrap().session.as_str(), "s2");

    assert!(repo.dequeue().unwrap().is_none());
}

#[test]
fn in_memory_repo_with_entries_initializes_correctly() {
    let entry = create_test_entry("preloaded");
    let entries = std::collections::VecDeque::from(vec![entry]);
    let repo = InMemoryQueueRepository::with_entries(entries);

    let all = repo.list_all().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].session.as_str(), "preloaded");
}

#[test]
fn in_memory_repo_get_returns_correct_entry_by_id() {
    let repo = InMemoryQueueRepository::new();
    let e1 = repo.enqueue(create_test_entry("s1")).unwrap();
    let e2 = repo.enqueue(create_test_entry("s2")).unwrap();

    let found1 = repo.get(&e1.id).unwrap();
    assert!(found1.is_some());
    assert_eq!(found1.unwrap().session.as_str(), "s1");

    let found2 = repo.get(&e2.id).unwrap();
    assert!(found2.is_some());
    assert_eq!(found2.unwrap().session.as_str(), "s2");
}

#[test]
fn in_memory_repo_update_preserves_id() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("s1");
    let enqueued = repo.enqueue(entry).unwrap();
    let id = enqueued.id.clone();

    let updated = repo.update(enqueued).unwrap();
    assert_eq!(updated.id, id);

    let found = repo.get(&id).unwrap().unwrap();
    assert_eq!(found.id, id);
}

#[test]
fn in_memory_repo_remove_then_get_returns_none() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("s1");
    let enqueued = repo.enqueue(entry).unwrap();
    let id = enqueued.id.clone();

    assert!(repo.remove(&id).is_ok());

    let found = repo.get(&id).unwrap();
    assert!(found.is_none());

    let all = repo.list_all().unwrap();
    assert!(all.is_empty());
}

// --- Proptests for repository invariants ---

use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

proptest! {
    #[test]
    fn proptest_repo_size_matches_enqueue_count(
        sessions in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..20)
    ) {
        let repo = InMemoryQueueRepository::new();
        let count = sessions.len();
        for session in &sessions {
            let entry = create_test_entry(session);
            repo.enqueue(entry).unwrap();
        }
        let all = repo.list_all().unwrap();
        prop_assert_eq!(all.len(), count);
    }

    #[test]
    fn proptest_repo_dequeue_count_matches_enqueue(
        sessions in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..20)
    ) {
        let repo = InMemoryQueueRepository::new();
        for session in &sessions {
            let entry = create_test_entry(session);
            repo.enqueue(entry).unwrap();
        }
        let mut dequeued_count = 0;
        while repo.dequeue().unwrap().is_some() {
            dequeued_count += 1;
        }
        prop_assert_eq!(dequeued_count, sessions.len());
    }

    #[test]
    fn proptest_repo_get_after_enqueue_always_succeeds(
        session in "[a-zA-Z0-9_-]{1,20}"
    ) {
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry(&session);
        let enqueued = repo.enqueue(entry).unwrap();
        let found = repo.get(&enqueued.id).unwrap();
        prop_assert!(found.is_some());
        let found_entry = found.unwrap();
        prop_assert_eq!(found_entry.session.as_str(), session);
    }

    #[test]
    fn proptest_repo_remove_decrements_count(
        sessions in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 2..10)
    ) {
        let repo = InMemoryQueueRepository::new();
        let mut ids = Vec::new();
        for session in &sessions {
            let entry = create_test_entry(session);
            let enqueued = repo.enqueue(entry).unwrap();
            ids.push(enqueued.id.clone());
        }
        repo.remove(&ids[0]).unwrap();
        let all = repo.list_all().unwrap();
        prop_assert_eq!(all.len(), sessions.len() - 1);
    }
}
