//! Tests for the queue repository ports.
//! These tests verify the in-memory repository implementation.

use crate::domain::ports::InMemoryQueueRepository;
use crate::domain::queue::{QueueEntry, QueueStatus};
use crate::domain::identifiers::QueueEntryId;
use chrono::Utc;

/// Helper to create a test entry
fn create_test_entry(session: &str) -> QueueEntry {
    QueueEntry::new(session, session, 1).unwrap()
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
    
    let get_result = repo.get(&enqueued.id);
    
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
    
    let remove_result = repo.remove(&enqueued.id);
    assert!(remove_result.is_ok(), "Remove should succeed");
    
    // Assert
    let get_result = repo.get(&enqueued.id);
    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_none(), "Entry should be removed");
}

#[test]
fn in_memory_repo_update_modifies_entry() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    let entry = create_test_entry("session-1");
    
    // Act
    let enqueued_result = repo.enqueue(entry);
    assert!(enqueued_result.is_ok(), "Enqueue should succeed");
    let enqueued = enqueued_result.unwrap();
    
    // Update status
    let updated = QueueEntry::with_status(
        enqueued.id.clone(),
        enqueued.session.clone(),
        enqueued.priority,
        enqueued.enqueued_at,
        QueueStatus::Claimed,
    ).unwrap();
    
    let update_result = repo.update(updated.clone());
    
    // Assert
    assert!(update_result.is_ok(), "Update should succeed");
    let updated_result = update_result.unwrap();
    assert_eq!(updated_result.status, QueueStatus::Claimed, "Status should be updated");
    
    // Verify persistence
    let get_result = repo.get(&enqueued.id);
    assert!(get_result.is_ok(), "Get should succeed");
    let found = get_result.unwrap();
    assert!(found.is_some(), "Entry should exist");
    assert_eq!(found.unwrap().status, QueueStatus::Claimed, "Status should persist");
}

#[test]
fn in_memory_repo_list_pending_filters_correctly() {
    // Arrange
    let repo = InMemoryQueueRepository::new();
    
    // Act - add multiple entries
    let entry1 = create_test_entry("session-1");
    let entry2 = create_test_entry("session-2");
    
    let _enqueued1 = repo.enqueue(entry1).unwrap();
    let enqueued2 = repo.enqueue(entry2).unwrap();
    
    // Mark one as claimed
    let updated = QueueEntry::with_status(
        enqueued2.id.clone(),
        enqueued2.session.clone(),
        enqueued2.priority,
        enqueued2.enqueued_at,
        QueueStatus::Claimed,
    ).unwrap();
    repo.update(updated).unwrap();
    
    // Assert
    let pending_result = repo.list_pending();
    assert!(pending_result.is_ok(), "List pending should succeed");
    let pending = pending_result.unwrap();
    assert_eq!(pending.len(), 1, "Should have 1 pending entry");
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
    let fake_id = QueueEntryId::new("nonexistent-id").unwrap();
    
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
    assert_eq!(list_result.unwrap().len(), 1, "Original should have 1 entry");
    
    let cloned_list_result = cloned_repo.list_all();
    assert!(cloned_list_result.is_ok(), "Cloned list should succeed");
    assert_eq!(cloned_list_result.unwrap().len(), 1, "Cloned should have 1 entry");
}
