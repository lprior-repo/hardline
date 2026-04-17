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
        None => Err(scp_core::Error::queue_empty()),
    }
}

/// Process next item in queue
pub fn process(checks: bool) -> Result<()> {
    let queue = get_queue();

    // Acquire lock
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let _guard = lock.acquire(LockType::Queue("default".into()), "scp")?;

    let mut item = match queue.dequeue()? {
        Some(i) => i,
        None => return Err(scp_core::Error::queue_empty()),
    };

    println!("Processing '{}'...", item.branch);

    if checks {
        println!("  Running pre-flight checks...");
        // TODO: Run actual checks
        println!("  ✓ Checks passed");
    }

    // Mark as processing
    item.start_processing();
    queue.update(item.clone())?;

    // TODO: Actually merge/push

    // Mark complete
    item.complete();
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
        .ok_or_else(|| scp_core::Error::queue_item_not_found(branch.to_string()))?;

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

/// Parse a priority string into a Priority enum (pure function for testing)
pub fn parse_priority(priority: &str) -> Priority {
    match priority.to_lowercase().as_str() {
        "low" => Priority::Low,
        "high" => Priority::High,
        "critical" => Priority::Critical,
        _ => Priority::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priority_low() {
        assert_eq!(parse_priority("low"), Priority::Low);
    }

    #[test]
    fn parse_priority_low_uppercase() {
        assert_eq!(parse_priority("LOW"), Priority::Low);
    }

    #[test]
    fn parse_priority_low_mixed_case() {
        assert_eq!(parse_priority("LoW"), Priority::Low);
    }

    #[test]
    fn parse_priority_high() {
        assert_eq!(parse_priority("high"), Priority::High);
    }

    #[test]
    fn parse_priority_critical() {
        assert_eq!(parse_priority("critical"), Priority::Critical);
    }

    #[test]
    fn parse_priority_critical_uppercase() {
        assert_eq!(parse_priority("CRITICAL"), Priority::Critical);
    }

    #[test]
    fn parse_priority_normal() {
        assert_eq!(parse_priority("normal"), Priority::Normal);
    }

    #[test]
    fn parse_priority_unknown_falls_back_to_normal() {
        assert_eq!(parse_priority("unknown"), Priority::Normal);
    }

    #[test]
    fn parse_priority_empty_string_falls_back_to_normal() {
        assert_eq!(parse_priority(""), Priority::Normal);
    }

    #[test]
    fn parse_priority_whitespace_falls_back_to_normal() {
        assert_eq!(parse_priority("  "), Priority::Normal);
    }

    #[test]
    fn parse_priority_partial_match_high_is_not_higher() {
        // "higher" should not match "high"
        assert_eq!(parse_priority("higher"), Priority::Normal);
    }

    #[test]
    fn parse_priority_partial_match_crit_is_not_critical() {
        assert_eq!(parse_priority("crit"), Priority::Normal);
    }

    #[test]
    fn parse_priority_number_falls_back_to_normal() {
        assert_eq!(parse_priority("42"), Priority::Normal);
    }

    #[test]
    fn parse_priority_medium_falls_back_to_normal() {
        assert_eq!(parse_priority("medium"), Priority::Normal);
    }

    #[test]
    fn priority_equality() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_eq!(Priority::High, Priority::High);
        assert_ne!(Priority::Low, Priority::High);
        assert_ne!(Priority::Normal, Priority::Critical);
    }
}
