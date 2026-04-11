//! Calculations for the queue command handler (Tier 2).
//!
//! Pure functions with no I/O. All business logic lives here.
//! Every function is deterministic and side-effect-free.

use super::data::{
    QueueItemDetail, QueueItemDisplay, QueueListItem, QueueOutputFormat, QueueStatusDisplay,
};

/// Format a priority value for display
#[must_use]
pub fn format_priority_display(priority: &str) -> String {
    priority.to_string()
}

/// Format a status value for display
#[must_use]
pub fn format_status_display(status: &str) -> String {
    status.to_string()
}

/// Sort queue items by priority (Critical > High > Normal > Low)
///
/// # Priority order
///
/// - Critical (0)
/// - High (1)
/// - Normal (2)
/// - Low (3)
pub fn sort_by_priority(items: Vec<QueueListItem>) -> Vec<QueueListItem> {
    let mut sorted = items;
    sorted.sort_by(|a, b| {
        let a_priority = parse_priority_ord(&a.priority);
        let b_priority = parse_priority_ord(&b.priority);
        a_priority.cmp(&b_priority)
    });
    sorted
}

/// Parse priority string to numeric order for sorting
#[must_use]
fn parse_priority_ord(priority: &str) -> u8 {
    match priority.to_lowercase().as_str() {
        "critical" => 0,
        "high" => 1,
        _ => 2,
    }
}

/// Filter queue items by status
#[must_use]
pub fn filter_by_status(items: &[QueueListItem], status: &str) -> Vec<QueueListItem> {
    items
        .iter()
        .filter(|item| item.status.eq_ignore_ascii_case(status))
        .cloned()
        .collect()
}

/// Build a queue table for table format output
#[must_use]
pub fn build_queue_table(items: &[QueueListItem], _format: QueueOutputFormat) -> String {
    if items.is_empty() {
        return "Queue is empty".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("Queue ({} items):", items.len()));

    for item in items {
        lines.push(format!(
            "  {}. {} [{}] {}",
            item.index, item.branch, item.priority, item.status
        ));
    }

    lines.join("\n")
}

/// Build a queue table for human-readable display
#[must_use]
pub fn build_queue_table_human(items: &[QueueItemDisplay]) -> String {
    if items.is_empty() {
        return "Queue is empty".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("Queue ({} items):", items.len()));

    for item in items {
        lines.push(format!(
            "  {}. {} [{}] {}",
            item.index, item.branch, item.priority, item.status
        ));
    }

    lines.join("\n")
}

/// Format a single queue item as a string
#[must_use]
pub fn format_single_item(item: &QueueItemDisplay) -> String {
    format!("{} [{}] {}", item.branch, item.priority, item.status)
}

/// Build queue status display
#[must_use]
pub fn build_queue_status(
    total: usize,
    pending: usize,
    next_item: Option<&str>,
) -> QueueStatusDisplay {
    QueueStatusDisplay {
        total_items: total,
        pending_items: pending,
        next_item: next_item.map(String::from),
    }
}

/// Format queue status for display
#[must_use]
pub fn format_queue_status(status: &QueueStatusDisplay) -> String {
    let mut lines = Vec::new();
    lines.push("Queue Status:".to_string());
    lines.push(format!("  Total items: {}", status.total_items));
    lines.push(format!("  Pending: {}", status.pending_items));

    if let Some(ref next) = status.next_item {
        lines.push(format!("  Next: {next}"));
    }

    lines.join("\n")
}

/// Format queue item detail for display
#[must_use]
pub fn format_item_detail(detail: &QueueItemDetail) -> String {
    let mut lines = Vec::new();
    lines.push("Item Detail:".to_string());
    lines.push(format!("  ID: {}", detail.id));
    lines.push(format!("  Branch: {}", detail.branch));
    lines.push(format!("  Priority: {}", detail.priority));
    lines.push(format!("  Status: {}", detail.status));
    lines.push(format!("  Source: {}", detail.source));
    lines.push(format!("  Attempt Count: {}", detail.attempt_count));

    if let Some(ref error) = detail.last_error {
        lines.push(format!("  Last Error: {error}"));
    } else {
        lines.push("  Last Error: None".to_string());
    }

    lines.push(format!("  Created At: {}", detail.created_at));
    lines.push(format!("  Updated At: {}", detail.updated_at));

    lines.join("\n")
}

/// Parse priority string to string representation
#[must_use]
pub fn parse_priority_to_string(priority: &str) -> String {
    let lower = priority.to_lowercase();
    match lower.as_str() {
        "low" => "Low".to_string(),
        "high" => "High".to_string(),
        "critical" => "Critical".to_string(),
        _ => "Normal".to_string(),
    }
}

/// Parse status string to display format
#[must_use]
pub fn parse_status_to_string(status: &str) -> String {
    let lower = status.to_lowercase();
    match lower.as_str() {
        "pending" => "Pending".to_string(),
        "processing" => "Processing".to_string(),
        "completed" => "Completed".to_string(),
        "failed" => "Failed".to_string(),
        "cancelled" => "Cancelled".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Build item detail from list item and additional info
#[must_use]
pub fn build_item_detail(
    item: &QueueItemDisplay,
    id: &str,
    source: &str,
    attempt_count: u32,
    last_error: Option<&str>,
    created_at: &str,
    updated_at: &str,
) -> QueueItemDetail {
    QueueItemDetail {
        id: id.to_string(),
        branch: item.branch.clone(),
        priority: item.priority.clone(),
        status: item.status.clone(),
        source: source.to_string(),
        attempt_count,
        last_error: last_error.map(String::from),
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
    }
}
