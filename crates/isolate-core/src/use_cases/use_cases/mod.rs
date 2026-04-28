//! Use cases layer - Pure orchestration for queue operations
//!
//! This module contains use cases following Domain-Driven Design principles:
//! - Pure functions that orchestrate domain logic
//! - No I/O operations (delegated to infrastructure layer)
//! - Railway-Oriented Programming for error chaining
//! - Each use case takes domain types and returns Results

use crate::queue::{Queue, QueueEntry, QueueEntryId, QueueStatus, SessionName};
use crate::Error;

/// Domain errors for queue operations
#[derive(Debug)]
pub enum DomainError {
    /// Entry not found
    NotFound(String),
    /// Entry already exists
    AlreadyExists(String),
    /// Invalid state transition
    InvalidStateTransition { from: String, to: String },
    /// Invalid priority
    InvalidPriority(u32),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            Self::InvalidPriority(priority) => write!(f, "Invalid priority: {}", priority),
        }
    }
}

impl std::error::Error for DomainError {}

/// View of a queue entry for display
#[derive(Debug, Clone)]
pub struct QueueEntryView {
    pub id: String,
    pub session: String,
    pub priority: u32,
    pub status: String,
    pub enqueued_at: String,
}

impl From<&QueueEntry> for QueueEntryView {
    fn from(entry: &QueueEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            session: entry.session.to_string(),
            priority: entry.priority,
            status: format!("{:?}", entry.status),
            enqueued_at: entry.enqueued_at.to_rfc3339(),
        }
    }
}

/// Enqueue a session into the queue
///
/// # Errors
/// Returns `DomainError::AlreadyExists` if session is already in queue.
/// Returns `DomainError::InvalidPriority` if priority is invalid.
pub fn enqueue_session(
    queue: &Queue,
    session: String,
    priority: u32,
) -> Result<Queue, DomainError> {
    // Validate session name
    let session_name =
        SessionName::new(&session).map_err(|_| DomainError::InvalidStateTransition {
            from: "input".to_string(),
            to: "session".to_string(),
        })?;

    // Check if session already exists
    if queue.find_by_session(&session_name).is_some() {
        return Err(DomainError::AlreadyExists(session));
    }

    // Create entry
    let id = QueueEntryId::new(format!("q-{}", chrono::Utc::now().timestamp_millis())).map_err(
        |_| DomainError::InvalidStateTransition {
            from: "timestamp".to_string(),
            to: "id".to_string(),
        },
    )?;

    let entry = QueueEntry::new(id, session_name, priority)
        .map_err(|_| DomainError::InvalidPriority(priority))?;

    // Add to queue
    let mut new_queue = queue.clone();
    new_queue.enqueue(entry);
    Ok(new_queue)
}

/// Dequeue a session from the queue
///
/// # Errors
/// Returns `DomainError::NotFound` if session is not in queue.
pub fn dequeue_session(queue: &Queue, session: &str) -> Result<Queue, DomainError> {
    let session_name =
        SessionName::new(session).map_err(|_| DomainError::NotFound(session.to_string()))?;

    let entry = queue
        .find_by_session(&session_name)
        .ok_or_else(|| DomainError::NotFound(session.to_string()))?;

    let mut new_queue = queue.clone();
    new_queue.dequeue(&entry.id);
    Ok(new_queue)
}

/// List all queue entries
#[must_use]
pub fn list_queue(queue: &Queue) -> Vec<QueueEntryView> {
    queue.entries().iter().map(QueueEntryView::from).collect()
}

/// Remove entry at a specific position
///
/// # Errors
/// Returns `DomainError::NotFound` if position is invalid.
pub fn remove_at_position(queue: &Queue, position: usize) -> Result<Queue, DomainError> {
    if position >= queue.len() {
        return Err(DomainError::NotFound(format!("Position {}", position)));
    }

    // Get the entry ID first to avoid borrow issues
    let entry_id = queue.entries().get(position).map(|e| e.id.clone());

    let mut new_queue = queue.clone();
    if let Some(id) = entry_id {
        new_queue.dequeue(&id);
    }
    Ok(new_queue)
}

/// Insert entry at a specific position
///
/// # Errors
/// Returns `DomainError::AlreadyExists` if session already in queue.
/// Returns `DomainError::InvalidPriority` if priority is invalid.
pub fn insert_at_position(
    queue: &Queue,
    session: String,
    priority: u32,
    position: usize,
) -> Result<Queue, DomainError> {
    // Validate session name
    let session_name =
        SessionName::new(&session).map_err(|_| DomainError::InvalidStateTransition {
            from: "input".to_string(),
            to: "session".to_string(),
        })?;

    // Check if session already exists
    if queue.find_by_session(&session_name).is_some() {
        return Err(DomainError::AlreadyExists(session));
    }

    // Create entry
    let id = QueueEntryId::new(format!("q-{}", chrono::Utc::now().timestamp_millis())).map_err(
        |_| DomainError::InvalidStateTransition {
            from: "timestamp".to_string(),
            to: "id".to_string(),
        },
    )?;

    let entry = QueueEntry::new(id, session_name, priority)
        .map_err(|_| DomainError::InvalidPriority(priority))?;

    // Insert at position
    let mut new_queue = queue.clone();
    new_queue
        .insert(position, entry)
        .map_err(|_| DomainError::NotFound(format!("Position {}", position)))?;
    Ok(new_queue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_session() {
        let queue = Queue::new();
        let result = enqueue_session(&queue, "test-session".to_string(), 10);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn test_enqueue_duplicate_fails() {
        let queue = Queue::new();
        let queue = enqueue_session(&queue, "test-session".to_string(), 10).unwrap();
        let result = enqueue_session(&queue, "test-session".to_string(), 20);
        assert!(result.is_err());
    }

    #[test]
    fn test_dequeue_session() {
        let queue = Queue::new();
        let queue = enqueue_session(&queue, "test-session".to_string(), 10).unwrap();
        let result = dequeue_session(&queue, "test-session");
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert!(new_queue.is_empty());
    }

    #[test]
    fn test_list_queue() {
        let queue = Queue::new();
        let queue = enqueue_session(&queue, "session1".to_string(), 10).unwrap();
        let queue = enqueue_session(&queue, "session2".to_string(), 20).unwrap();
        let entries = list_queue(&queue);
        assert_eq!(entries.len(), 2);
    }
}
