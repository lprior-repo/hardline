//! Queue command handler for managing branch queue operations.
//!
//! This module provides handlers for queue enqueue/dequeue commands, queue status
//! display, priority display formatting, queue listing with filters, queue clear,
//! queue item detail, and output formatting (table vs JSON).
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): `QueueItemDisplay`, `QueueStatusDisplay`, `QueueListItem`,
//!   `QueueOutputFormat`, `QueueSubcommand`, `QueueOptions` (inert, serializable)
//! - **Calculations** (`calculations.rs`): `format_priority_display`, `format_status_display`,
//!   `sort_by_priority`, `filter_by_status`, `format_item_detail`, `build_queue_table`
//!   (pure functions)
//! - **Actions** (`actions.rs`): `run_list`, `run_enqueue`, `run_dequeue`, `run_status`,
//!   `run_clear`, `run_detail` (I/O boundary: serialization + Output)
//!
//! # Functional Rust Principles
//!
//! - Zero panic, zero unwrap in source code
//! - All functions return `Result<T, E>`
//! - Pure functions in calculations layer
//! - I/O boundary in actions layer
//! - Data types are invariant and serializable

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

mod actions;
mod calculations;
mod data;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mod_tests {
    // =========================================================================
    // Module re-exports: verify all public types are accessible from this module
    // =========================================================================

    #[test]
    fn data_types_are_accessible_via_super() {
        use crate::commands::handlers::queue::data::{
            QueueItemDisplay, QueueListItem, QueueOptions, QueueOutputFormat, QueueStatusDisplay,
            QueueSubcommand,
        };

        let _item = QueueItemDisplay {
            index: 0,
            branch: "test".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        };
        let _status = QueueStatusDisplay {
            total_items: 0,
            pending_items: 0,
            next_item: None,
        };
        let _format = QueueOutputFormat::Table;
        let _subcmd = QueueSubcommand::List;
    }

    #[test]
    fn calculation_functions_are_accessible() {
        use super::data::{QueueItemDetail, QueueItemDisplay, QueueListItem};
        use crate::commands::handlers::queue::calculations::{
            build_queue_table_human, filter_by_status, format_item_detail, format_priority_display,
            format_status_display, sort_by_priority,
        };

        let priority = format_priority_display("low");
        assert!(!priority.is_empty());

        let status = format_status_display("pending");
        assert!(!status.is_empty());

        let mut items: Vec<QueueListItem> = vec![
            QueueListItem {
                index: 1,
                branch: "b".to_string(),
                priority: "Normal".to_string(),
                status: "Pending".to_string(),
            },
            QueueListItem {
                index: 0,
                branch: "a".to_string(),
                priority: "Critical".to_string(),
                status: "Pending".to_string(),
            },
        ];
        let sorted = sort_by_priority(items.clone());
        assert!(sorted.len() == items.len());

        let filtered = filter_by_status(&items, "Pending");
        assert!(filtered.len() <= items.len());

        // Convert to display items for table
        let display_items: Vec<QueueItemDisplay> = sorted
            .iter()
            .map(|item| QueueItemDisplay {
                index: item.index,
                branch: item.branch.clone(),
                priority: item.priority.clone(),
                status: item.status.clone(),
            })
            .collect();

        let detail = QueueItemDetail {
            id: "test".to_string(),
            branch: "test".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
            source: "Direct".to_string(),
            attempt_count: 0,
            last_error: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };
        let detail_str = format_item_detail(&detail);
        assert!(!detail_str.is_empty());

        let table = build_queue_table_human(&display_items);
        assert!(!table.is_empty());
    }

    #[test]
    fn action_functions_are_accessible() {
        use crate::commands::handlers::queue::actions::{
            run_clear, run_dequeue, run_detail, run_enqueue, run_list, run_status,
        };
        use crate::commands::handlers::queue::data::{QueueOptions, QueueSubcommand};

        let opts = QueueOptions {
            subcommand: QueueSubcommand::List,
            format: Default::default(),
        };
        // These functions will use the global queue which may be empty
        let _ = run_list(&opts);
    }
}

// Re-export all public types from submodules
