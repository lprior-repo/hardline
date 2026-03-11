use scp_queue::{Queue, QueueEntry, QueueEntryId};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_adversarial_max_priority_overflow() {
    let res = QueueEntry::new("id-1", "session-1", u32::MAX);
    assert!(res.is_err(), "Expected error on max priority overflow");
}

#[test]
fn test_adversarial_negative_priority() {
    let priority = -1i32 as u32; // Underflows to u32::MAX
    let res = QueueEntry::new("id-2", "session-2", priority);
    assert!(res.is_err(), "Expected error on negative priority");
}

#[test]
fn test_adversarial_duplicate_entries() {
    let mut queue = Queue::new();
    let entry1 = QueueEntry::new("id-dup", "session-dup", 10).unwrap();
    let entry2 = QueueEntry::new("id-dup", "session-dup", 20).unwrap();

    queue = queue.enqueue(entry1);
    queue = queue.enqueue(entry2);

    // This asserts that the queue currently DOES allow duplicate IDs!
    assert_eq!(
        queue.len(),
        2,
        "Queue allowed duplicate entries with the same ID!"
    );
}

#[test]
fn test_adversarial_concurrent_race_condition_lost_updates() {
    let shared_queue = Arc::new(Mutex::new(Queue::new()));
    let mut handles = vec![];

    for i in 0..1000 {
        let q = Arc::clone(&shared_queue);
        handles.push(thread::spawn(move || {
            // Anti-pattern: Read, mutate, write back (Lost Update race condition)
            let current = q.lock().unwrap().clone();

            // Introduce a small artificial delay to ensure the race window is hit
            thread::yield_now();

            let entry =
                QueueEntry::new(format!("id-{}", i), format!("sess-{}", i), (i % 100) as u32)
                    .unwrap();
            let next = current.enqueue(entry);

            *q.lock().unwrap() = next;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_len = shared_queue.lock().unwrap().len();
    println!(
        "Expected 1000 items, got {} due to race conditions.",
        final_len
    );
    assert!(
        final_len < 1000,
        "Expected lost updates due to race conditions"
    );
}

#[test]
fn test_adversarial_concurrent_enqueue_dequeue() {
    let shared_queue = Arc::new(Mutex::new(Queue::new()));

    // Pre-populate with some data
    {
        let mut q = shared_queue.lock().unwrap();
        for i in 0..100 {
            let entry = QueueEntry::new(format!("initial-{}", i), "sess", 50).unwrap();
            *q = q.enqueue(entry);
        }
    }

    let mut handles = vec![];

    // Spawn enqueuers
    for i in 0..50 {
        let q = Arc::clone(&shared_queue);
        handles.push(thread::spawn(move || {
            let current = q.lock().unwrap().clone();
            thread::yield_now();
            let entry = QueueEntry::new(format!("enq-{}", i), "sess", 50).unwrap();
            *q.lock().unwrap() = current.enqueue(entry);
        }));
    }

    // Spawn dequeuers
    for i in 0..50 {
        let q = Arc::clone(&shared_queue);
        handles.push(thread::spawn(move || {
            let current = q.lock().unwrap().clone();
            thread::yield_now();
            let id = QueueEntryId::new(format!("initial-{}", i)).unwrap();
            let (next, _) = current.dequeue(&id);
            *q.lock().unwrap() = next;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_len = shared_queue.lock().unwrap().len();
    println!(
        "Final length after concurrent enqueue/dequeue: {} (expected 100 if serialized correctly)",
        final_len
    );
    assert!(
        final_len != 100,
        "Expected race conditions to violate invariants"
    );
}
