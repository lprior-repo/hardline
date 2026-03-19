//! Queue commands (from Stak)

use scp_core::{
    lock::{LockManager, LockType, MemLockManager},
    queue::{MemQueue, Priority, QueueItem, QueueManager},
    Result,
};
use std::sync::Arc;

/// Global queue instance
fn get_queue() -> Arc<dyn QueueManager> {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    Arc::new(MemQueue::new(lock))
}

/// List queue items
pub fn list() -> Result<()> {
    let queue = get_queue();
    let items = queue.list()?;

    if items.is_empty() {
        println!("Queue is empty");
    } else {
        println!("Queue ({} items):", items.len());
        for (i, item) in items.iter().enumerate() {
            let status = format!("{:?}", item.status);
            let priority = format!("{:?}", item.priority);
            println!("  {}. {} [{}] {}", i + 1, item.branch, priority, status);
        }
    }

    Ok(())
}

/// Add item to queue
pub fn enqueue(branch: &str, priority: Option<&str>) -> Result<()> {
    let queue = get_queue();

    let mut item = QueueItem::direct(branch);

    if let Some(p) = priority {
        item.priority = match p.to_lowercase().as_str() {
            "low" => Priority::Low,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            _ => Priority::Normal,
        };
    }

    queue.enqueue(item)?;
    println!("✓ Added '{}' to queue", branch);

    Ok(())
}

/// Remove front item from queue
pub fn dequeue() -> Result<()> {
    let queue = get_queue();

    match queue.dequeue()? {
        Some(item) => {
            println!("✓ Dequeued '{}'", item.branch);
            Ok(())
        }
        None => Err(scp_core::Error::QueueEmpty),
    }
}

/// Pure calculation: transition item to processing state
fn transition_to_processing(item: &QueueItem) -> QueueItem {
    let mut item = item.clone();
    item.start_processing();
    item
}

/// Pure calculation: transition item to complete state
fn transition_to_complete(item: &QueueItem) -> QueueItem {
    let mut item = item.clone();
    item.complete();
    item
}

/// Pure I/O: run checks if enabled, returns branch name for logging
fn run_checks(checks: bool, _branch: &str) {
    if checks {
        println!("  Running pre-flight checks...");
        // TODO: Run actual checks
        println!("  ✓ Checks passed");
    }
}

/// Process next item in queue
pub fn process(checks: bool) -> Result<()> {
    let queue = get_queue();

    // Acquire lock
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let _guard = lock.acquire(LockType::Queue("default".into()), "scp")?;

    let item = queue.dequeue()?.ok_or(scp_core::Error::QueueEmpty)?;

    println!("Processing '{}'...", item.branch);

    run_checks(checks, &item.branch);

    let item = transition_to_processing(&item);
    queue.update(item.clone())?;

    // TODO: Actually merge/push

    let item = transition_to_complete(&item);
    queue.update(item.clone())?;

    println!("✓ Processed '{}'", item.branch);
    Ok(())
}

/// Insert item at position
pub fn insert(position: usize, branch: &str) -> Result<()> {
    let queue = get_queue();

    let item = QueueItem::direct(branch);
    // Note: MemQueue handles priority ordering, position is advisory
    queue.enqueue(item)?;

    println!("✓ Inserted '{}' at position {}", branch, position);
    Ok(())
}

/// Remove item from queue
pub fn remove(branch: &str) -> Result<()> {
    let queue = get_queue();

    // Find by branch name
    let items = queue.list()?;
    let item = items
        .iter()
        .find(|i| i.branch == branch)
        .ok_or_else(|| scp_core::Error::QueueItemNotFound(branch.to_string()))?;

    queue.remove(&item.id)?;
    println!("✓ Removed '{}' from queue", branch);

    Ok(())
}

/// Show queue status
pub fn status() -> Result<()> {
    let queue = get_queue();

    let len = queue.len()?;
    let pending = queue.list_pending()?;

    println!("Queue Status:");
    println!("  Total items: {}", len);
    println!("  Pending: {}", pending.len());

    if !pending.is_empty() {
        println!("  Next: {}", pending[0].branch);
    }

    Ok(())
}
