//! Queue - An immutable persistent data structure for merge queue entries
//!
//! All operations return new Queue instances, preserving structural sharing
//! where possible. This enables safe concurrent access and easy undo/redo.

use super::entry::QueueEntry;
use super::status::QueueStatus;
#[allow(unused_imports)]
use crate::domain::contracts::{ensures, invariant, requires};
use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::validation::{ValidationError, ValidationResult};

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
    pub const fn from_entries(entries: Vec<QueueEntry>) -> Self {
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
    #[ensures(self.len() + 1 == ret.len(), "queue length increases by 1")]
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

    /// Remove an entry from the queue by ID, returning (`new_queue`, `removed_entry`).
    ///
    /// Uses functional patterns to find and remove the entry.
    #[must_use]
    pub fn dequeue(&self, id: &QueueEntryId) -> (Self, Option<QueueEntry>) {
        self.entries.iter().position(|e| &e.id == id).map_or_else(
            || (self.clone(), None),
            |idx| {
                let mut new_entries = self.entries.clone();
                let removed = new_entries.remove(idx);
                (
                    Self {
                        entries: new_entries,
                    },
                    Some(removed),
                )
            },
        )
    }

    /// Insert an entry at a given position.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
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

    /// Update an entry's status by ID, returning Result<Queue, `ValidationError`>.
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
            .and_then(|entry| entry.status.transition_to(new_status))?;

        Ok(self.with_updated_entry_status(id, new_status))
    }

    fn with_updated_entry_status(&self, id: &QueueEntryId, new_status: QueueStatus) -> Self {
        let mut new_entries = self.entries.clone();
        if let Some(idx) = new_entries.iter().position(|e| &e.id == id) {
            new_entries[idx].status = new_status;
        }
        Self {
            entries: new_entries,
        }
    }

    /// Remove an entry at a given position.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
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
