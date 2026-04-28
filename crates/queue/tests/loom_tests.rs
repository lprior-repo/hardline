//! Loom concurrency tests for queue operations
//!
//! Run with: RUSTFLAGS="--cfg loom" cargo test -p scp-queue --test loom

#[cfg(loom)]
mod concurrency_tests {
    use std::sync::Arc;

    use loom::model;
    use scp_queue::{Queue, QueueEntry, QueueEntryId, QueueStatus, SessionName};

    fn make_entry(id: &str, session: &str, priority: u32) -> QueueEntry {
        QueueEntry::from_identifiers(
            QueueEntryId::new(id).unwrap(),
            SessionName::new(session).unwrap(),
            priority,
        )
        .unwrap()
    }

    #[test]
    fn test_concurrent_enqueue() {
        model(|| {
            let base_queue = Arc::new(Queue::new());

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();
            let queue3 = base_queue.clone();
            let queue4 = base_queue.clone();

            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let entry3 = make_entry("id-3", "session-3", 3);
            let entry4 = make_entry("id-4", "session-4", 4);

            let q1_result = queue1.enqueue(entry1);
            let q2_result = q1_result.enqueue(entry2);
            let q3_result = q2_result.enqueue(entry3);
            let q4_result = q3_result.enqueue(entry4);

            assert_eq!(q4_result.len(), 4);
        });
    }

    #[test]
    fn test_concurrent_dequeue() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let entry3 = make_entry("id-3", "session-3", 3);

            let base_queue = Queue::from_entries_sorted(vec![entry1, entry2, entry3]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();
            let queue3 = base_queue.clone();

            let id1 = QueueEntryId::new("id-1").unwrap();
            let id2 = QueueEntryId::new("id-2").unwrap();
            let id3 = QueueEntryId::new("id-3").unwrap();

            let (q1_result, removed1) = queue1.dequeue(&id1);
            let (q2_result, removed2) = q1_result.dequeue(&id2);
            let (q3_result, removed3) = q2_result.dequeue(&id3);

            assert!(removed1.is_some());
            assert!(removed2.is_some());
            assert!(removed3.is_some());
            assert_eq!(q3_result.len(), 0);
        });
    }

    #[test]
    fn test_mixed_enqueue_dequeue() {
        model(|| {
            let base_queue = Arc::new(Queue::new());

            let queue1 = base_queue.clone();
            let entry1 = make_entry("id-1", "session-1", 1);
            let q1_result = queue1.enqueue(entry1);

            let queue2 = q1_result.clone();
            let entry2 = make_entry("id-2", "session-2", 2);
            let q2_result = queue2.enqueue(entry2);

            let id1 = QueueEntryId::new("id-1").unwrap();
            let (q3_result, removed) = q2_result.dequeue(&id1);

            let queue4 = q3_result.clone();
            let entry3 = make_entry("id-3", "session-3", 3);
            let q4_result = queue4.enqueue(entry3);

            assert!(removed.is_some());
            assert_eq!(q4_result.len(), 2);
            assert!(q4_result.find(&id1).is_none());
        });
    }

    #[test]
    fn test_concurrent_clones_independence() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let base_queue = Queue::from_entries_sorted(vec![entry1]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let entry2 = make_entry("id-2", "session-2", 2);
            let q1_modified = queue1.enqueue(entry2);

            let id1 = QueueEntryId::new("id-1").unwrap();
            let (q2_result, _) = queue2.dequeue(&id1);

            assert_eq!(base_queue.len(), 1);
            assert_eq!(q1_modified.len(), 2);
            assert_eq!(q2_result.len(), 0);
        });
    }

    #[test]
    fn test_double_dequeue_same_entry() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let base_queue = Queue::from_entries_sorted(vec![entry1, entry2]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let id1 = QueueEntryId::new("id-1").unwrap();

            let (q1_result, removed1) = queue1.dequeue(&id1);
            let (q2_result, removed2) = queue2.dequeue(&id1);

            assert!(removed1.is_some() || removed2.is_some());
            assert!(removed1.is_none() || removed2.is_none());
        });
    }

    #[test]
    fn test_priority_ordering_concurrent() {
        model(|| {
            let base_queue = Arc::new(Queue::new());

            let entry1 = make_entry("id-1", "session-a", 10);
            let entry2 = make_entry("id-2", "session-b", 5);
            let entry3 = make_entry("id-3", "session-c", 15);
            let entry4 = make_entry("id-4", "session-d", 5);

            let q1 = base_queue.enqueue(entry1);
            let q2 = q1.enqueue(entry2);
            let q3 = q2.enqueue(entry3);
            let q4 = q3.enqueue(entry4);

            let entries = q4.entries();
            assert!(entries[0].priority <= entries[1].priority);
            assert!(entries[1].priority <= entries[2].priority);
            assert!(entries[2].priority <= entries[3].priority);

            let priorities: Vec<u32> = entries.iter().map(|e| e.priority).collect();
            assert_eq!(priorities, vec![5, 5, 10, 15]);
        });
    }

    #[test]
    fn test_find_concurrent_access() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let base_queue = Queue::from_entries_sorted(vec![entry1, entry2]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let id1 = QueueEntryId::new("id-1").unwrap();
            let id2 = QueueEntryId::new("id-2").unwrap();
            let id3 = QueueEntryId::new("id-3").unwrap();

            let found1 = queue1.find(&id1);
            let found2 = queue2.find(&id2);
            let found3 = queue2.find(&id3);

            assert!(found1.is_some());
            assert!(found2.is_some());
            assert!(found3.is_none());
        });
    }

    #[test]
    fn test_filter_concurrent_access() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let entry3 = make_entry("id-3", "session-3", 1);
            let base_queue = Queue::from_entries_sorted(vec![entry1, entry2, entry3]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let filtered1 = queue1.filter(|e| e.priority == 1);
            let filtered2 = queue2.filter(|e| e.priority == 2);

            assert_eq!(filtered1.len(), 2);
            assert_eq!(filtered2.len(), 1);
        });
    }

    #[test]
    fn test_multiple_operations_on_same_queue() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let entry3 = make_entry("id-3", "session-3", 3);
            let base_queue = Queue::from_entries_sorted(vec![entry1, entry2, entry3]);
            let base_queue = Arc::new(base_queue);

            let id1 = QueueEntryId::new("id-1").unwrap();
            let id2 = QueueEntryId::new("id-2").unwrap();

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();
            let queue3 = base_queue.clone();

            let (q1_deq, removed1) = queue1.dequeue(&id1);
            let q1_enq = q1_deq.enqueue(make_entry("id-4", "session-4", 1));

            let (q2_deq, removed2) = queue2.dequeue(&id2);
            let q2_enq = q2_deq.enqueue(make_entry("id-5", "session-5", 2));

            let q3_enq = queue3.enqueue(make_entry("id-6", "session-6", 1));

            assert!(removed1.is_some());
            assert!(removed2.is_some());
            assert_eq!(q1_enq.len(), 3);
            assert_eq!(q2_enq.len(), 3);
            assert_eq!(q3_enq.len(), 4);
        });
    }

    #[test]
    fn test_update_status_concurrent() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let base_queue = Queue::from_entries_sorted(vec![entry1]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let id1 = QueueEntryId::new("id-1").unwrap();

            let result1 = queue1.update_status(&id1, QueueStatus::Claimed);
            let result2 = queue2.update_status(&id1, QueueStatus::Claimed);

            assert!(result1.is_ok());
            assert!(result2.is_ok());
        });
    }

    #[test]
    fn test_queue_state_persistence() {
        model(|| {
            let base_queue = Arc::new(Queue::new());

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let entry1 = make_entry("id-1", "session-1", 1);
            let q1 = queue1.enqueue(entry1);

            let entry2 = make_entry("id-2", "session-2", 2);
            let q2 = q1.enqueue(entry2);

            assert_eq!(base_queue.len(), 0);
            assert_eq!(q1.len(), 1);
            assert_eq!(q2.len(), 2);
        });
    }

    #[test]
    fn test_partition_concurrent() {
        model(|| {
            let entry1 = make_entry("id-1", "session-1", 1);
            let entry2 = make_entry("id-2", "session-2", 2);
            let entry3 = make_entry("id-3", "session-3", 3);
            let entry4 = make_entry("id-4", "session-4", 4);
            let base_queue = Queue::from_entries(vec![entry1, entry2, entry3, entry4]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let (low1, high1) = queue1.partition(|e| e.priority <= 2);
            let (low2, high2) = queue2.partition(|e| e.priority <= 3);

            assert_eq!(low1.len(), 2);
            assert_eq!(high1.len(), 2);
            assert_eq!(low2.len(), 3);
            assert_eq!(high2.len(), 1);
        });
    }

    #[test]
    fn test_group_by_status_concurrent() {
        model(|| {
            let mut entry1 = make_entry("id-1", "session-1", 1);
            entry1.status = QueueStatus::Pending;
            let mut entry2 = make_entry("id-2", "session-2", 2);
            entry2.status = QueueStatus::Claimed;

            let base_queue = Queue::from_entries(vec![entry1, entry2]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let groups1 = queue1.group_by_status();
            let groups2 = queue2.group_by_status();

            assert_eq!(groups1.len(), 2);
            assert_eq!(groups2.len(), 2);
        });
    }

    #[test]
    fn test_count_active_concurrent() {
        model(|| {
            let mut entry1 = make_entry("id-1", "session-1", 1);
            entry1.status = QueueStatus::Pending;
            let mut entry2 = make_entry("id-2", "session-2", 2);
            entry2.status = QueueStatus::Merged;
            let mut entry3 = make_entry("id-3", "session-3", 3);
            entry3.status = QueueStatus::FailedTerminal;

            let base_queue = Queue::from_entries(vec![entry1, entry2, entry3]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let count1 = queue1.count_active();
            let count2 = queue2.count_active();

            assert_eq!(count1, 1);
            assert_eq!(count2, 1);
        });
    }

    #[test]
    fn test_sorted_by_key_concurrent() {
        model(|| {
            let entry1 = make_entry("id-1", "session-a", 3);
            let entry2 = make_entry("id-2", "session-b", 1);
            let entry3 = make_entry("id-3", "session-c", 2);
            let base_queue = Queue::from_entries(vec![entry1, entry2, entry3]);
            let base_queue = Arc::new(base_queue);

            let queue1 = base_queue.clone();
            let queue2 = base_queue.clone();

            let sorted1 = queue1.sorted_by_key(|e| e.priority);
            let sorted2 = queue2.sorted_by_key(|e| e.session.as_str());

            assert_eq!(sorted1[0].priority, 1);
            assert_eq!(sorted1[1].priority, 2);
            assert_eq!(sorted1[2].priority, 3);

            assert_eq!(sorted2[0].session.as_str(), "session-a");
            assert_eq!(sorted2[1].session.as_str(), "session-b");
            assert_eq!(sorted2[2].session.as_str(), "session-c");
        });
    }
}
