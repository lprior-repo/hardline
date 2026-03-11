#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

//! Beads issue tracking integration.
//!
//! This module provides functionality for working with the beads issue tracker,
//! including querying issues, filtering, sorting, and analysis.
//!
//! # Architecture
//!
//! The module is organized following Domain-Driven Design principles:
//!
//! - **[`domain`]**: Core domain types with validation (newtypes, enums)
//! - **[`issue`]**: Aggregate root with business logic
//! - **[`types`]**: Legacy types for backward compatibility
//! - **[`db`]**: Database operations (imperative shell)
//! - **[`query`]**: Query and filtering logic
//! - **[`analysis`]**: Analytical operations on issue collections
//!
//! # Migration Path
//!
//! New code should use:
//! - `Issue` from `issue` module instead of `BeadIssue`
//! - `IssueState` from `domain` module instead of `IssueStatus` + `closed_at`
//! - Semantic newtypes (`IssueId`, `Title`, `Description`) instead of `String`

mod analysis;
mod db;
mod domain;
mod issue;
mod query;
mod types;

#[cfg(test)]
mod db_tests;
#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod state_transition_tests;

// Re-export public API

// Domain types (DDD refactored)
// Analysis operations
pub use analysis::{
    all_match, any_match, calculate_critical_path, count_by_status, extract_labels, find_blocked,
    find_blockers, find_potential_duplicates, find_ready, find_stale, get_dependency_graph,
    get_issue, get_issues_by_id, group_by_status, group_by_type, summarize, to_ids, to_titles,
};
// Database operations
pub use db::{delete_bead, ensure_schema, insert_bead, query_beads, update_bead};
pub use domain::{
    Assignee, BlockedBy, DependsOn, Description, DomainError, IssueId, IssueState, IssueType,
    Labels, ParentId, Priority, Title,
};
// Issue aggregate root
pub use issue::{Issue, IssueBuilder};
// Query operations
pub use query::{apply_query, filter_issues, paginate, sort_issues};
// Legacy types (for backward compatibility)
pub use types::{
    BeadFilter, BeadIssue, BeadQuery, BeadSort, BeadsError, BeadsSummary, IssueStatus,
    SortDirection,
};

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_bead_issue_is_blocked() {
        let blocked = BeadIssue {
            id: "test".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::Blocked,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: Some(vec!["other".to_string()]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let unblocked = BeadIssue {
            id: "test2".to_string(),
            title: "Test2".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        assert!(blocked.is_blocked());
        assert!(!unblocked.is_blocked());
    }

    #[test]
    fn test_bead_issue_is_open() {
        let open = BeadIssue {
            id: "test".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let in_progress = BeadIssue {
            id: "test2".to_string(),
            title: "Test2".to_string(),
            status: IssueStatus::InProgress,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let closed = BeadIssue {
            id: "test3".to_string(),
            title: "Test3".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: Some(Utc::now()),
        };

        assert!(open.is_open());
        assert!(in_progress.is_open());
        assert!(!closed.is_open());
    }

    #[test]
    fn test_priority_conversion() {
        assert_eq!(Priority::from_u32(0), Some(Priority::P0));
        assert_eq!(Priority::from_u32(4), Some(Priority::P4));
        assert_eq!(Priority::from_u32(5), None);

        assert_eq!(Priority::P0.to_u32(), 0);
        assert_eq!(Priority::P4.to_u32(), 4);
    }

    #[test]
    fn test_bead_filter_builder() {
        let filter = BeadFilter::new()
            .with_status(IssueStatus::Open)
            .with_label("bug")
            .with_assignee("test")
            .blocked_only()
            .limit(10);

        assert!(filter.status.contains(&IssueStatus::Open));
        assert!(filter.labels.contains(&"bug".to_string()));
        assert_eq!(filter.assignee, Some("test".to_string()));
        assert!(filter.blocked_only);
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_beads_summary() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let summary = BeadsSummary::from_issues(&issues);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.open, 1);
        assert_eq!(summary.closed, 1);
        assert_eq!(summary.active(), 1);
        assert!(!summary.has_blockers());
    }
}
