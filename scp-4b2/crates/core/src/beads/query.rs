#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use std::cmp::Reverse;

use itertools::Itertools;
use tap::Pipe;

use super::types::{BeadFilter, BeadIssue, BeadQuery, BeadSort, Priority, SortDirection};

/// Filter issues based on the given filter criteria.
#[must_use]
pub fn filter_issues(issues: &[BeadIssue], filter: &BeadFilter) -> Vec<BeadIssue> {
    issues
        .iter()
        .filter(|issue| matches_filter(issue, filter))
        .cloned()
        .collect()
}

#[allow(clippy::unnecessary_map_or)]
pub fn matches_filter(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    (filter.status.is_empty() || filter.status.contains(&issue.status))
        && (filter.issue_type.is_empty()
            || issue
                .issue_type
                .as_ref()
                .map_or(false, |t| filter.issue_type.contains(t)))
        && (filter
            .priority_min
            .as_ref()
            .map_or(true, |min| issue.priority.map_or(true, |p| p >= *min)))
        && (filter
            .priority_max
            .as_ref()
            .map_or(true, |max| issue.priority.map_or(true, |p| p <= *max)))
        && (filter.labels.is_empty()
            || issue.labels.as_ref().map_or(false, |issue_labels| {
                filter.labels.iter().all(|l| issue_labels.contains(l))
            }))
        && (filter
            .assignee
            .as_ref()
            .map_or(true, |assignee| issue.assignee.as_ref() == Some(assignee)))
        && (filter
            .parent
            .as_ref()
            .map_or(true, |parent| issue.parent.as_ref() == Some(parent)))
        && (!filter.has_parent || issue.parent.is_some())
        && (!filter.blocked_only || issue.is_blocked())
        && filter.search_text.as_ref().map_or(true, |text| {
            let text_lower = text.to_lowercase();
            issue.title.to_lowercase().contains(&text_lower)
                || issue
                    .description
                    .as_ref()
                    .map_or(false, |d| d.to_lowercase().contains(&text_lower))
        })
}

/// Sort issues based on the given sort field and direction.
#[must_use]
pub fn sort_issues(
    issues: &[BeadIssue],
    sort: BeadSort,
    direction: SortDirection,
) -> Vec<BeadIssue> {
    match sort {
        BeadSort::Priority => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| {
                    (i.priority.map_or(5, Priority::to_u32), i.updated_at)
                })
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| {
                    (
                        Reverse(i.priority.map_or(5, Priority::to_u32)),
                        Reverse(i.updated_at),
                    )
                })
                .cloned()
                .collect(),
        },
        BeadSort::Created => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.created_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.created_at))
                .cloned()
                .collect(),
        },
        BeadSort::Updated => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.updated_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.updated_at))
                .cloned()
                .collect(),
        },
        BeadSort::Closed => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.closed_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.closed_at))
                .cloned()
                .collect(),
        },
        BeadSort::Status => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.status)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.status))
                .cloned()
                .collect(),
        },
        BeadSort::Title => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.title.to_lowercase())
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.title.to_lowercase()))
                .cloned()
                .collect(),
        },
        BeadSort::Id => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| i.id.to_lowercase())
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i: &&BeadIssue| Reverse(i.id.to_lowercase()))
                .cloned()
                .collect(),
        },
    }
}

/// Paginate issues based on offset and limit.
#[must_use]
pub fn paginate(
    issues: &[BeadIssue],
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vec<BeadIssue> {
    let offset = offset.map_or(0, |v| v);
    #[allow(clippy::unnecessary_lazy_evaluations)]
    let limit = limit.unwrap_or_else(|| issues.len());
    issues.iter().skip(offset).take(limit).cloned().collect()
}

/// Apply a complete query to issues (filter, sort, and paginate).
#[must_use]
pub fn apply_query(issues: &[BeadIssue], query: &BeadQuery) -> Vec<BeadIssue> {
    issues
        .pipe(|i| filter_issues(i, &query.filter))
        .pipe(|i| sort_issues(&i, query.sort, query.direction))
        .pipe(|i| paginate(&i, query.filter.offset, query.filter.limit))
}
