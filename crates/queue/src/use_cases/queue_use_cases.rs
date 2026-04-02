#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Queue use cases - Pure orchestration for queue operations
//!
//! This module implements Railway-Oriented Programming where each use case:
//! 1. Takes domain types as input
//! 2. Validates inputs using the validation layer
//! 3. Applies business logic
//! 4. Returns Result<T, DomainError>
//!
//! All functions are pure - no I/O, no side effects.

use chrono::{DateTime, Utc};

use crate::domain::{Queue, QueueEntry, QueueEntryId, SessionName, MAX_PRIORITY};

/// Domain-level errors for use cases
///
/// These errors represent business rule violations and domain constraints.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Session name validation failed
    #[error("Invalid session name: {0}")]
    InvalidSessionName(String),

    /// Priority validation failed
    #[error("Invalid priority {0}: must be between 0 and 100")]
    InvalidPriority(u32),

    /// Position is out of bounds
    #[error("Position {position} is out of bounds (queue length: {length})")]
    PositionOutOfBounds { position: usize, length: usize },

    /// Session not found in queue
    #[error("Session '{0}' not found in queue")]
    SessionNotFound(String),

    /// Queue is empty
    #[error("Queue is empty")]
    QueueEmpty,

    /// Session already exists in queue
    #[error("Session '{0}' already exists in queue")]
    SessionAlreadyExists(String),

    /// Invalid state transition
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}

/// Result type for use case operations
pub type UseCaseResult<T> = Result<T, DomainError>;

/// View model for a queue entry
///
/// This is a read-only projection of a queue entry for display purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntryView {
    /// Position in the queue (0-indexed)
    pub position: usize,
    /// Session name
    pub session: String,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Current status
    pub status: String,
    /// When enqueued
    pub enqueued_at: DateTime<Utc>,
}

impl QueueEntryView {
    /// Create a view from a queue entry at a specific position
    #[must_use]
    pub fn new(position: usize, entry: &QueueEntry) -> Self {
        Self {
            position,
            session: entry.session.as_str().to_string(),
            priority: entry.priority,
            status: entry.status.to_string(),
            enqueued_at: entry.enqueued_at,
        }
    }
}

/// Enqueue a session into the queue
///
/// This use case validates the session name and priority, checks for duplicates,
/// and returns a new queue with the session added at the appropriate position
/// based on priority ordering.
pub fn enqueue_session(queue: Queue, session_name: &str, priority: u32) -> UseCaseResult<Queue> {
    let session = SessionName::new(session_name).map_err(|_| {
        DomainError::InvalidSessionName(if session_name.trim().is_empty() {
            "cannot be empty".to_string()
        } else {
            format!("invalid characters in '{}'", session_name)
        })
    })?;

    if priority > MAX_PRIORITY {
        return Err(DomainError::InvalidPriority(priority));
    }

    if queue.find_by_session(&session).is_some() {
        return Err(DomainError::SessionAlreadyExists(
            session.as_str().to_string(),
        ));
    }

    let timestamp = Utc::now().timestamp();
    let id = QueueEntryId::new(format!("{}-{}", session.as_str(), timestamp))
        .map_err(|_| DomainError::InvalidSessionName("failed to create entry ID".to_string()))?;

    let entry = QueueEntry::new(id, session, priority)?;

    let new_queue = queue.enqueue(entry);
    Ok(new_queue)
}

/// Dequeue a session from the queue
///
/// This use case validates the session name and removes it from the queue,
/// returning a new queue without the session.
pub fn dequeue_session(queue: Queue, session_name: &str) -> UseCaseResult<Queue> {
    let session = SessionName::new(session_name).map_err(|_| {
        DomainError::InvalidSessionName(if session_name.trim().is_empty() {
            "cannot be empty".to_string()
        } else {
            format!("invalid characters in '{}'", session_name)
        })
    })?;

    let entry_id = queue
        .find_by_session(&session)
        .ok_or_else(|| DomainError::SessionNotFound(session.as_str().to_string()))?
        .id
        .clone();

    let (new_queue, _removed) = queue.dequeue(&entry_id);
    Ok(new_queue)
}

/// Insert a session at a specific position in the queue
///
/// This use case validates the session name, priority, and position,
/// then inserts the session at the specified position regardless of priority.
pub fn insert_at_position(
    queue: Queue,
    session_name: &str,
    position: usize,
    priority: u32,
) -> UseCaseResult<Queue> {
    let session = SessionName::new(session_name).map_err(|_| {
        DomainError::InvalidSessionName(if session_name.trim().is_empty() {
            "cannot be empty".to_string()
        } else {
            format!("invalid characters in '{}'", session_name)
        })
    })?;

    if priority > MAX_PRIORITY {
        return Err(DomainError::InvalidPriority(priority));
    }

    if position > queue.len() {
        return Err(DomainError::PositionOutOfBounds {
            position,
            length: queue.len(),
        });
    }

    if queue.find_by_session(&session).is_some() {
        return Err(DomainError::SessionAlreadyExists(
            session.as_str().to_string(),
        ));
    }

    let timestamp = Utc::now().timestamp();
    let id = QueueEntryId::new(format!("{}-{}", session.as_str(), timestamp))
        .map_err(|_| DomainError::InvalidSessionName("failed to create entry ID".to_string()))?;
    let entry = QueueEntry::new(id, session, priority)?;

    queue.with_entry(position, entry).map_err(|e| match e {
        crate::domain::ValidationError::OutOfBounds { position, length } => {
            DomainError::PositionOutOfBounds { position, length }
        }
        _ => DomainError::InvalidSessionName("Unknown error".to_string()),
    })
}

/// Remove the entry at a specific position
pub fn remove_at_position(queue: Queue, position: usize) -> UseCaseResult<Queue> {
    if position >= queue.len() {
        return Err(DomainError::PositionOutOfBounds {
            position,
            length: queue.len(),
        });
    }

    let entries = queue.entries();
    let entry = entries
        .get(position)
        .ok_or_else(|| DomainError::PositionOutOfBounds {
            position,
            length: queue.len(),
        })?;

    let entry_id = entry.id.clone();

    let (new_queue, _removed) = queue.dequeue(&entry_id);
    Ok(new_queue)
}

/// List all entries in the queue as view models
#[must_use]
pub fn list_queue(queue: &Queue) -> Vec<QueueEntryView> {
    queue
        .entries()
        .iter()
        .enumerate()
        .map(|(position, entry)| QueueEntryView::new(position, entry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_queue() -> Queue {
        let entry1 = QueueEntry::new("session-1", "session-1", 10).unwrap();
        let entry2 = QueueEntry::new("session-2", "session-2", 20).unwrap();
        Queue::new().enqueue(entry1).enqueue(entry2)
    }

    #[test]
    fn test_enqueue_session_adds_to_empty_queue() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "new-session", 50);

        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn test_enqueue_session_maintains_priority_order() {
        let queue = Queue::new();
        let queue = enqueue_session(queue, "low-priority", 100).unwrap();
        let queue = enqueue_session(queue, "high-priority", 1).unwrap();
        let queue = enqueue_session(queue, "mid-priority", 50).unwrap();

        let entries = queue.entries();
        assert_eq!(entries[0].session.as_str(), "high-priority");
        assert_eq!(entries[1].session.as_str(), "mid-priority");
        assert_eq!(entries[2].session.as_str(), "low-priority");
    }

    #[test]
    fn test_enqueue_session_rejects_invalid_priority() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "session", 101);

        assert!(matches!(result, Err(DomainError::InvalidPriority(101))));
    }

    #[test]
    fn test_enqueue_session_rejects_empty_session_name() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "", 50);

        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_enqueue_session_rejects_shell_metacharacters() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "session$name", 50);

        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_enqueue_session_rejects_duplicate_session() {
        let queue = create_test_queue();
        let result = enqueue_session(queue, "session-1", 30);

        assert!(matches!(result, Err(DomainError::SessionAlreadyExists(_))));
    }

    #[test]
    fn test_dequeue_session_removes_existing_session() {
        let queue = create_test_queue();
        let result = dequeue_session(queue, "session-1");

        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn test_dequeue_session_errors_on_nonexistent_session() {
        let queue = create_test_queue();
        let result = dequeue_session(queue, "nonexistent");

        assert!(matches!(result, Err(DomainError::SessionNotFound(_))));
    }

    #[test]
    fn test_insert_at_position_inserts_at_front() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "new-session", 0, 5);

        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 3);
    }

    #[test]
    fn test_insert_at_position_rejects_out_of_bounds() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "new-session", 10, 5);

        assert!(matches!(
            result,
            Err(DomainError::PositionOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_remove_at_position_removes_first_entry() {
        let queue = create_test_queue();
        let result = remove_at_position(queue, 0);

        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn test_remove_at_position_rejects_out_of_bounds() {
        let queue = create_test_queue();
        let result = remove_at_position(queue, 5);

        assert!(matches!(
            result,
            Err(DomainError::PositionOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_list_queue_returns_entry_views() {
        let queue = create_test_queue();
        let views = list_queue(&queue);

        assert_eq!(views.len(), 2);
        assert_eq!(views[0].position, 0);
        assert_eq!(views[0].session, "session-1");
        assert_eq!(views[0].priority, 10);
    }

    #[test]
    fn test_list_queue_returns_empty_for_empty_queue() {
        let queue = Queue::new();
        let views = list_queue(&queue);

        assert!(views.is_empty());
    }

    #[test]
    fn test_railway_chain_enqueue_then_dequeue() {
        let queue = Queue::new();

        let result = enqueue_session(queue, "session-1", 10)
            .and_then(|q| enqueue_session(q, "session-2", 20))
            .and_then(|q| dequeue_session(q, "session-1"));

        assert!(result.is_ok());
        let final_queue = result.unwrap();
        assert_eq!(final_queue.len(), 1);
    }

    #[test]
    fn test_railway_chain_fails_on_first_error() {
        let queue = Queue::new();

        let result = enqueue_session(queue, "session-1", 10)
            .and_then(|q| enqueue_session(q, "session-2", 200))
            .and_then(|q| enqueue_session(q, "session-3", 30));

        assert!(matches!(result, Err(DomainError::InvalidPriority(_))));
    }

    #[test]
    fn test_domain_error_display_all_variants() {
        assert!(format!("{}", DomainError::InvalidSessionName("bad".into())).contains("bad"));
        assert!(format!("{}", DomainError::InvalidPriority(200)).contains("200"));
        let pos_err = DomainError::PositionOutOfBounds { position: 5, length: 3 };
        let pos_msg = format!("{pos_err}");
        assert!(pos_msg.contains("5") && pos_msg.contains("3"));
        assert!(format!("{}", DomainError::SessionNotFound("x".into())).contains("x"));
        assert!(format!("{}", DomainError::QueueEmpty).contains("empty"));
        assert!(format!("{}", DomainError::SessionAlreadyExists("s".into())).contains("s"));
        let trans_err = DomainError::InvalidStateTransition {
            from: "A".into(),
            to: "B".into(),
        };
        let trans_msg = format!("{trans_err}");
        assert!(trans_msg.contains("A") && trans_msg.contains("B"));
    }

    #[test]
    fn test_domain_error_debug() {
        let err = DomainError::QueueEmpty;
        let debug = format!("{err:?}");
        assert!(debug.contains("QueueEmpty"));
    }

    #[test]
    fn test_domain_error_clone_and_eq() {
        let a = DomainError::QueueEmpty;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_domain_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(DomainError::QueueEmpty);
        let _ = format!("{err:?}");
    }

    #[test]
    fn test_queue_entry_view_new() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let view = QueueEntryView::new(0, &entry);
        assert_eq!(view.position, 0);
        assert_eq!(view.session, "session-1");
        assert_eq!(view.priority, 10);
        assert_eq!(view.status, "pending");
    }

    #[test]
    fn test_queue_entry_view_equality() {
        let entry1 = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let entry2 = QueueEntry::new("id-2", "session-2", 20).unwrap();
        let view1 = QueueEntryView::new(0, &entry1);
        let view2 = QueueEntryView::new(0, &entry1);
        let view3 = QueueEntryView::new(1, &entry2);
        assert_eq!(view1, view2);
        assert_ne!(view1, view3);
    }

    #[test]
    fn test_dequeue_session_empty_name() {
        let queue = Queue::new();
        let result = dequeue_session(queue, "");
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_dequeue_session_with_metacharacters() {
        let queue = Queue::new();
        let result = dequeue_session(queue, "ses|on");
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_insert_at_position_duplicate_session() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "session-1", 0, 50);
        assert!(matches!(result, Err(DomainError::SessionAlreadyExists(_))));
    }

    #[test]
    fn test_insert_at_position_invalid_priority() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "new-session", 0, 200);
        assert!(matches!(result, Err(DomainError::InvalidPriority(_))));
    }

    #[test]
    fn test_insert_at_position_invalid_session() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "bad$name", 0, 50);
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_remove_at_position_empty_queue() {
        let queue = Queue::new();
        let result = remove_at_position(queue, 0);
        assert!(matches!(result, Err(DomainError::PositionOutOfBounds { .. })));
    }

    #[test]
    fn test_enqueue_session_accepts_max_priority() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "session-1", 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enqueue_session_accepts_zero_priority() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "session-0", 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_queue_maps_all_entries() {
        let queue = create_test_queue();
        let views = list_queue(&queue);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].position, 0);
        assert_eq!(views[1].position, 1);
    }

    #[test]
    fn test_use_case_result_type() {
        let ok: UseCaseResult<i32> = Ok(42);
        let err: UseCaseResult<i32> = Err(DomainError::QueueEmpty);
        assert_eq!(ok.unwrap(), 42);
        assert!(err.is_err());
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn test_enqueue_session_with_whitespace_trimmed() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "  spaced-session  ", 50);
        assert!(result.is_ok());
        let entries = result.unwrap().entries();
        assert_eq!(entries[0].session.as_str(), "spaced-session");
    }

    #[test]
    fn test_enqueue_session_with_zero_priority() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "zero-prio", 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entries()[0].priority, 0);
    }

    #[test]
    fn test_enqueue_session_with_max_priority() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "max-prio", 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entries()[0].priority, 100);
    }

    #[test]
    fn test_enqueue_session_priority_one_above_max_rejected() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "over-max", 101);
        assert!(matches!(result, Err(DomainError::InvalidPriority(101))));
    }

    #[test]
    fn test_dequeue_session_leaves_other_entries() {
        let queue = create_test_queue();
        let result = dequeue_session(queue, "session-1");
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
        assert_eq!(new_queue.entries()[0].session.as_str(), "session-2");
    }

    #[test]
    fn test_dequeue_session_whitespace_name_rejected() {
        let queue = Queue::new();
        let result = dequeue_session(queue, "   ");
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_insert_at_position_at_end() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "end-session", 2, 50);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 3);
        assert_eq!(new_queue.entries()[2].session.as_str(), "end-session");
    }

    #[test]
    fn test_insert_at_position_in_middle() {
        let queue = create_test_queue();
        let result = insert_at_position(queue, "mid-session", 1, 15);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 3);
        assert_eq!(new_queue.entries()[1].session.as_str(), "mid-session");
    }

    #[test]
    fn test_insert_at_position_empty_queue() {
        let queue = Queue::new();
        let result = insert_at_position(queue, "first", 0, 50);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_insert_at_position_empty_name_rejected() {
        let queue = Queue::new();
        let result = insert_at_position(queue, "", 0, 50);
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_remove_at_position_last_entry() {
        let queue = create_test_queue();
        let result = remove_at_position(queue, 1);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 1);
        assert_eq!(new_queue.entries()[0].session.as_str(), "session-1");
    }

    #[test]
    fn test_list_queue_entry_view_status_field() {
        let queue = create_test_queue();
        let views = list_queue(&queue);
        assert_eq!(views[0].status, "pending");
        assert_eq!(views[1].status, "pending");
    }

    #[test]
    fn test_list_queue_entry_view_enqueued_at_set() {
        let before = Utc::now();
        let queue = create_test_queue();
        let views = list_queue(&queue);
        let after = Utc::now();
        for view in &views {
            assert!(view.enqueued_at >= before);
            assert!(view.enqueued_at <= after);
        }
    }

    #[test]
    fn test_queue_entry_view_clone() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let view = QueueEntryView::new(0, &entry);
        let cloned = view.clone();
        assert_eq!(view, cloned);
    }

    #[test]
    fn test_enqueue_then_dequeue_empty() {
        let queue = Queue::new();
        let queue = enqueue_session(queue, "s1", 10).unwrap();
        let result = dequeue_session(queue, "s1");
        assert!(result.is_ok());
        let final_queue = result.unwrap();
        assert!(final_queue.is_empty());
    }

    #[test]
    fn test_enqueue_session_with_tab_rejected() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "ses\tion", 50);
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_enqueue_session_with_newline_rejected() {
        let queue = Queue::new();
        let result = enqueue_session(queue, "ses\nion", 50);
        assert!(matches!(result, Err(DomainError::InvalidSessionName(_))));
    }

    #[test]
    fn test_domain_error_invalid_state_transition_display() {
        let err = DomainError::InvalidStateTransition {
            from: "Pending".into(),
            to: "Merged".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Pending"));
        assert!(msg.contains("Merged"));
    }

    #[test]
    fn test_domain_error_position_out_of_bounds_display() {
        let err = DomainError::PositionOutOfBounds { position: 99, length: 5 };
        let msg = format!("{err}");
        assert!(msg.contains("99"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn test_domain_error_session_not_found_display() {
        let err = DomainError::SessionNotFound("missing-session".into());
        let msg = format!("{err}");
        assert!(msg.contains("missing-session"));
    }

    #[test]
    fn test_domain_error_queue_empty_display() {
        let err = DomainError::QueueEmpty;
        let msg = format!("{err}");
        assert!(msg.contains("empty"));
    }
}
