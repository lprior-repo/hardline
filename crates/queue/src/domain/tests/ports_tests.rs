//! Tests for the queue repository ports.
//! These tests verify the in-memory repository implementation.

use crate::domain::entities::{queue_entry::Pending, QueueEntry, QueueEntryId};
use crate::domain::ports::{InMemoryQueueRepository, QueueRepository};
use crate::domain::value_objects::Priority;

/// Helper to create a test entry
fn create_test_entry(session: &str) -> QueueEntry {
    QueueEntry::<Pending>::enqueue(session.to_string(), None, Priority::default()).unwrap()
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

    // Act - enqueue an entry, then update with a different entry
    let _enqueued = repo.enqueue(entry1).unwrap();
    let replacement = create_test_entry("session-2");
    let update_result = repo.update(replacement);

    // Assert
    assert!(update_result.is_ok(), "Update should succeed");
    let updated = update_result.unwrap();
    assert_eq!(
        updated.session_id(),
        "session-2",
        "Session should be updated"
    );
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
