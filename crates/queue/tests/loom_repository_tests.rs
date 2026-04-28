//! Loom concurrency tests for InMemoryQueueRepository
//!
//! Tests the `Arc<Mutex<VecDeque<T>>>` concurrency pattern used by
//! `InMemoryQueueRepository` under all possible thread interleavings
//! explored by loom.
//!
//! Run with: RUSTFLAGS="--cfg loom" cargo test -p scp-queue --test loom_repository

#[cfg(loom)]
mod tests {
    use std::collections::VecDeque;

    use loom::{
        model,
        sync::{Arc, Mutex},
        thread,
    };

    /// Mirrors InMemoryQueueRepository's internal structure using loom primitives.
    /// Uses `u32` values instead of `QueueEntry` to avoid complex type dependencies
    /// in the loom model — the concurrency properties of `Arc<Mutex<VecDeque<T>>>`
    /// are identical regardless of `T`.
    struct LoomQueue {
        entries: Arc<Mutex<VecDeque<u32>>>,
    }

    impl LoomQueue {
        fn new() -> Self {
            Self {
                entries: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        fn enqueue(&self, val: u32) {
            self.entries.lock().unwrap().push_back(val);
        }

        fn dequeue(&self) -> Option<u32> {
            self.entries.lock().unwrap().pop_front()
        }

        fn get(&self, idx: usize) -> Option<u32> {
            self.entries.lock().unwrap().get(idx).copied()
        }

        fn len(&self) -> usize {
            self.entries.lock().unwrap().len()
        }

        /// Mirrors `InMemoryQueueRepository::Clone` — locks, clones inner state,
        /// wraps in new Arc<Mutex>.
        fn clone_snapshot(&self) -> VecDeque<u32> {
            self.entries.lock().unwrap().clone()
        }
    }

    /// Multiple threads enqueue concurrently. Verifies no entries are lost
    /// under any interleaving — the Mutex must serialize all push_back operations.
    #[test]
    fn test_concurrent_enqueue_no_data_loss() {
        model(|| {
            let queue = Arc::new(LoomQueue::new());

            let q1 = queue.clone();
            let q2 = queue.clone();
            let q3 = queue.clone();

            let t1 = thread::spawn(move || q1.enqueue(1));
            let t2 = thread::spawn(move || q2.enqueue(2));
            let t3 = thread::spawn(move || q3.enqueue(3));

            t1.join().unwrap();
            t2.join().unwrap();
            t3.join().unwrap();

            assert_eq!(queue.len(), 3);

            let mut values: Vec<_> = queue.clone_snapshot().into_iter().collect();
            values.sort();
            assert_eq!(values, vec![1, 2, 3]);
        });
    }

    /// Two threads race to dequeue from a pre-populated queue.
    /// Under Mutex serialization, both threads must succeed — no duplicate
    /// pops and no lost items.
    #[test]
    fn test_concurrent_dequeue_both_succeed() {
        model(|| {
            let queue = Arc::new(LoomQueue::new());
            queue.enqueue(10);
            queue.enqueue(20);

            let q1 = queue.clone();
            let q2 = queue.clone();

            let t1 = thread::spawn(move || q1.dequeue());
            let t2 = thread::spawn(move || q2.dequeue());

            let r1 = t1.join().unwrap();
            let r2 = t2.join().unwrap();

            // Both threads get an item (order depends on interleaving)
            assert!(r1.is_some(), "first thread must dequeue an item");
            assert!(r2.is_some(), "second thread must dequeue an item");

            // No duplicates: the two results must be different values
            assert_ne!(r1, r2, "threads must not dequeue the same item");

            // Queue is now empty
            assert_eq!(queue.len(), 0);
        });
    }

    /// One thread enqueues while another reads via `get`. Verifies the reader
    /// always sees a consistent state — never a partial/corrupt VecDeque.
    #[test]
    fn test_enqueue_and_get_consistent_read() {
        model(|| {
            let queue = Arc::new(LoomQueue::new());

            let q1 = queue.clone();
            let q2 = queue.clone();

            let t1 = thread::spawn(move || {
                q1.enqueue(42);
                q1.len()
            });

            let t2 = thread::spawn(move || q2.get(0));

            let len_after_enqueue = t1.join().unwrap();
            let found = t2.join().unwrap();

            // If get observed the item, the queue length must reflect it
            if found.is_some() {
                assert_eq!(len_after_enqueue, 1);
            }

            // Final state is consistent regardless of interleaving
            assert_eq!(queue.len(), 1);
            assert_eq!(queue.get(0), Some(42));
        });
    }

    /// One thread clones (snapshots) the queue while another mutates it.
    /// The snapshot must be a consistent point-in-time copy — never a mix
    /// of pre- and post-mutation state.
    #[test]
    fn test_clone_during_mutation_is_consistent() {
        model(|| {
            let queue = Arc::new(LoomQueue::new());
            queue.enqueue(1);

            let q1 = queue.clone();
            let q2 = queue.clone();

            let t1 = thread::spawn(move || q1.clone_snapshot());
            let t2 = thread::spawn(move || {
                q2.enqueue(2);
                q2.enqueue(3);
            });

            let snapshot = t1.join().unwrap();
            t2.join().unwrap();

            // Snapshot is a consistent point-in-time view:
            // either taken before mutations (len=1) or after (len=3)
            assert!(
                snapshot.len() == 1 || snapshot.len() == 3,
                "snapshot len={}, expected 1 or 3",
                snapshot.len(),
            );

            // Current queue always has all items
            assert_eq!(queue.len(), 3);
        });
    }
}
