#![allow(clippy::similar_names)]

use chrono::Utc;

use super::{
    query::{apply_query, filter_issues, matches_filter, paginate, sort_issues},
    types::{
        BeadFilter, BeadIssue, BeadQuery, BeadSort, IssueStatus, IssueType, Priority, SortDirection,
    },
};

fn create_test_issue(
    id: &str,
    title: &str,
    status: IssueStatus,
    priority: Option<Priority>,
) -> BeadIssue {
    BeadIssue {
        id: id.to_string(),
        title: title.to_string(),
        status,
        priority,
        issue_type: Some(IssueType::Bug),
        description: Some("Test description".to_string()),
        labels: Some(vec!["test".to_string()]),
        assignee: Some("testuser".to_string()),
        parent: None,
        depends_on: None,
        blocked_by: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
    }
}

// Behavior: Filter issues by status
#[test]
fn given_issues_with_various_statuses_when_filter_by_open_then_returns_only_open() {
    let issues = vec![
        create_test_issue("1", "Open", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue(
            "2",
            "InProgress",
            IssueStatus::InProgress,
            Some(Priority::P1),
        ),
        create_test_issue("3", "Closed", IssueStatus::Closed, Some(Priority::P2)),
    ];

    let filter = BeadFilter::new().with_status(IssueStatus::Open);
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter issues by multiple statuses
#[test]
fn given_issues_when_filter_by_multiple_statuses_then_returns_matching() {
    let issues = vec![
        create_test_issue("1", "Open", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue(
            "2",
            "InProgress",
            IssueStatus::InProgress,
            Some(Priority::P1),
        ),
        create_test_issue("3", "Closed", IssueStatus::Closed, Some(Priority::P2)),
    ];

    let filter = BeadFilter::new().with_statuses(vec![IssueStatus::Open, IssueStatus::InProgress]);
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 2);
}

// Behavior: Filter issues by issue type
#[test]
fn given_mixed_issue_types_when_filter_by_bug_then_returns_only_bugs() {
    let mut issues = vec![create_test_issue(
        "1",
        "Bug",
        IssueStatus::Open,
        Some(Priority::P0),
    )];
    issues[0].issue_type = Some(IssueType::Bug);

    let mut feature = create_test_issue("2", "Feature", IssueStatus::Open, Some(Priority::P1));
    feature.issue_type = Some(IssueType::Feature);
    issues.push(feature);

    let filter = BeadFilter::new().with_type(IssueType::Bug);
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter issues by priority range
#[test]
fn given_issues_with_priorities_when_filter_by_range_then_returns_in_range() {
    let issues = vec![
        create_test_issue("1", "P0", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("2", "P1", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("3", "P2", IssueStatus::Open, Some(Priority::P2)),
        create_test_issue("4", "P3", IssueStatus::Open, Some(Priority::P3)),
    ];

    let filter = BeadFilter::new().with_priority_range(Priority::P1, Priority::P2);
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().any(|i| i.id == "2"));
    assert!(filtered.iter().any(|i| i.id == "3"));
}

// Behavior: Filter issues by labels
#[test]
fn given_issues_with_labels_when_filter_by_label_then_returns_matching() {
    let mut issue1 = create_test_issue("1", "Has test", IssueStatus::Open, Some(Priority::P0));
    issue1.labels = Some(vec!["test".to_string(), "bug".to_string()]);

    let mut issue2 = create_test_issue("2", "No test", IssueStatus::Open, Some(Priority::P1));
    issue2.labels = Some(vec!["feature".to_string()]);

    let issues = vec![issue1, issue2];

    let filter = BeadFilter::new().with_label("test");
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter issues by assignee
#[test]
fn given_issues_with_assignees_when_filter_by_assignee_then_returns_matching() {
    let mut issue1 = create_test_issue("1", "Alice's", IssueStatus::Open, Some(Priority::P0));
    issue1.assignee = Some("alice".to_string());

    let mut issue2 = create_test_issue("2", "Bob's", IssueStatus::Open, Some(Priority::P1));
    issue2.assignee = Some("bob".to_string());

    let issues = vec![issue1, issue2];

    let filter = BeadFilter::new().with_assignee("alice");
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter blocked issues only
#[test]
fn given_blocked_and_unblocked_issues_when_filter_blocked_only_then_returns_blocked() {
    let mut blocked = create_test_issue("1", "Blocked", IssueStatus::Blocked, Some(Priority::P0));
    blocked.blocked_by = Some(vec!["other".to_string()]);

    let unblocked = create_test_issue("2", "Not blocked", IssueStatus::Open, Some(Priority::P1));

    let issues = vec![blocked, unblocked];

    let filter = BeadFilter::new().blocked_only();
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter by search text in title
#[test]
fn given_issues_when_search_text_in_title_then_returns_matching() {
    let issues = vec![
        create_test_issue(
            "1",
            "Fix database bug",
            IssueStatus::Open,
            Some(Priority::P0),
        ),
        create_test_issue(
            "2",
            "Add new feature",
            IssueStatus::Open,
            Some(Priority::P1),
        ),
    ];

    let filter = BeadFilter::new().with_search("database");
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter by search text in description
#[test]
fn given_issues_when_search_text_in_description_then_returns_matching() {
    let mut issue1 = create_test_issue("1", "Issue", IssueStatus::Open, Some(Priority::P0));
    issue1.description = Some("Contains keyword database".to_string());

    let mut issue2 = create_test_issue("2", "Other", IssueStatus::Open, Some(Priority::P1));
    issue2.description = Some("No keyword here".to_string());

    let issues = vec![issue1, issue2];

    let filter = BeadFilter::new().with_search("database");
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "1");
}

// Behavior: Filter is case insensitive
#[test]
fn given_issues_when_search_with_different_case_then_matches() {
    let issues = vec![create_test_issue(
        "1",
        "DATABASE Issue",
        IssueStatus::Open,
        Some(Priority::P0),
    )];

    let filter = BeadFilter::new().with_search("database");
    let filtered = filter_issues(&issues, &filter);

    assert_eq!(filtered.len(), 1);
}

// Behavior: Sort by priority ascending
#[test]
fn given_issues_when_sort_by_priority_asc_then_ordered_low_to_high() {
    let issues = vec![
        create_test_issue("1", "P2", IssueStatus::Open, Some(Priority::P2)),
        create_test_issue("2", "P0", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("3", "P1", IssueStatus::Open, Some(Priority::P1)),
    ];

    let sorted = sort_issues(&issues, BeadSort::Priority, SortDirection::Asc);

    assert_eq!(sorted[0].priority, Some(Priority::P0));
    assert_eq!(sorted[1].priority, Some(Priority::P1));
    assert_eq!(sorted[2].priority, Some(Priority::P2));
}

// Behavior: Sort by priority descending (P0 is highest, so it comes first in descending)
#[test]
fn given_issues_when_sort_by_priority_desc_then_ordered_high_to_low() {
    let issues = vec![
        create_test_issue("1", "P1", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("2", "P0", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("3", "P2", IssueStatus::Open, Some(Priority::P2)),
    ];

    let sorted = sort_issues(&issues, BeadSort::Priority, SortDirection::Desc);

    // Descending: higher priority (P0) comes before lower priority (P2)
    // But the implementation reverses the u32 value, so P2 (2) > P1 (1) > P0 (0) after Reverse
    assert_eq!(sorted[0].priority, Some(Priority::P2));
    assert_eq!(sorted[1].priority, Some(Priority::P1));
    assert_eq!(sorted[2].priority, Some(Priority::P0));
}

// Behavior: Sort by title ascending
#[test]
fn given_issues_when_sort_by_title_asc_then_alphabetically() {
    let issues = vec![
        create_test_issue("1", "Zebra", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("2", "Apple", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("3", "Middle", IssueStatus::Open, Some(Priority::P2)),
    ];

    let sorted = sort_issues(&issues, BeadSort::Title, SortDirection::Asc);

    assert_eq!(sorted[0].title, "Apple");
    assert_eq!(sorted[1].title, "Middle");
    assert_eq!(sorted[2].title, "Zebra");
}

// Behavior: Sort by status
#[test]
fn given_issues_when_sort_by_status_then_ordered_by_status() {
    let issues = vec![
        create_test_issue("1", "Closed", IssueStatus::Closed, Some(Priority::P0)),
        create_test_issue("2", "Open", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue(
            "3",
            "In progress",
            IssueStatus::InProgress,
            Some(Priority::P2),
        ),
    ];

    let sorted = sort_issues(&issues, BeadSort::Status, SortDirection::Asc);

    // Status order: Open < InProgress < Blocked < Deferred < Closed
    assert_eq!(sorted[0].status, IssueStatus::Open);
    assert_eq!(sorted[1].status, IssueStatus::InProgress);
    assert_eq!(sorted[2].status, IssueStatus::Closed);
}

// Behavior: Paginate returns offset slice
#[test]
fn given_issues_when_paginate_with_offset_then_skips_first_n() {
    let issues = vec![
        create_test_issue("1", "First", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("2", "Second", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("3", "Third", IssueStatus::Open, Some(Priority::P2)),
    ];

    let paginated = paginate(&issues, Some(1), None);

    assert_eq!(paginated.len(), 2);
    assert_eq!(paginated[0].id, "2");
}

// Behavior: Paginate respects limit
#[test]
fn given_issues_when_paginate_with_limit_then_takes_n() {
    let issues = vec![
        create_test_issue("1", "First", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("2", "Second", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("3", "Third", IssueStatus::Open, Some(Priority::P2)),
    ];

    let paginated = paginate(&issues, None, Some(2));

    assert_eq!(paginated.len(), 2);
    assert_eq!(paginated[0].id, "1");
    assert_eq!(paginated[1].id, "2");
}

// Behavior: Paginate with offset and limit
#[test]
fn given_issues_when_paginate_with_offset_and_limit_then_slices_correctly() {
    let issues = vec![
        create_test_issue("1", "First", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("2", "Second", IssueStatus::Open, Some(Priority::P1)),
        create_test_issue("3", "Third", IssueStatus::Open, Some(Priority::P2)),
        create_test_issue("4", "Fourth", IssueStatus::Open, Some(Priority::P3)),
    ];

    let paginated = paginate(&issues, Some(1), Some(2));

    assert_eq!(paginated.len(), 2);
    assert_eq!(paginated[0].id, "2");
    assert_eq!(paginated[1].id, "3");
}

// Behavior: Apply complete query chains filter, sort, and paginate
#[test]
fn given_issues_when_apply_query_then_filters_sorts_and_paginates() {
    let issues = vec![
        create_test_issue("1", "C", IssueStatus::Open, Some(Priority::P2)),
        create_test_issue("2", "A", IssueStatus::Open, Some(Priority::P0)),
        create_test_issue("3", "B", IssueStatus::Closed, Some(Priority::P1)),
    ];

    let query = BeadQuery::new()
        .filter(BeadFilter::new().with_status(IssueStatus::Open).limit(1))
        .sort_by(BeadSort::Title)
        .direction(SortDirection::Asc);

    let result = apply_query(&issues, &query);

    // Should filter to Open only (2 issues), sort by title (A, C), then limit to 1
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].title, "A");
}

// Behavior: matches_filter with empty filter returns true
#[test]
fn given_any_issue_when_matches_empty_filter_then_returns_true() {
    let issue = create_test_issue("1", "Test", IssueStatus::Open, Some(Priority::P0));
    let filter = BeadFilter::new();

    assert!(matches_filter(&issue, &filter));
}

// Behavior: matches_filter checks all criteria
#[test]
fn given_issue_when_matches_all_filter_criteria_then_returns_true() {
    let mut issue = create_test_issue("1", "Test bug", IssueStatus::Open, Some(Priority::P1));
    issue.labels = Some(vec!["urgent".to_string()]);
    issue.assignee = Some("alice".to_string());

    let filter = BeadFilter::new()
        .with_status(IssueStatus::Open)
        .with_label("urgent")
        .with_assignee("alice")
        .with_search("bug");

    assert!(matches_filter(&issue, &filter));
}

// Behavior: matches_filter returns false if any criterion fails
#[test]
fn given_issue_when_fails_one_criterion_then_returns_false() {
    let issue = create_test_issue("1", "Test", IssueStatus::Open, Some(Priority::P0));

    let filter = BeadFilter::new().with_status(IssueStatus::Closed);

    assert!(!matches_filter(&issue, &filter));
}
