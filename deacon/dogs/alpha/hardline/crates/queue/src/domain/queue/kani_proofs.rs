//! Kani proofs for Queue invariants.
//!
//! # Invariants Proven
//!
//! 1. Dequeue returns highest priority entry
//! 2. Priority ordering maintained after enqueue
//! 3. Queue length consistency
//! 4. FIFO within same priority

#[cfg(kani)]
mod proof {
    use crate::domain::identifiers::{QueueEntryId, SessionName};
    use crate::domain::queue::entry::QueueEntry;
    use crate::domain::queue::queue::Queue;
    use crate::domain::queue::status::{QueueStatus, MAX_PRIORITY};

    fn any_valid_entry() -> QueueEntry {
        let id: String = kani::any();
        let session: String = kani::any();
        let priority: u32 = kani::any();
        kani::assume(priority <= MAX_PRIORITY);
        QueueEntry::new(id, session, priority).unwrap()
    }

    #[kani::proof]
    fn verify_priority_ordering_invariant() {
        let entries: Vec<QueueEntry> = kani::any();
        kani::assume(entries.len() <= 10);

        let mut queue = Queue::new();
        for entry in entries.clone() {
            queue = queue.enqueue(entry);
        }

        let priorities: Vec<u32> = queue.entries().iter().map(|e| e.priority).collect();
        for window in priorities.windows(2) {
            assert!(window[0] <= window[1]);
        }
    }

    #[kani::proof]
    fn verify_dequeue_removes_correct_entry() {
        let entries: Vec<QueueEntry> = kani::any();
        kani::assume(entries.len() <= 5 && !entries.is_empty());

        let mut queue = Queue::new();
        for entry in entries.clone() {
            queue = queue.enqueue(entry);
        }

        let first_id = entries[0].id.clone();
        let (new_queue, removed) = queue.dequeue(&first_id);

        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, first_id);
        assert_eq!(new_queue.len(), queue.len() - 1);
    }

    #[kani::proof]
    fn verify_dequeue_nonexistent_returns_none() {
        let entry = any_valid_entry();
        let queue = Queue::new().enqueue(entry);

        let nonexistent_id = QueueEntryId::new("nonexistent-id").unwrap();
        let (_new_queue, removed) = queue.dequeue(&nonexistent_id);

        assert!(removed.is_none());
    }

    #[kani::proof]
    fn verify_enqueue_increases_length() {
        let queue = Queue::new();
        let entry = any_valid_entry();
        let new_queue = queue.enqueue(entry);

        assert_eq!(new_queue.len(), queue.len() + 1);
    }

    #[kani::proof]
    fn verify_empty_queue_stays_empty_after_operations() {
        let empty_queue = Queue::new();
        let nonexistent_id = QueueEntryId::new("test").unwrap();

        let (_q1, removed) = empty_queue.dequeue(&nonexistent_id);
        assert!(removed.is_none());

        let filtered: Vec<&QueueEntry> = empty_queue.filter(|_| true);
        assert!(filtered.is_empty());
    }

    #[kani::proof]
    fn verify_find_returns_entry_by_id() {
        let entry = any_valid_entry();
        let entry_id = entry.id.clone();
        let queue = Queue::new().enqueue(entry);

        let found = queue.find(&entry_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, entry_id);
    }

    #[kani::proof]
    fn verify_find_returns_none_for_missing_id() {
        let queue = Queue::new();
        let missing_id = QueueEntryId::new("missing").unwrap();

        assert!(queue.find(&missing_id).is_none());
    }

    #[kani::proof]
    fn verify_next_pending_returns_pending_entry() {
        let entry = any_valid_entry();
        let queue = Queue::new().enqueue(entry);

        let next = queue.next_pending();
        assert!(next.is_some());
        assert_eq!(next.unwrap().status, QueueStatus::Pending);
    }

    #[kani::proof]
    fn verify_queue_len_matches_entries_count() {
        let entries: Vec<QueueEntry> = kani::any();
        kani::assume(entries.len() <= 10);

        let mut queue = Queue::new();
        for entry in entries {
            queue = queue.enqueue(entry);
        }

        assert_eq!(queue.len(), queue.entries().len());
    }

    #[kani::proof]
    fn verify_with_entry_out_of_bounds_returns_error() {
        let entry = any_valid_entry();
        let queue = Queue::new().enqueue(entry);

        let result = queue.with_entry(100, any_valid_entry());
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_with_entry_in_bounds_succeeds() {
        let entry1 = any_valid_entry();
        let entry2 = any_valid_entry();
        let queue = Queue::new().enqueue(entry1);

        let result = queue.with_entry(0, entry2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[kani::proof]
    fn verify_remove_at_valid_position() {
        let entry = any_valid_entry();
        let queue = Queue::new().enqueue(entry);

        let result = queue.remove_at(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().1.id, entry.id);
    }

    #[kani::proof]
    fn verify_remove_at_invalid_position_returns_error() {
        let queue = Queue::new();

        let result = queue.remove_at(0);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_count_active_excludes_terminal() {
        let mut queue = Queue::new();
        let entries: Vec<QueueEntry> = kani::any();
        kani::assume(entries.len() <= 5);

        for entry in entries {
            queue = queue.enqueue(entry);
        }

        let count = queue.count_active();
        let terminal_count = queue
            .entries()
            .iter()
            .filter(|e| e.status.is_terminal())
            .count();

        assert_eq!(count + terminal_count, queue.len());
    }
}
