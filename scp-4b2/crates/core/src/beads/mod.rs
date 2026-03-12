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

    fn create_dummy_issue(status: IssueStatus, blocked_by: Option<Vec<String>>) -> BeadIssue {
        BeadIssue {
            id: "test".to_string(),
            title: "Test".to_string(),
            status,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: if status == IssueStatus::Closed {
                Some(Utc::now())
            } else {
                None
            },
        }
    }

    #[test]
    fn given_blocked_issue_when_is_blocked_called_then_returns_true() {
        let issue = create_dummy_issue(IssueStatus::Blocked, Some(vec!["other".to_string()]));
        assert!(issue.is_blocked());
    }

    #[test]
    fn given_unblocked_issue_when_is_blocked_called_then_returns_false() {
        let issue = create_dummy_issue(IssueStatus::Open, None);
        assert!(!issue.is_blocked());
    }

    #[test]
    fn given_open_issue_when_is_open_called_then_returns_true() {
        let issue = create_dummy_issue(IssueStatus::Open, None);
        assert!(issue.is_open());
    }

    #[test]
    fn given_in_progress_issue_when_is_open_called_then_returns_true() {
        let issue = create_dummy_issue(IssueStatus::InProgress, None);
        assert!(issue.is_open());
    }

    #[test]
    fn given_closed_issue_when_is_open_called_then_returns_false() {
        let issue = create_dummy_issue(IssueStatus::Closed, None);
        assert!(!issue.is_open());
    }

    #[test]
    fn given_valid_u32_zero_when_from_u32_then_returns_p0() {
        assert_eq!(Priority::from_u32(0), Some(Priority::P0));
    }

    #[test]
    fn given_valid_u32_four_when_from_u32_then_returns_p4() {
        assert_eq!(Priority::from_u32(4), Some(Priority::P4));
    }

    #[test]
    fn given_invalid_u32_five_when_from_u32_then_returns_none() {
        assert_eq!(Priority::from_u32(5), None);
    }

    #[test]
    fn given_priority_p0_when_to_u32_then_returns_zero() {
        assert_eq!(Priority::P0.to_u32(), 0);
    }

    #[test]
    fn given_priority_p4_when_to_u32_then_returns_four() {
        assert_eq!(Priority::P4.to_u32(), 4);
    }

    #[test]
    fn given_new_filter_when_with_status_called_then_status_is_set() {
        let filter = BeadFilter::new().with_status(IssueStatus::Open);
        assert!(filter.status.contains(&IssueStatus::Open));
    }

    #[test]
    fn given_new_filter_when_with_label_called_then_label_is_set() {
        let filter = BeadFilter::new().with_label("bug");
        assert!(filter.labels.contains(&"bug".to_string()));
    }

    #[test]
    fn given_new_filter_when_with_assignee_called_then_assignee_is_set() {
        let filter = BeadFilter::new().with_assignee("test");
        assert_eq!(filter.assignee, Some("test".to_string()));
    }

    #[test]
    fn given_new_filter_when_blocked_only_called_then_blocked_only_is_true() {
        let filter = BeadFilter::new().blocked_only();
        assert!(filter.blocked_only);
    }

    #[test]
    fn given_new_filter_when_limit_called_then_limit_is_set() {
        let filter = BeadFilter::new().limit(10);
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn given_mixed_issues_when_summary_from_issues_then_total_is_correct() {
        let issues = vec![
            create_dummy_issue(IssueStatus::Open, None),
            create_dummy_issue(IssueStatus::Closed, None),
        ];
        let summary = BeadsSummary::from_issues(&issues);
        assert_eq!(summary.total, 2);
    }

    #[test]
    fn given_mixed_issues_when_summary_from_issues_then_open_is_correct() {
        let issues = vec![
            create_dummy_issue(IssueStatus::Open, None),
            create_dummy_issue(IssueStatus::Closed, None),
        ];
        let summary = BeadsSummary::from_issues(&issues);
        assert_eq!(summary.open, 1);
    }

    #[test]
    fn given_mixed_issues_when_summary_from_issues_then_closed_is_correct() {
        let issues = vec![
            create_dummy_issue(IssueStatus::Open, None),
            create_dummy_issue(IssueStatus::Closed, None),
        ];
        let summary = BeadsSummary::from_issues(&issues);
        assert_eq!(summary.closed, 1);
    }

    #[test]
    fn given_mixed_issues_when_summary_from_issues_then_active_is_correct() {
        let issues = vec![
            create_dummy_issue(IssueStatus::Open, None),
            create_dummy_issue(IssueStatus::Closed, None),
        ];
        let summary = BeadsSummary::from_issues(&issues);
        assert_eq!(summary.active(), 1);
    }

    #[test]
    fn given_mixed_issues_when_summary_from_issues_then_has_blockers_is_false() {
        let issues = vec![
            create_dummy_issue(IssueStatus::Open, None),
            create_dummy_issue(IssueStatus::Closed, None),
        ];
        let summary = BeadsSummary::from_issues(&issues);
        assert!(!summary.has_blockers());
    }
}
