use queue::domain::queue::{Queue, QueueEntry, QueueEntryId};
use scp_core::domain::identifiers::SessionName;
use scp_error::Error;

#[test]
fn test_priority_boundary() {
    let id = "id-1";
    let session = "session-1";

    // min priority
    let min = 0;
    assert!(QueueEntry::new(id, session, min).is_ok());

    // max priority
    let max = 100;
    assert!(QueueEntry::new(id, session, max).is_ok());

    // max + 1
    let max_plus_1 = 101;
    let res = QueueEntry::new(id, session, max_plus_1);
    if res.is_ok() {
        println!("BUG: QueueEntry accepts priority > MAX_PRIORITY");
    }
    assert!(res.is_err());

    // negative priority: tested in adversarial tests and wraps around due to u32, so handled.
}

#[test]
fn test_collection_boundary() {
    // Empty collection
    let queue = Queue::new();
    assert_eq!(queue.len(), 0);

    // One item
    let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
    let q1 = queue.enqueue(entry);
    assert_eq!(q1.len(), 1);

    // Many items
    let mut q_many = Queue::new();
    for i in 0..10 {
        let id = format!("id-{}", i);
        let session = format!("session-{}", i);
        let entry = QueueEntry::new(&id, &session, 10).unwrap();
        q_many = q_many.enqueue(entry);
    }
    assert_eq!(q_many.len(), 10);

    // Overflow collection? Is there a MAX_QUEUE_SIZE?
    // Let's see if we can trigger QueueFull.
    // If it's a domain struct `Queue` maybe it has no limit natively, but the use cases enforce it?
    // We will just print we didn't find a limit if we can push 1000 items.
    let mut q_large = Queue::new();
    for i in 0..2000 {
        let id = format!("id-{}", i);
        let session = format!("session-{}", i);
        let entry = QueueEntry::new(&id, &session, 10).unwrap();
        q_large = q_large.enqueue(entry);
    }
    println!("Queue can hold at least {} items", q_large.len());
}
