//! Session query operations
//!
//! Provides pure functions for filtering, sorting, and paginating sessions.
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `filter_sessions()` - pure function for filtering
//! - `sort_sessions()` - pure function for sorting
//! - `paginate_sessions()` - pure function for pagination
//! - `apply_query()` - compose filter + sort + paginate

use itertools::Itertools;
use tap::Pipe;

use crate::domain::repository::Session;

use super::session_filter::SessionFilter;
use super::session_query_types::SessionQuery;
use super::session_sort::{SessionSort, SessionSortField, SortDirection};

// ============================================================================
// FILTERING FUNCTIONS
// ============================================================================

/// Filter sessions based on filter criteria
///
/// Pure function - no side effects, deterministic.
/// Uses iterator pipeline with functional composition.
#[must_use]
pub fn filter_sessions(sessions: &[Session], filter: &SessionFilter) -> Vec<Session> {
    sessions
        .iter()
        .filter(|session| filter.matches(session))
        .cloned()
        .collect()
}

// ============================================================================
// SORTING FUNCTIONS
// ============================================================================

/// Sort sessions based on sort specification
///
/// Pure function - no side effects, deterministic.
/// Uses iterator pipeline with functional composition.
#[must_use]
pub fn sort_sessions(sessions: &[Session], sort: &SessionSort) -> Vec<Session> {
    let sorted = match (sort.field, sort.direction) {
        (SessionSortField::Name, SortDirection::Asc) => sessions
            .iter()
            .sorted_by_key(|s| s.name.as_str().to_lowercase())
            .cloned()
            .collect(),
        (SessionSortField::Name, SortDirection::Desc) => sessions
            .iter()
            .sorted_by(|a, b| {
                b.name
                    .as_str()
                    .to_lowercase()
                    .cmp(&a.name.as_str().to_lowercase())
            })
            .cloned()
            .collect(),
        (SessionSortField::Workspace, SortDirection::Asc) => sessions
            .iter()
            .sorted_by_key(|s| &s.workspace_path)
            .cloned()
            .collect(),
        (SessionSortField::Workspace, SortDirection::Desc) => sessions
            .iter()
            .sorted_by(|a, b| b.workspace_path.cmp(&a.workspace_path))
            .cloned()
            .collect(),
        (SessionSortField::Branch, SortDirection::Asc) => sessions
            .iter()
            .sorted_by_key(|s| s.branch.branch_name().unwrap_or(""))
            .cloned()
            .collect(),
        (SessionSortField::Branch, SortDirection::Desc) => sessions
            .iter()
            .sorted_by(|a, b| {
                let a_branch = a.branch.branch_name().unwrap_or("");
                let b_branch = b.branch.branch_name().unwrap_or("");
                b_branch.cmp(a_branch)
            })
            .cloned()
            .collect(),
    };
    sorted
}

// ============================================================================
// PAGINATION FUNCTIONS
// ============================================================================

/// Paginate sessions (skip + take)
///
/// Pure function - no side effects.
#[must_use]
pub fn paginate_sessions(
    sessions: &[Session],
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vec<Session> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(sessions.len());
    sessions.iter().skip(offset).take(limit).cloned().collect()
}

// ============================================================================
// QUERY APPLICATION
// ============================================================================

/// Apply a complete query (filter + sort + paginate)
///
/// Uses `tap::Pipe` for functional composition.
#[must_use]
pub fn apply_query(sessions: &[Session], query: &SessionQuery) -> Vec<Session> {
    sessions
        .pipe(|s| filter_sessions(s, &query.filter))
        .pipe(|s| {
            query
                .sort
                .as_ref()
                .map_or_else(|| s.clone(), |sort| sort_sessions(&s, sort))
        })
        .pipe(|s| paginate_sessions(&s, query.offset, query.limit))
}
