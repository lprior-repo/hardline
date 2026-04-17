//! Merge queue - Immutable persistent data structure

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::validation::{ValidationError, ValidationResult};

use super::entry::QueueEntry;
use super::status::QueueStatus;

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
    /// Entries with the same priority maintain FIFO order (insert after existing entries).
    #[must_use]
    pub fn enqueue(&self, entry: QueueEntry) -> Self {
        let priority = entry.priority;

        let base_pos = self
            .entries
            .binary_search_by_key(&priority, |e| e.priority)
            .unwrap_or_else(|pos| pos);

        let mut insert_pos = base_pos;
        while insert_pos < self.entries.len() && self.entries[insert_pos].priority == priority {
            insert_pos += 1;
        }

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
                    .unwrap_or_else(|| self.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::{QueueEntryId, SessionName};
    use crate::domain::queue::status::MAX_PRIORITY;

    fn make_entry(id: &str, session: &str, priority: u32) -> QueueEntry {
        QueueEntry::new(id, session, priority).unwrap()
    }

    #[test]
    fn queue_from_entries_preserves_order() {
        let entries = vec![
            make_entry("c", "s-c", 30),
            make_entry("a", "s-a", 10),
            make_entry("b", "s-b", 20),
        ];
        let queue = Queue::from_entries(entries);
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.entries()[0].id.as_str(), "c");
    }

    #[test]
    fn queue_from_entries_sorted() {
        let entries = vec![
            make_entry("c", "s-c", 30),
            make_entry("a", "s-a", 10),
            make_entry("b", "s-b", 20),
        ];
        let queue = Queue::from_entries_sorted(entries);
        let priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
        assert_eq!(priorities, vec![10, 20, 30]);
    }

    #[test]
    fn queue_sorted_by_key() {
        let queue = Queue::new()
            .enqueue(make_entry("a", "s-a", 30))
            .enqueue(make_entry("b", "s-b", 10))
            .enqueue(make_entry("c", "s-c", 20));

        let sorted = queue.sorted_by_key(|e| e.priority);
        let priorities: Vec<u32> = sorted.iter().map(|e| e.priority).collect();
        assert_eq!(priorities, vec![10, 20, 30]);
    }

    #[test]
    fn queue_partition() {
        let queue = Queue::new().enqueue(make_entry("a", "s-a", 10)).enqueue(
            make_entry("b", "s-b", 20)
                .transition_status(QueueStatus::Claimed)
                .unwrap(),
        );

        let (pending, non_pending) = queue.partition(|e| e.status == QueueStatus::Pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(non_pending.len(), 1);
        assert_eq!(pending[0].id.as_str(), "a");
    }

    #[test]
    fn queue_into_inner() {
        let queue = Queue::new()
            .enqueue(make_entry("a", "s-a", 10))
            .enqueue(make_entry("b", "s-b", 20));

        let entries = queue.into_inner();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn queue_dequeue_empty_returns_same_queue() {
        let queue = Queue::new();
        let id = QueueEntryId::new("nope").unwrap();
        let (new_queue, removed) = queue.dequeue(&id);
        assert!(removed.is_none());
        assert!(new_queue.is_empty());
    }

    #[test]
    fn queue_enqueue_fifo_within_same_priority() {
        let queue = Queue::new()
            .enqueue(make_entry("first", "s1", 10))
            .enqueue(make_entry("second", "s2", 10))
            .enqueue(make_entry("third", "s3", 10));

        let ids: Vec<&str> = queue.entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["first", "second", "third"]);
    }

    #[test]
    fn queue_update_status_chains_through_states() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let id = QueueEntryId::new("e1").unwrap();

        let queue = queue.update_status(&id, QueueStatus::Claimed).unwrap();
        let queue = queue.update_status(&id, QueueStatus::Rebasing).unwrap();
        let queue = queue.update_status(&id, QueueStatus::Testing).unwrap();
        let queue = queue.update_status(&id, QueueStatus::ReadyToMerge).unwrap();
        let queue = queue.update_status(&id, QueueStatus::Merging).unwrap();
        let queue = queue.update_status(&id, QueueStatus::Merged).unwrap();

        assert_eq!(queue.find(&id).unwrap().status, QueueStatus::Merged);
    }

    #[test]
    fn queue_with_entry_at_end() {
        let queue = Queue::new().enqueue(make_entry("a", "s-a", 10));
        let new_entry = make_entry("b", "s-b", 20);
        let result = queue.with_entry(1, new_entry);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 2);
        assert_eq!(new_queue.entries()[1].id.as_str(), "b");
    }

    #[test]
    fn queue_default_is_empty() {
        let queue = Queue::default();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn queue_clone() {
        let queue = Queue::new().enqueue(make_entry("a", "s-a", 10));
        let cloned = queue.clone();
        assert_eq!(cloned.len(), queue.len());
    }

    #[test]
    fn queue_debug() {
        let queue = Queue::new().enqueue(make_entry("a", "s-a", 10));
        let debug = format!("{queue:?}");
        assert!(debug.contains("entries"));
    }

    #[test]
    fn queue_serde_roundtrip() {
        let queue = Queue::new()
            .enqueue(make_entry("a", "s-a", 10))
            .enqueue(make_entry("b", "s-b", 20));
        let json = serde_json::to_string(&queue).unwrap();
        let back: Queue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.entries()[0].id.as_str(), "a");
    }

    #[test]
    fn queue_next_pending_skips_claimed() {
        let queue = Queue::new()
            .enqueue(
                make_entry("a", "s-a", 10)
                    .transition_status(QueueStatus::Claimed)
                    .unwrap(),
            )
            .enqueue(make_entry("b", "s-b", 20));

        let next = queue.next_pending();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id.as_str(), "b");
    }

    #[test]
    fn queue_empty_entries_slice() {
        let queue = Queue::new();
        assert!(queue.entries().is_empty());
    }

    #[test]
    fn queue_fold_empty() {
        let queue = Queue::new();
        let result = queue.fold(0, |acc, e| acc + e.priority);
        assert_eq!(result, 0);
    }

    #[test]
    fn queue_any_empty() {
        let queue = Queue::new();
        assert!(!queue.any(|_| true));
    }

    #[test]
    fn queue_all_empty() {
        let queue = Queue::new();
        assert!(queue.all(|_| true));
    }

    #[test]
    fn queue_find_by_session_nonexistent() {
        let queue = Queue::new();
        let session = SessionName::new("nope").unwrap();
        assert!(queue.find_by_session(&session).is_none());
    }

    #[test]
    fn queue_count_active_empty() {
        let queue = Queue::new();
        assert_eq!(queue.count_active(), 0);
    }

    #[test]
    fn queue_count_active_all_terminal() {
        let merged_entry = make_entry("a", "s-a", 10)
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
        let queue = Queue::new().enqueue(merged_entry);
        assert_eq!(queue.count_active(), 0);
    }

    #[test]
    fn queue_group_by_status_empty() {
        let queue = Queue::new();
        let grouped = queue.group_by_status();
        assert!(grouped.is_empty());
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn queue_enqueue_preserves_immutability() {
        let queue = Queue::new();
        let entry = make_entry("e1", "s1", 10);
        let new_queue = queue.enqueue(entry);
        assert!(queue.is_empty(), "Original queue should be unchanged");
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn queue_dequeue_preserves_immutability() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let id = QueueEntryId::new("e1").unwrap();
        let (new_queue, removed) = queue.dequeue(&id);
        assert_eq!(queue.len(), 1, "Original queue should be unchanged");
        assert!(removed.is_some());
        assert!(new_queue.is_empty());
    }

    #[test]
    fn queue_find_by_session_exists() {
        let queue = Queue::new().enqueue(make_entry("e1", "target-session", 10));
        let session = SessionName::new("target-session").unwrap();
        let found = queue.find_by_session(&session);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), "e1");
    }

    #[test]
    fn queue_next_pending_empty_queue() {
        let queue = Queue::new();
        assert!(queue.next_pending().is_none());
    }

    #[test]
    fn queue_next_pending_returns_first_pending() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(make_entry("e2", "s2", 20));
        let next = queue.next_pending();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id.as_str(), "e1");
    }

    #[test]
    fn queue_filter_all_match() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(make_entry("e2", "s2", 20));
        let all = queue.filter(|_| true);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn queue_filter_none_match() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let none = queue.filter(|_| false);
        assert!(none.is_empty());
    }

    #[test]
    fn queue_map_transforms() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let ids: Vec<String> = queue.map(|e| e.id.as_str().to_string());
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "e1");
    }

    #[test]
    fn queue_any_true() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        assert!(queue.any(|e| e.priority == 10));
    }

    #[test]
    fn queue_any_false() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        assert!(!queue.any(|e| e.priority == 999));
    }

    #[test]
    fn queue_all_true() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(make_entry("e2", "s2", 20));
        assert!(queue.all(|e| e.priority <= 20));
    }

    #[test]
    fn queue_all_false() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(make_entry("e2", "s2", 20));
        assert!(!queue.all(|e| e.priority < 20));
    }

    #[test]
    fn queue_sorted_by_key_same_priority() {
        let queue = Queue::new()
            .enqueue(make_entry("a", "s-a", 10))
            .enqueue(make_entry("b", "s-b", 10));
        let sorted = queue.sorted_by_key(|e| e.priority);
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn queue_partition_all_match() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let (yes, no) = queue.partition(|_| true);
        assert_eq!(yes.len(), 1);
        assert!(no.is_empty());
    }

    #[test]
    fn queue_partition_none_match() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let (yes, no) = queue.partition(|_| false);
        assert!(yes.is_empty());
        assert_eq!(no.len(), 1);
    }

    #[test]
    fn queue_count_active_mixed() {
        let merged = make_entry("a", "s-a", 10)
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
        let queue = Queue::new()
            .enqueue(make_entry("b", "s-b", 20))
            .enqueue(merged);
        assert_eq!(queue.count_active(), 1);
    }

    #[test]
    fn queue_group_by_status_multiple_groups() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(
                make_entry("e2", "s2", 20)
                    .transition_status(QueueStatus::Claimed)
                    .unwrap(),
            )
            .enqueue(
                make_entry("e3", "s3", 30)
                    .transition_status(QueueStatus::Cancelled)
                    .unwrap(),
            );
        let grouped = queue.group_by_status();
        assert_eq!(grouped.len(), 3);
    }

    #[test]
    fn queue_with_entry_at_front() {
        let queue = Queue::new().enqueue(make_entry("e1", "s1", 10));
        let new_entry = make_entry("e0", "s0", 5);
        let result = queue.with_entry(0, new_entry);
        assert!(result.is_ok());
        let new_queue = result.unwrap();
        assert_eq!(new_queue.len(), 2);
        assert_eq!(new_queue.entries()[0].id.as_str(), "e0");
    }

    #[test]
    fn queue_with_entry_out_of_bounds() {
        let queue = Queue::new();
        let entry = make_entry("e1", "s1", 10);
        let result = queue.with_entry(1, entry);
        assert!(matches!(result, Err(ValidationError::OutOfBounds { .. })));
    }

    #[test]
    fn queue_update_status_entry_not_found() {
        let queue = Queue::new();
        let id = QueueEntryId::new("nonexistent").unwrap();
        let result = queue.update_status(&id, QueueStatus::Claimed);
        assert!(matches!(result, Err(ValidationError::NotFound { .. })));
    }

    #[test]
    fn queue_remove_at_last_position() {
        let queue = Queue::new()
            .enqueue(make_entry("e1", "s1", 10))
            .enqueue(make_entry("e2", "s2", 20));
        let result = queue.remove_at(1);
        assert!(result.is_ok());
        let (new_queue, removed) = result.unwrap();
        assert_eq!(removed.id.as_str(), "e2");
        assert_eq!(new_queue.len(), 1);
    }

    #[test]
    fn queue_enqueue_max_priority() {
        let entry = QueueEntry::new("e-max", "s-max", MAX_PRIORITY).unwrap();
        let queue = Queue::new().enqueue(entry);
        assert_eq!(queue.entries()[0].priority, MAX_PRIORITY);
    }

    #[test]
    fn queue_enqueue_priority_zero() {
        let entry = QueueEntry::new("e-zero", "s-zero", 0).unwrap();
        let queue = Queue::new().enqueue(entry);
        assert_eq!(queue.entries()[0].priority, 0);
    }

    #[test]
    fn queue_from_entries_empty() {
        let queue = Queue::from_entries(vec![]);
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_from_entries_sorted_empty() {
        let queue = Queue::from_entries_sorted(vec![]);
        assert!(queue.is_empty());
    }
}
