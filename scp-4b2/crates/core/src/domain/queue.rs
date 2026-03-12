#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Domain layer - Pure functional queue implementation
//!
//! This module provides an immutable, persistent queue using functional paradigms:
//! - All operations return new Queue instances (persistent data structure)
//! - Railway-Oriented Programming with `Result` return types
//! - Pure functions - no I/O, no side effects
//! - Domain validation errors (ValidationError)
//! - Functional patterns: iterators, combinators, no for loops

use chrono::{DateTime, Utc};
use std::cmp::Ordering;

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::validation::{ValidationError, ValidationResult};

/// Maximum priority value for queue entries
pub const MAX_PRIORITY: u32 = 100;

/// Status of a queue entry
///
/// Represents the state machine for a queue entry through its lifecycle.
/// All state transitions are validated via `transition_to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum QueueStatus {
    /// Waiting to be processed
    Pending,
    /// Claimed by an agent
    Claimed,
    /// Currently being rebased
    Rebasing,
    /// Running tests
    Testing,
    /// Ready to merge
    ReadyToMerge,
    /// Currently merging
    Merging,
    /// Successfully merged
    Merged,
    /// Failed with retryable error
    FailedRetryable,
    /// Failed terminally
    FailedTerminal,
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Claimed => write!(f, "claimed"),
            Self::Rebasing => write!(f, "rebasing"),
            Self::Testing => write!(f, "testing"),
            Self::ReadyToMerge => write!(f, "ready_to_merge"),
            Self::Merging => write!(f, "merging"),
            Self::Merged => write!(f, "merged"),
            Self::FailedRetryable => write!(f, "failed_retryable"),
            Self::FailedTerminal => write!(f, "failed_terminal"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl QueueStatus {
    /// Check if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }

    /// Check if this is a failed state.
    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::FailedRetryable | Self::FailedTerminal)
    }

    /// Try to transition to a new state using Railway-Oriented Programming.
    ///
    /// # Errors
    /// Returns `ValidationError::InvalidStateTransition` if the transition is not allowed.
    pub fn transition_to(self, new_status: Self) -> ValidationResult<Self> {
        match (self, new_status) {
            // Valid transitions from Pending
            (Self::Pending, Self::Claimed | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Claimed
            (Self::Claimed, Self::Rebasing | Self::Cancelled) => Ok(new_status),

            // Valid transitions from Rebasing
            (Self::Rebasing, Self::Testing | Self::FailedRetryable) => Ok(new_status),

            // Valid transitions from Testing
            (Self::Testing, Self::ReadyToMerge | Self::FailedRetryable | Self::FailedTerminal) => {
                Ok(new_status)
            }

            // Valid transitions from ReadyToMerge
            (Self::ReadyToMerge, Self::Merging | Self::FailedRetryable) => Ok(new_status),

            // Valid transitions from Merging
            (Self::Merging, Self::Merged | Self::FailedRetryable) => Ok(new_status),

            // Valid transitions from FailedRetryable
            (Self::FailedRetryable, Self::Pending | Self::Cancelled) => Ok(new_status),

            // Terminal states and invalid transitions
            _ => Err(ValidationError::InvalidStateTransition {
                from: self.to_string(),
                to: new_status.to_string(),
            }),
        }
    }
}

/// A queue entry representing a session waiting to be merged.
///
/// This is a value object - immutable and validated on construction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueEntry {
    /// Unique identifier
    pub id: QueueEntryId,
    /// Session name
    pub session: SessionName,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When enqueued
    pub enqueued_at: DateTime<Utc>,
    /// Current status
    pub status: QueueStatus,
}

impl QueueEntry {
    /// Create a new queue entry with validation.
    ///
    /// # Errors
    /// Returns `ValidationError` if:
    /// - The priority exceeds `MAX_PRIORITY`
    /// - The ID is invalid (via `QueueEntryId::new`)
    /// - The session name is invalid (via `SessionName::new`)
    pub fn new(
        id: impl Into<String>,
        session: impl Into<String>,
        priority: u32,
    ) -> ValidationResult<Self> {
        let id = QueueEntryId::new(id)?;
        let session = SessionName::new(session)?;

        validate_range(priority, 0, MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        })
    }

    /// Create a new queue entry from validated identifiers.
    ///
    /// # Errors
    /// Returns `ValidationError` if priority is out of range.
    pub fn from_identifiers(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        })
    }

    /// Create a new queue entry with explicit timestamp (for testing/rehydration).
    ///
    /// # Errors
    /// Returns `ValidationError` if validation fails.
    pub fn with_timestamp(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
        enqueued_at: DateTime<Utc>,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at,
            status: QueueStatus::Pending,
        })
    }

    /// Create a queue entry with a specific status (for rehydration).
    ///
    /// # Errors
    /// Returns `ValidationError` if priority is out of range.
    pub fn with_status(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
        enqueued_at: DateTime<Utc>,
        status: QueueStatus,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at,
            status,
        })
    }

    /// Transition to a new status, returning a new entry.
    ///
    /// # Errors
    /// Returns `ValidationError` if the transition is invalid.
    pub fn transition_status(self, new_status: QueueStatus) -> ValidationResult<Self> {
        self.status
            .transition_to(new_status)
            .map(|status| QueueEntry { status, ..self })
    }

    /// Update the priority, returning a new entry.
    ///
    /// # Errors
    /// Returns `ValidationError` if the priority is out of range.
    pub fn with_priority(self, priority: u32) -> ValidationResult<Self> {
        validate_range(priority, 0, MAX_PRIORITY, "priority")?;
        Ok(QueueEntry { priority, ..self })
    }
}

/// Partial equality for QueueEntry (ignores timestamp)
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.session == other.session
            && self.priority == other.priority
            && self.status == other.status
    }
}

impl Eq for QueueEntry {}

/// The merge queue - an immutable persistent data structure.
///
/// All operations return new Queue instances, preserving structural sharing
/// where possible. This enables safe concurrent access and easy undo/redo.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Queue {
    entries: Vec<QueueEntry>,
}

impl Queue {
    /// Create a new empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a queue from a vector of entries (for testing/serialization).
    #[must_use]
    pub fn from_entries(entries: Vec<QueueEntry>) -> Self {
        Self { entries }
    }

    /// Create a queue with entries sorted by priority.
    #[must_use]
    pub fn from_entries_sorted(mut entries: Vec<QueueEntry>) -> Self {
        entries.sort_by_key(|e| e.priority);
        Self { entries }
    }

    /// Get the number of entries in the queue.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the queue is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries as a slice.
    #[must_use]
    pub fn entries(&self) -> &[QueueEntry] {
        &self.entries
    }

    /// Find an entry by ID using functional find.
    #[must_use]
    pub fn find(&self, id: &QueueEntryId) -> Option<&QueueEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Find an entry by session name using functional find.
    #[must_use]
    pub fn find_by_session(&self, session: &SessionName) -> Option<&QueueEntry> {
        self.entries.iter().find(|e| &e.session == session)
    }

    /// Get the next pending entry using functional find.
    #[must_use]
    pub fn next_pending(&self) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|e| e.status == QueueStatus::Pending)
    }

    /// Add an entry to the queue, returning a new Queue.
    ///
    /// Uses binary search to maintain priority order.
    #[must_use]
    pub fn enqueue(&self, entry: QueueEntry) -> Self {
        let priority = entry.priority;

        // Use binary_search_by for functional insertion point
        let insert_pos = self
            .entries
            .binary_search_by_key(&priority, |e| e.priority)
            .unwrap_or_else(|pos| pos);

        let mut new_entries = self.entries.clone();
        new_entries.insert(insert_pos, entry);

        Self {
            entries: new_entries,
        }
    }

    /// Remove an entry from the queue by ID, returning (new_queue, removed_entry).
    ///
    /// Uses functional patterns to find and remove the entry.
    #[must_use]
    pub fn dequeue(&self, id: &QueueEntryId) -> (Self, Option<QueueEntry>) {
        match self.entries.iter().position(|e| &e.id == id) {
            Some(idx) => {
                let mut new_entries = self.entries.clone();
                let removed = new_entries.remove(idx);
                (
                    Self {
                        entries: new_entries,
                    },
                    Some(removed),
                )
            }
            None => (self.clone(), None),
        }
    }

    /// Insert an entry at a specific position, returning Result<Queue, ValidationError>.
    ///
    /// Uses Railway-Oriented Programming for validation.
    ///
    /// # Errors
    /// Returns `ValidationError::OutOfBounds` if position is invalid.
    pub fn with_entry(&self, position: usize, entry: QueueEntry) -> ValidationResult<Self> {
        if position > self.entries.len() {
            return Err(ValidationError::OutOfBounds {
                position,
                length: self.entries.len(),
            });
        }

        let mut new_entries = self.entries.clone();
        new_entries.insert(position, entry);

        Ok(Self {
            entries: new_entries,
        })
    }

    /// Update an entry's status by ID, returning Result<Queue, ValidationError>.
    ///
    /// # Errors
    /// Returns `ValidationError::NotFound` if the entry doesn't exist or
    /// `ValidationError::InvalidStateTransition` if the transition is invalid.
    pub fn update_status(
        &self,
        id: &QueueEntryId,
        new_status: QueueStatus,
    ) -> ValidationResult<Self> {
        self.find(id)
            .ok_or_else(|| ValidationError::NotFound {
                field: "entry".to_string(),
                value: id.to_string(),
            })
            .and_then(|entry| entry.status.transition_to(new_status))
            .map(|_| {
                // Use functional iteration to find and update index
                self.entries
                    .iter()
                    .position(|e| &e.id == id)
                    .map(|idx| {
                        let mut new_entries = self.entries.clone();
                        new_entries[idx].status = new_status;
                        Self {
                            entries: new_entries,
                        }
                    })
                    .unwrap_or_else(|| {
                        // Should never happen since we validated above
                        self.clone()
                    })
            })
    }

    /// Remove an entry at a specific position.
    ///
    /// # Errors
    /// Returns `ValidationError::OutOfBounds` if the position is invalid.
    pub fn remove_at(&self, position: usize) -> ValidationResult<(Self, QueueEntry)> {
        if position >= self.entries.len() {
            return Err(ValidationError::OutOfBounds {
                position,
                length: self.entries.len(),
            });
        }
        let mut new_entries = self.entries.clone();
        let removed = new_entries.remove(position);
        Ok((
            Self {
                entries: new_entries,
            },
            removed,
        ))
    }

    /// Filter entries by predicate using functional filter.
    #[must_use]
    pub fn filter<F>(&self, predicate: F) -> Vec<&QueueEntry>
    where
        F: Fn(&&QueueEntry) -> bool,
    {
        self.entries.iter().filter(predicate).collect()
    }

    /// Map entries using functional map.
    #[must_use]
    pub fn map<T, F>(&self, f: F) -> Vec<T>
    where
        F: Fn(&QueueEntry) -> T,
    {
        self.entries.iter().map(f).collect()
    }

    /// Fold/reduce over entries using functional fold.
    #[must_use]
    pub fn fold<T, F>(&self, initial: T, f: F) -> T
    where
        F: Fn(T, &QueueEntry) -> T,
    {
        self.entries.iter().fold(initial, f)
    }

    /// Check if predicate holds for any entry using functional any.
    #[must_use]
    pub fn any<F>(&self, predicate: F) -> bool
    where
        F: Fn(&QueueEntry) -> bool,
    {
        self.entries.iter().any(predicate)
    }

    /// Check if predicate holds for all entries using functional all.
    #[must_use]
    pub fn all<F>(&self, predicate: F) -> bool
    where
        F: Fn(&QueueEntry) -> bool,
    {
        self.entries.iter().all(predicate)
    }

    /// Get entries grouped by status using functional grouping.
    #[must_use]
    pub fn group_by_status(&self) -> Vec<(QueueStatus, Vec<&QueueEntry>)> {
        use std::collections::HashMap;

        self.entries
            .iter()
            .fold::<HashMap<QueueStatus, Vec<&QueueEntry>>, _>(HashMap::new(), |mut acc, entry| {
                acc.entry(entry.status).or_default().push(entry);
                acc
            })
            .into_iter()
            .collect()
    }

    /// Count entries that are not merged using functional counting.
    #[must_use]
    pub fn count_active(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.status.is_terminal())
            .count()
    }

    /// Get entries sorted by a custom key using functional sorting.
    #[must_use]
    pub fn sorted_by_key<F, K>(&self, f: F) -> Vec<&QueueEntry>
    where
        F: Fn(&QueueEntry) -> K,
        K: Ord,
    {
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by_key(|e| f(e));
        entries
    }

    /// Partition entries by predicate.
    #[must_use]
    pub fn partition<F>(&self, predicate: F) -> (Vec<&QueueEntry>, Vec<&QueueEntry>)
    where
        F: Fn(&&QueueEntry) -> bool,
    {
        self.entries.iter().partition(predicate)
    }

    /// Convert into the inner vector of entries.
    #[must_use]
    pub fn into_inner(self) -> Vec<QueueEntry> {
        self.entries
    }
}

/// Railway combinator: Validate a value is within a range
fn validate_range(value: u32, min: u32, max: u32, field: &str) -> ValidationResult<u32> {
    match value.cmp(&min) {
        Ordering::Less => Err(ValidationError::BelowMinimum {
            field: field.to_string(),
            value,
            min,
        }),
        Ordering::Greater if value > max => Err(ValidationError::ExceedsMaximum {
            field: field.to_string(),
            value,
            max,
        }),
        _ => Ok(value),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
        // Create an entry with a terminal status via valid transitions
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

        assert!(matches!(result, Err(ValidationError::OutOfBounds { .. })));
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
            Err(ValidationError::InvalidStateTransition { .. })
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
            Err(ValidationError::ExceedsMaximum { .. })
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
        assert!(matches!(result, Err(ValidationError::EmptyValue(_))));
    }

    #[test]
    fn test_queue_entry_new_rejects_empty_session() {
        let result = QueueEntry::new("test-1", "", 10);
        assert!(matches!(result, Err(ValidationError::EmptyValue(_))));
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
            Err(ValidationError::InvalidStateTransition { .. })
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
            Err(ValidationError::ExceedsMaximum { .. })
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
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            QueueEntryId::new("   "),
            Err(ValidationError::EmptyValue(_))
        ));
    }

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::new("my-session").is_ok());
        assert!(SessionName::new("  my-session  ").is_ok());
        assert!(SessionName::new("session_123").is_ok());
        assert!(SessionName::new("session.with.dots").is_ok());
    }

    #[test]
    fn test_session_name_empty() {
        assert!(matches!(
            SessionName::new(""),
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            SessionName::new("   "),
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
    fn test_session_name_validate_works() {
        assert!(SessionName::validate("valid-name").is_ok());
        assert!(SessionName::validate("invalid$name").is_err());
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
        let session = SessionName::new("session").unwrap();
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
        let session = SessionName::new("session").unwrap();
        let timestamp = Utc::now();
        let entry = QueueEntry::with_timestamp(id, session, 50, timestamp).unwrap();
        assert_eq!(entry.enqueued_at, timestamp);
    }

    #[test]
    fn test_queue_entry_with_status() {
        let id = QueueEntryId::new("test-1").unwrap();
        let session = SessionName::new("session").unwrap();
        let timestamp = Utc::now();
        let entry =
            QueueEntry::with_status(id, session, 50, timestamp, QueueStatus::Claimed).unwrap();
        assert_eq!(entry.status, QueueStatus::Claimed);
    }
}
