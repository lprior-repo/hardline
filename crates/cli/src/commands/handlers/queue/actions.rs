//! Actions for the queue command handler.
//!
//! This module contains I/O operations that call the core queue functions.
//! All functions return Result and handle errors appropriately.

use crate::commands::handlers::queue::calculations::{
    build_item_detail, build_queue_status, build_queue_table_human, filter_by_status,
    format_item_detail, format_queue_status, parse_priority_to_string, parse_status_to_string,
    sort_by_priority,
};
use crate::commands::handlers::queue::data::{
    QueueItemDetail, QueueItemDisplay, QueueListItem, QueueOptions, QueueOutputFormat,
    QueueStatusDisplay, QueueSubcommand,
};
use scp_core::{
    lock::{LockManager, LockType, MemLockManager},
    queue::{MemQueue, Priority, QueueItem, QueueManager},
    Result,
};
use std::sync::Arc;

/// Global queue instance (for CLI operations)
fn get_queue() -> Arc<dyn QueueManager> {
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    Arc::new(MemQueue::new(lock))
}

/// Run the list subcommand
pub fn run_list(opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();
    let items = queue.list()?;

    if items.is_empty() {
        println!("Queue is empty");
        return Ok(());
    }

    // Convert to list items and sort by priority
    let list_items: Vec<QueueListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| QueueListItem {
            index: i + 1,
            branch: item.branch.clone(),
            priority: priority_to_string(item.priority),
            status: status_to_string(item.status),
        })
        .collect();

    let sorted = sort_by_priority(list_items);

    // Output based on format
    match opts.format {
        QueueOutputFormat::Table => {
            // Convert back to display items
            let display_items: Vec<QueueItemDisplay> = sorted
                .iter()
                .map(|item| QueueItemDisplay {
                    index: item.index,
                    branch: item.branch.clone(),
                    priority: item.priority.clone(),
                    status: item.status.clone(),
                })
                .collect();
            let table = build_queue_table_human(&display_items);
            println!("{}", table);
        }
        QueueOutputFormat::Json => {
            let json = serde_json::to_string_pretty(&sorted).map_err(|e| {
                scp_core::Error::io_error(format!("JSON serialization failed: {}", e))
            })?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Run the enqueue subcommand
pub fn run_enqueue(opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();

    if let QueueSubcommand::Enqueue { branch, priority } = &opts.subcommand {
        let mut item = QueueItem::direct(branch);

        if let Some(p) = priority {
            item.priority = parse_priority(p);
        }

        queue.enqueue(item)?;
        println!("✓ Added '{}' to queue", branch);
    } else {
        return Err(scp_core::Error::internal(
            "invalid subcommand for enqueue".to_string(),
        ));
    }

    Ok(())
}

/// Run the dequeue subcommand
pub fn run_dequeue(_opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();

    match queue.dequeue()? {
        Some(item) => {
            println!("✓ Dequeued '{}'", item.branch);
            Ok(())
        }
        None => Err(scp_core::Error::queue_empty()),
    }
}

/// Run the status subcommand
pub fn run_status(_opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();

    let total = queue.len()?;
    let pending_items = queue.list_pending()?;

    let next_item = pending_items.first().map(|item| item.branch.as_str());
    let status = build_queue_status(total, pending_items.len(), next_item);

    let output = format_queue_status(&status);
    println!("{}", output);

    Ok(())
}

/// Run the clear subcommand
pub fn run_clear(_opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();

    let removed = queue.clear_completed()?;
    println!("✓ Cleared {} completed/failed items", removed);

    Ok(())
}

/// Run the detail subcommand
pub fn run_detail(opts: &QueueOptions) -> Result<()> {
    let queue = get_queue();

    if let QueueSubcommand::Detail { target } = &opts.subcommand {
        // Try to find by branch name first
        let items = queue.list()?;
        let item = items
            .iter()
            .find(|i| i.branch == *target || i.id == *target)
            .ok_or_else(|| scp_core::Error::queue_item_not_found(target.clone()))?;

        let detail = build_item_detail(
            &QueueItemDisplay {
                index: 0,
                branch: item.branch.clone(),
                priority: priority_to_string(item.priority),
                status: status_to_string(item.status),
            },
            &item.id,
            &source_to_string(&item.source),
            item.attempt_count,
            item.last_error.as_deref(),
            &item.created_at.to_rfc3339(),
            &item.updated_at.to_rfc3339(),
        );

        let output = format_item_detail(&detail);
        println!("{}", output);
    } else {
        return Err(scp_core::Error::internal(
            "invalid subcommand for detail".to_string(),
        ));
    }

    Ok(())
}

/// Convert Priority enum to string
#[must_use]
fn priority_to_string(priority: Priority) -> String {
    match priority {
        Priority::Critical => "Critical".to_string(),
        Priority::High => "High".to_string(),
        Priority::Normal => "Normal".to_string(),
        Priority::Low => "Low".to_string(),
    }
}

/// Convert QueueStatus to string
#[must_use]
fn status_to_string(status: scp_core::queue::QueueStatus) -> String {
    match status {
        scp_core::queue::QueueStatus::Pending => "Pending".to_string(),
        scp_core::queue::QueueStatus::Processing => "Processing".to_string(),
        scp_core::queue::QueueStatus::Completed => "Completed".to_string(),
        scp_core::queue::QueueStatus::Failed => "Failed".to_string(),
        scp_core::queue::QueueStatus::Cancelled => "Cancelled".to_string(),
    }
}

/// Convert QueueSource to string
#[must_use]
fn source_to_string(source: &scp_core::queue::QueueSource) -> String {
    match source {
        scp_core::queue::QueueSource::Direct => "Direct".to_string(),
        scp_core::queue::QueueSource::Workspace(name) => format!("Workspace({})", name),
    }
}

/// Parse priority string to Priority enum
#[must_use]
fn parse_priority(priority: &str) -> Priority {
    match priority.to_lowercase().as_str() {
        "low" => Priority::Low,
        "high" => Priority::High,
        "critical" => Priority::Critical,
        _ => Priority::Normal,
    }
}
