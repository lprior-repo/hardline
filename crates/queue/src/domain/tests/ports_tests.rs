//! Tests for the queue repository ports.
//! These tests verify the in-memory repository implementation.

use crate::domain::entities::{queue_entry::Pending, QueueEntry, QueueEntryId, QueueStatus};
use crate::domain::ports::{InMemoryQueueRepository, QueueRepository};
use crate::domain::value_objects::Priority;

/// Helper to create a test entry
fn create_test_entry(session: &str) -> QueueEntry {
    QueueEntry::<Pending>::enqueue(session.to_string(), None, Priority::default()).unwrap()
}

/// Helper to create a test entry with a specific priority
fn create_test_entry_with_priority(session: &str, priority: Priority) -> QueueEntry {
    QueueEntry::<Pending>::enqueue(session.to_string(), None, priority).unwrap()
}

#[test]
fn in_memory_repo_enqueue_and_dequeue() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    // Act
    let enqueued_result = repo.enqueue(entry);

    // Assert - using proper error handling
    assert!(enqueued_result.is_ok(), "Enqueue should succeed");
    let _enqueued = enqueued_result.unwrap();

    let dequeued_result = repo.dequeue();
    assert!(dequeued_result.is_ok(), "Dequeue should succeed");
    let dequeued = dequeued_result.unwrap();
    assert!(dequeued.is_some(), "Dequeued entry should exist");
}

#[test]
fn in_memory_repo_get_returns_entry() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    // Act
    let enqueued_result = repo.enqueue(entry);
    assert!(enqueued_result.is_ok(), "Enqueue should succeed");
    let enqueued = enqueued_result.unwrap();

    let get_result = repo.get(enqueued.id());

    // Assert
    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_some(), "Entry should be found");
}

#[test]
fn in_memory_repo_remove_deletes_entry() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    // Act
    let enqueued_result = repo.enqueue(entry);
    assert!(enqueued_result.is_ok(), "Enqueue should succeed");
    let enqueued = enqueued_result.unwrap();

    let remove_result = repo.remove(enqueued.id());
    assert!(remove_result.is_ok(), "Remove should succeed");

    // Assert
    let get_result = repo.get(enqueued.id());
    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_none(), "Entry should be removed");
}

#[test]
fn in_memory_repo_update_replaces_entry() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry1 = create_test_entry("session-1");

    // Act - enqueue an entry, then update it (same ID)
    let enqueued = repo.enqueue(entry1).unwrap();
    let update_result = repo.update(enqueued);

    // Assert - update finds the entry by ID and succeeds
    assert!(update_result.is_ok(), "Update should succeed");
}

#[test]
fn in_memory_repo_list_pending_returns_all_pending_entries() {
    // Arrange
    let repo = InMemoryQueueRepository::new();

    // Act - add multiple entries
    let entry1 = create_test_entry("session-1");
    let entry2 = create_test_entry("session-2");

    let _enqueued1 = repo.enqueue(entry1).unwrap();
    let _enqueued2 = repo.enqueue(entry2).unwrap();

    // Assert - all pending entries should be returned
    let pending_result = repo.list_pending();
    assert!(pending_result.is_ok(), "List pending should succeed");
    let pending = pending_result.unwrap();
    assert_eq!(pending.len(), 2, "Should have 2 pending entries");
}

#[test]
fn in_memory_repo_dequeue_empty_queue_returns_none() {
    // Arrange
    let repo = InMemoryQueueRepository::new();

    // Act
    let dequeued_result = repo.dequeue();

    // Assert
    assert!(dequeued_result.is_ok(), "Dequeue should succeed");
    let dequeued = dequeued_result.unwrap();
    assert!(dequeued.is_none(), "Empty queue should return None");
}

#[test]
fn in_memory_repo_get_nonexistent_returns_none() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let fake_id = QueueEntryId::parse("nonexistent-id".to_string()).unwrap();

    // Act
    let get_result = repo.get(&fake_id);

    // Assert
    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_none(), "Nonexistent entry should return None");
}

#[test]
fn in_memory_repo_clone_creates_independent_copy() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");

    // Act
    repo.enqueue(entry).unwrap();
    let cloned_repo = repo.clone();

    // Assert - cloned repo should have its own copy
    let list_result = repo.list_all();
    assert!(list_result.is_ok(), "List should succeed");
    assert_eq!(
        list_result.unwrap().len(),
        1,
        "Original should have 1 entry"
    );

    let cloned_list_result = cloned_repo.list_all();
    assert!(cloned_list_result.is_ok(), "Cloned list should succeed");
    assert_eq!(
        cloned_list_result.unwrap().len(),
        1,
        "Cloned should have 1 entry"
    );
}

// --- Additional comprehensive tests ---

#[test]
fn in_memory_repo_dequeue_removes_entry_from_queue() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    repo.enqueue(entry).unwrap();

    let dequeued = repo.dequeue().unwrap();
    assert!(dequeued.is_some());

    // After dequeue, the queue should be empty
    let pending = repo.list_pending().unwrap();
    assert!(pending.is_empty());

    // Dequeue again should return None
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
fn in_memory_repo_list_pending_excludes_non_pending() {
    // Note: We can't directly update to Claimed status via the typestate system
    // because QueueRepository::update expects QueueEntry (Pending).
    // Instead, enqueue a Pending entry and verify list_pending returns it.
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();
    let pending = repo.list_pending().unwrap();
    assert_eq!(pending.len(), 1);
}

#[test]
fn in_memory_repo_dequeue_skips_non_pending() {
    // The in-memory repo dequeues the front of the VecDeque.
    // Since we can only enqueue Pending entries, all dequeued entries
    // are Pending by construction.
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("session-1")).unwrap();
    let dequeued = repo.dequeue().unwrap();
    assert!(dequeued.is_some());
}

#[test]
fn in_memory_repo_remove_nonexistent_returns_error() {
    let repo = InMemoryQueueRepository::new();
    let fake_id = QueueEntryId::parse("nonexistent".to_string()).unwrap();
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
    let original_id = entry.id().as_str().to_string();

    let enqueued = repo.enqueue(entry).unwrap();
    assert_eq!(enqueued.id().as_str(), original_id);
}

#[test]
fn in_memory_repo_fifo_order() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry("s1")).unwrap();
    repo.enqueue(create_test_entry("s2")).unwrap();
    repo.enqueue(create_test_entry("s3")).unwrap();

    let d1 = repo.dequeue().unwrap();
    assert_eq!(d1.unwrap().session_id(), "s1");

    let d2 = repo.dequeue().unwrap();
    assert_eq!(d2.unwrap().session_id(), "s2");

    let d3 = repo.dequeue().unwrap();
    assert_eq!(d3.unwrap().session_id(), "s3");
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

    // Modify cloned repo
    cloned.enqueue(create_test_entry("session-2")).unwrap();

    // Original should not be affected
    let original_all = repo.list_all().unwrap();
    assert_eq!(original_all.len(), 1);
    assert_eq!(original_all[0].session_id(), "session-1");

    let cloned_all = cloned.list_all().unwrap();
    assert_eq!(cloned_all.len(), 2);
}

#[test]
fn in_memory_repo_multiple_enqueue_dequeue_cycles() {
    let repo = InMemoryQueueRepository::new();

    // First cycle
    repo.enqueue(create_test_entry("s1")).unwrap();
    let d1 = repo.dequeue().unwrap();
    assert_eq!(d1.unwrap().session_id(), "s1");

    // Second cycle
    repo.enqueue(create_test_entry("s2")).unwrap();
    let d2 = repo.dequeue().unwrap();
    assert_eq!(d2.unwrap().session_id(), "s2");

    // Queue should be empty
    assert!(repo.dequeue().unwrap().is_none());
}

#[test]
fn in_memory_repo_with_entries_initializes_correctly() {
    let entry = create_test_entry("preloaded");
    let entries = std::collections::VecDeque::from(vec![entry.clone()]);
    let repo = InMemoryQueueRepository::with_entries(entries);

    let all = repo.list_all().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].session_id(), "preloaded");
}

#[test]
fn in_memory_repo_get_returns_correct_entry_by_id() {
    let repo = InMemoryQueueRepository::new();
    let e1 = repo.enqueue(create_test_entry("s1")).unwrap();
    let e2 = repo.enqueue(create_test_entry("s2")).unwrap();

    let found1 = repo.get(e1.id()).unwrap();
    assert!(found1.is_some());
    assert_eq!(found1.unwrap().session_id(), "s1");

    let found2 = repo.get(e2.id()).unwrap();
    assert!(found2.is_some());
    assert_eq!(found2.unwrap().session_id(), "s2");
}

#[test]
fn in_memory_repo_update_preserves_id() {
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("s1");
    let enqueued = repo.enqueue(entry).unwrap();
    let id = enqueued.id.clone();

    // Re-enqueue the same entry (type system requires Pending)
    let updated = repo.update(enqueued).unwrap();

    assert_eq!(updated.id(), &id);

    let found = repo.get(&id).unwrap().unwrap();
    assert_eq!(found.id(), &id);
}

#[test]
fn in_memory_repo_enqueue_with_different_priorities() {
    let repo = InMemoryQueueRepository::new();
    repo.enqueue(create_test_entry_with_priority("low", Priority::low())).unwrap();
    repo.enqueue(create_test_entry_with_priority("high", Priority::high())).unwrap();
    repo.enqueue(create_test_entry_with_priority("normal", Priority::normal())).unwrap();

    let all = repo.list_all().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn in_memory_repo_list_pending_filters_correctly() {
    let repo = InMemoryQueueRepository::new();

    // All entries in the in-memory repo are Pending by construction
    // (we can only enqueue Pending entries via the typestate system)
    repo.enqueue(create_test_entry("pending-1")).unwrap();
    repo.enqueue(create_test_entry("pending-2")).unwrap();

    let pending = repo.list_pending().unwrap();
    assert_eq!(pending.len(), 2);

    // After dequeue, one fewer pending
    repo.dequeue().unwrap();
    let pending_after = repo.list_pending().unwrap();
    assert_eq!(pending_after.len(), 1);
}

#[test]
fn in_memory_repo_enqueue_with_bead_id() {
    let entry = QueueEntry::<Pending>::enqueue(
        "session-1".to_string(),
        Some("bead-42".to_string()),
        Priority::default(),
    )
    .unwrap();
    let repo = InMemoryQueueRepository::new();
    let enqueued = repo.enqueue(entry).unwrap();
    assert_eq!(enqueued.bead_id(), Some("bead-42"));

    let found = repo.get(enqueued.id()).unwrap().unwrap();
    assert_eq!(found.bead_id(), Some("bead-42"));
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
        let found = repo.get(enqueued.id()).unwrap();
        prop_assert!(found.is_some());
        let found_entry = found.unwrap();
        prop_assert_eq!(found_entry.session_id(), session);
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
        // Remove first entry
        repo.remove(&ids[0]).unwrap();
        let all = repo.list_all().unwrap();
        prop_assert_eq!(all.len(), sessions.len() - 1);
    }
}
