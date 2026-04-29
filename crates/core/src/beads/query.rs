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

pub fn matches_filter(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    (filter.status.is_empty() || filter.status.contains(&issue.status))
        && (filter.issue_type.is_empty()
            || issue
                .issue_type
                .as_ref()
                .is_some_and(|t| filter.issue_type.contains(t)))
        && (filter
            .priority_min
            .as_ref()
            .is_none_or(|min| issue.priority.is_none_or(|p| p >= *min)))
        && (filter
            .priority_max
            .as_ref()
            .is_none_or(|max| issue.priority.is_none_or(|p| p <= *max)))
        && (filter.labels.is_empty()
            || issue
                .labels
                .as_ref()
                .is_some_and(|issue_labels| filter.labels.iter().all(|l| issue_labels.contains(l))))
        && (filter
            .assignee
            .as_ref()
            .is_none_or(|assignee| issue.assignee.as_ref() == Some(assignee)))
        && (filter
            .parent
            .as_ref()
            .is_none_or(|parent| issue.parent.as_ref() == Some(parent)))
        && (!filter.has_parent || issue.parent.is_some())
        && (!filter.blocked_only || issue.is_blocked())
        && filter.search_text.as_ref().is_none_or(|text| {
            let text_lower = text.to_lowercase();
            issue.title.to_lowercase().contains(&text_lower)
                || issue
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&text_lower))
        })
}

/// Sort issues by a simple key with direction awareness.
fn sort_by_key<K>(
    issues: &[BeadIssue],
    key_fn: impl Fn(&BeadIssue) -> K,
    direction: SortDirection,
) -> Vec<BeadIssue>
where
    K: Ord,
{
    match direction {
        SortDirection::Asc => issues
            .iter()
            .sorted_by_key(|i| key_fn(i))
            .cloned()
            .collect(),
        SortDirection::Desc => issues
            .iter()
            .sorted_by_key(|i| Reverse(key_fn(i)))
            .cloned()
            .collect(),
    }
}

/// Sort issues based on the given sort field and direction.
#[must_use]
pub fn sort_issues(
    issues: &[BeadIssue],
    sort: BeadSort,
    direction: SortDirection,
) -> Vec<BeadIssue> {
    match sort {
        BeadSort::Priority => sort_by_key(
            issues,
            |i: &BeadIssue| (i.priority.map_or(5, Priority::to_u32), i.updated_at),
            direction,
        ),
        BeadSort::Created => sort_by_key(issues, |i: &BeadIssue| i.created_at, direction),
        BeadSort::Updated => sort_by_key(issues, |i: &BeadIssue| i.updated_at, direction),
        BeadSort::Closed => sort_by_key(issues, |i: &BeadIssue| i.closed_at, direction),
        BeadSort::Status => sort_by_key(issues, |i: &BeadIssue| i.status, direction),
        BeadSort::Title => {
            sort_by_key(issues, |i: &BeadIssue| i.title.to_lowercase(), direction)
        }
        BeadSort::Id => sort_by_key(issues, |i: &BeadIssue| i.id.to_lowercase(), direction),
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
