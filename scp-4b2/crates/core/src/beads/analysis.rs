#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use chrono::{Duration, Utc};
use im::HashMap;
use itertools::Itertools;

use super::{
    query::matches_filter,
    types::{BeadFilter, BeadIssue, BeadsSummary, IssueStatus},
};

/// Generate a summary of issues.
#[must_use]
pub fn summarize(issues: &[BeadIssue]) -> BeadsSummary {
    BeadsSummary::from_issues(issues)
}

/// Find issues that are blocking other issues.
#[must_use]
pub fn find_blockers(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

/// Find issues that are blocked.
#[must_use]
pub fn find_blocked(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    issues.iter().filter(|i| i.is_blocked()).cloned().collect()
}

/// Get the dependency graph of issues.
///
/// Returns a mapping from issue ID to the list of issues that depend on it.
#[must_use]
pub fn get_dependency_graph(issues: &[BeadIssue]) -> HashMap<String, Vec<String>> {
    issues
        .iter()
        .filter_map(|issue| {
            issue
                .depends_on
                .as_ref()
                .map(|deps| deps.iter().map(move |dep| (dep.clone(), issue.id.clone())))
        })
        .flatten()
        .into_group_map()
        .into_iter()
        .collect()
}

/// Group issues by their status.
#[must_use]
pub fn group_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.status, issue.clone()))
        .into_group_map()
        .into_iter()
        .collect()
}

/// Group issues by their type.
#[must_use]
pub fn group_by_type(
    issues: &[BeadIssue],
) -> HashMap<Option<super::types::IssueType>, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.issue_type.clone(), issue.clone()))
        .into_group_map()
        .into_iter()
        .collect()
}

/// Find issues that are ready to be worked on (open and not blocked).
#[must_use]
pub fn find_ready(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| i.is_open() && !blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

/// Find issues that haven't been updated in a while.
///
/// # Panics
///
/// Panics if `days` value would cause integer overflow when converting to i64.
/// This is intentional - extremely large day values indicate a bug in the caller.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn find_stale(issues: &[BeadIssue], days: u64) -> Vec<BeadIssue> {
    #[allow(clippy::arithmetic_side_effects)]
    let cutoff = Utc::now() - Duration::days(days as i64);

    issues
        .iter()
        .filter(|i| i.updated_at < cutoff && i.status != IssueStatus::Closed)
        .cloned()
        .collect()
}

/// Find potential duplicate issues based on title similarity.
#[must_use]
pub fn find_potential_duplicates(
    issues: &[BeadIssue],
    threshold: usize,
) -> Vec<(BeadIssue, Vec<BeadIssue>)> {
    let issues_vec: Vec<BeadIssue> = issues.to_vec();

    issues_vec
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < issues_vec.len().saturating_sub(1))
        .filter_map(|(i, issue)| {
            #[allow(clippy::arithmetic_side_effects)]
            let similar: Vec<BeadIssue> = issues_vec
                .iter()
                .skip(i + 1)
                .filter(|other| {
                    let self_words: std::collections::HashSet<_> =
                        issue.title.split_whitespace().collect();
                    let other_words: std::collections::HashSet<_> =
                        other.title.split_whitespace().collect();
                    self_words.intersection(&other_words).count() >= threshold
                })
                .cloned()
                .collect();

            if similar.is_empty() {
                None
            } else {
                Some((issue.clone(), similar))
            }
        })
        .collect()
}

/// Get a specific issue by ID.
///
/// # Returns
///
/// Returns `Some(issue)` if found, `None` otherwise. The result should be checked
/// as this is a lookup operation that may fail.
#[must_use]
pub fn get_issue(issues: &[BeadIssue], id: &str) -> Option<BeadIssue> {
    issues.iter().find(|i| i.id == id).cloned()
}

/// Get multiple issues by their IDs.
#[must_use]
pub fn get_issues_by_id(issues: &[BeadIssue], ids: &[String]) -> Vec<BeadIssue> {
    let id_set: std::collections::HashSet<_> = ids.iter().collect();
    issues
        .iter()
        .filter(|i| id_set.contains(&i.id))
        .cloned()
        .collect()
}

/// Calculate the critical path through the dependency graph.
///
/// Returns the longest path of dependent issues.
#[must_use]
pub fn calculate_critical_path(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        path: &mut Vec<BeadIssue>,
        visited: &mut std::collections::HashSet<String>,
        all_issues: &[BeadIssue],
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(issue) = all_issues.iter().find(|i| i.id == node) {
            path.push(issue.clone());
        }

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                dfs(dep, graph, path, visited, all_issues);
            }
        }
    }

    let graph = get_dependency_graph(issues);

    let critical_path = issues
        .iter()
        .filter_map(|issue| {
            let mut path = Vec::new();
            let mut visited = std::collections::HashSet::new();
            dfs(&issue.id, &graph, &mut path, &mut visited, issues);
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        })
        .max_by_key(std::vec::Vec::len);

    critical_path.unwrap_or_default()
}

/// Extract issue IDs.
#[must_use]
pub fn to_ids(issues: &[BeadIssue]) -> Vec<String> {
    issues.iter().map(|i| i.id.clone()).collect()
}

/// Extract issue titles.
#[must_use]
pub fn to_titles(issues: &[BeadIssue]) -> Vec<String> {
    issues.iter().map(|i| i.title.clone()).collect()
}

/// Extract all unique labels from issues.
#[must_use]
pub fn extract_labels(issues: &[BeadIssue]) -> Vec<String> {
    issues
        .iter()
        .filter_map(|i| i.labels.as_ref())
        .flatten()
        .unique()
        .cloned()
        .collect()
}

/// Count issues by status.
#[must_use]
pub fn count_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, usize> {
    issues
        .iter()
        .map(|issue| issue.status)
        .counts()
        .into_iter()
        .collect()
}

/// Check if any issues match the given filter.
#[must_use]
pub fn any_match(issues: &[BeadIssue], filter: &BeadFilter) -> bool {
    issues.iter().any(|i| matches_filter(i, filter))
}

/// Check if all issues match the given filter.
#[must_use]
pub fn all_match(issues: &[BeadIssue], filter: &BeadFilter) -> bool {
    issues.iter().all(|i| matches_filter(i, filter))
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_find_blockers() {
        let issues = vec![
            BeadIssue {
                id: "blocked-1".to_string(),
                title: "Blocked Issue".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vec!["blocker-1".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "blocker-1".to_string(),
                title: "Blocker Issue".to_string(),
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
        ];

        let blockers = find_blockers(&issues);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].id, "blocker-1");
    }

    #[test]
    fn test_find_ready() {
        let issues = vec![
            BeadIssue {
                id: "ready-1".to_string(),
                title: "Ready Issue".to_string(),
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
                id: "blocked-1".to_string(),
                title: "Blocked Issue".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vec!["blocker-1".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let ready = find_ready(&issues);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "ready-1");
    }

    #[test]
    fn test_group_by_status() {
        let issues = vec![
            BeadIssue {
                id: "issue-1".to_string(),
                title: "Open Issue".to_string(),
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
                id: "issue-2".to_string(),
                title: "Closed Issue".to_string(),
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

        let grouped = group_by_status(&issues);
        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key(&IssueStatus::Open));
        assert!(grouped.contains_key(&IssueStatus::Closed));
    }
}
