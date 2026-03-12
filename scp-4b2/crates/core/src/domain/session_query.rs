//! Session list query and filtering
//!
//! Provides functional filtering and sorting for session lists using:
//! - Value objects for filter criteria
//! - Iterator pipelines with `itertools` and `tap::Pipe`
//! - Railway-oriented error handling with `Result<T, E>`
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionFilter` - value object for filter criteria
//! - `SessionSort` - sort field and direction
//! - `filter_sessions()` - pure function for filtering
//! - `sort_sessions()` - pure function for sorting
//! - `apply_query()` - compose filter + sort + paginate

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![cfg_attr(test, allow(clippy::todo))]
#![cfg_attr(test, allow(clippy::unimplemented))]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_clone)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tap::Pipe;

use crate::domain::repository::{Session, SessionRepository};
use crate::session_state::SessionState;

// ============================================================================
// SESSION FILTER VALUE OBJECT
// ============================================================================

/// Filter criteria for session queries
///
/// A value object that encapsulates all filterable criteria.
/// All fields are optional - None means "don't filter by this criteria".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    /// Filter by session status (active, paused, completed, failed, etc.)
    /// Note: Currently simplified to check if session is "active" (has branch + valid workspace)
    #[serde(default)]
    pub status: Option<SessionState>,
    /// Filter by branch name (substring match)
    #[serde(default)]
    pub branch: Option<String>,
    /// Filter by session name (substring match, case-insensitive)
    #[serde(default)]
    pub name_contains: Option<String>,
    /// Filter by workspace path prefix
    #[serde(default)]
    pub workspace_prefix: Option<PathBuf>,
    /// Only include sessions with valid workspace paths
    #[serde(default)]
    pub valid_workspace_only: bool,
    /// Only include detached sessions
    #[serde(default)]
    pub detached_only: bool,
    /// Only include sessions on a branch (not detached)
    #[serde(default)]
    pub on_branch_only: bool,
}

impl SessionFilter {
    /// Create a new empty filter (matches everything)
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by session status
    #[must_use]
    pub fn with_status(mut self, status: SessionState) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by branch name
    #[must_use]
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Filter by name (case-insensitive substring)
    #[must_use]
    pub fn with_name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_contains = Some(name.into());
        self
    }

    /// Only include sessions with valid workspace paths
    #[must_use]
    pub fn with_valid_workspace_only(mut self) -> Self {
        self.valid_workspace_only = true;
        self
    }

    /// Only include detached sessions
    #[must_use]
    pub fn with_detached_only(mut self) -> Self {
        self.detached_only = true;
        self
    }

    /// Only include sessions on a branch
    #[must_use]
    pub fn with_on_branch_only(mut self) -> Self {
        self.on_branch_only = true;
        self
    }

    /// Check if a session matches this filter
    #[must_use]
    pub fn matches(&self, session: &Session) -> bool {
        // Note: Status filtering is simplified.
        // The session's is_active() represents "currently usable" (has branch + valid workspace).
        // For more complex status filtering, we'd need to map SessionState to this check.

        // Status filter - check if session is active based on filter
        let status_match = self.status.map_or(true, |_| session.is_active());

        // Branch filter (substring match)
        let branch_match = self.branch.as_ref().map_or(true, |branch_pattern| {
            session
                .branch
                .branch_name()
                .map_or(false, |name| name.contains(branch_pattern))
        });

        // Name contains filter (case-insensitive)
        let name_match = self.name_contains.as_ref().map_or(true, |pattern| {
            let pattern_lower = pattern.to_lowercase();
            session
                .name
                .as_str()
                .to_lowercase()
                .contains(&pattern_lower)
        });

        // Workspace prefix filter
        let workspace_match = self
            .workspace_prefix
            .as_ref()
            .map_or(true, |prefix| session.workspace_path.starts_with(prefix));

        // Valid workspace only
        let valid_workspace = !self.valid_workspace_only || session.workspace_path.exists();

        // Detached only
        let detached_match = !self.detached_only || session.branch.is_detached();

        // On branch only
        let on_branch_match = !self.on_branch_only || !session.branch.is_detached();

        status_match
            && branch_match
            && name_match
            && workspace_match
            && valid_workspace
            && detached_match
            && on_branch_match
    }
}

// ============================================================================
// SESSION SORT
// ============================================================================

/// Sort field for session queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSortField {
    #[default]
    /// Sort by session name
    Name,
    /// Sort by workspace path
    Workspace,
    /// Sort by branch name
    Branch,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// Sort specification for session queries
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionSort {
    pub field: SessionSortField,
    pub direction: SortDirection,
}

impl SessionSort {
    /// Create a new sort specification
    #[must_use]
    pub fn new(field: SessionSortField, direction: SortDirection) -> Self {
        Self { field, direction }
    }

    /// Sort by name ascending
    #[must_use]
    pub fn by_name_asc() -> Self {
        Self {
            field: SessionSortField::Name,
            direction: SortDirection::Asc,
        }
    }

    /// Sort by name descending
    #[must_use]
    pub fn by_name_desc() -> Self {
        Self {
            field: SessionSortField::Name,
            direction: SortDirection::Desc,
        }
    }
}

// ============================================================================
// SESSION QUERY
// ============================================================================

/// Complete query specification for sessions
///
/// Combines filter, sort, and pagination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionQuery {
    pub filter: SessionFilter,
    pub sort: Option<SessionSort>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl SessionQuery {
    /// Create a new query
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a filter
    #[must_use]
    pub fn with_filter(mut self, filter: SessionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Add sorting
    #[must_use]
    pub fn with_sort(mut self, sort: SessionSort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Add offset
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add limit
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Filter by name contains (delegates to filter)
    #[must_use]
    pub fn with_name_contains(mut self, name: impl Into<String>) -> Self {
        self.filter = self.filter.with_name_contains(name);
        self
    }
}

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
                .map_or(s.clone(), |sort| sort_sessions(&s, sort))
        })
        .pipe(|s| paginate_sessions(&s, query.offset, query.limit))
}

// ============================================================================
// REPOSITORY EXTENSIONS
// ============================================================================

/// Extension trait for SessionRepository to support filtering
pub trait SessionRepositoryExt {
    /// List sessions with a filter
    fn list_filtered(&self, filter: &SessionFilter) -> Vec<Session>;

    /// List sessions with a complete query
    fn query(&self, query: &SessionQuery) -> Vec<Session>;
}

impl<R: SessionRepository> SessionRepositoryExt for R {
    fn list_filtered(&self, filter: &SessionFilter) -> Vec<Session> {
        self.list_all()
            .map(|sessions| filter_sessions(&sessions, filter))
            .unwrap_or_default()
    }

    fn query(&self, query: &SessionQuery) -> Vec<Session> {
        self.list_all()
            .map(|sessions| apply_query(&sessions, query))
            .unwrap_or_default()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::SessionName;
    use crate::domain::session::BranchState;

    /// Helper to create test sessions
    fn create_test_sessions() -> Vec<Session> {
        vec![
            Session {
                id: crate::domain::identifiers::SessionId::parse("session-1").unwrap(),
                name: SessionName::parse("alpha-session").unwrap(),
                branch: BranchState::OnBranch {
                    name: "main".to_string(),
                },
                workspace_path: PathBuf::from("/tmp/workspace-alpha"),
            },
            Session {
                id: crate::domain::identifiers::SessionId::parse("session-2").unwrap(),
                name: SessionName::parse("beta-session").unwrap(),
                branch: BranchState::OnBranch {
                    name: "feature".to_string(),
                },
                workspace_path: PathBuf::from("/tmp/workspace-beta"),
            },
            Session {
                id: crate::domain::identifiers::SessionId::parse("session-3").unwrap(),
                name: SessionName::parse("gamma-session").unwrap(),
                branch: BranchState::Detached,
                workspace_path: PathBuf::from("/tmp/workspace-gamma"),
            },
        ]
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FILTER TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_empty_filter_matches_all() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new();
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_filter_by_name_contains() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_name_contains("alpha");
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name.as_str(), "alpha-session");
    }

    #[test]
    fn test_filter_by_name_case_insensitive() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_name_contains("ALPHA");
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_detached_only() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_detached_only();
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].branch.is_detached());
    }

    #[test]
    fn test_filter_on_branch_only() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_on_branch_only();
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|s| !s.branch.is_detached()));
    }

    #[test]
    fn test_filter_by_branch_name() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_branch("main");
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch.branch_name(), Some("main"));
    }

    #[test]
    fn test_filter_combined() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new()
            .with_on_branch_only()
            .with_name_contains("session");
        let filtered = filter_sessions(&sessions, &filter);
        assert_eq!(filtered.len(), 2);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SORT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_sort_by_name_asc() {
        let sessions = create_test_sessions();
        let sort = SessionSort::by_name_asc();
        let sorted = sort_sessions(&sessions, &sort);
        assert_eq!(sorted[0].name.as_str(), "alpha-session");
        assert_eq!(sorted[1].name.as_str(), "beta-session");
        assert_eq!(sorted[2].name.as_str(), "gamma-session");
    }

    #[test]
    fn test_sort_by_name_desc() {
        let sessions = create_test_sessions();
        let sort = SessionSort::new(SessionSortField::Name, SortDirection::Desc);
        let sorted = sort_sessions(&sessions, &sort);
        assert_eq!(sorted[0].name.as_str(), "gamma-session");
        assert_eq!(sorted[1].name.as_str(), "beta-session");
        assert_eq!(sorted[2].name.as_str(), "alpha-session");
    }

    #[test]
    fn test_sort_by_workspace_asc() {
        let sessions = create_test_sessions();
        let sort = SessionSort::new(SessionSortField::Workspace, SortDirection::Asc);
        let sorted = sort_sessions(&sessions, &sort);
        assert!(sorted[0].workspace_path.to_string_lossy().contains("alpha"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PAGINATION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_paginate_with_offset_and_limit() {
        let sessions = create_test_sessions();
        let paginated = paginate_sessions(&sessions, Some(1), Some(1));
        assert_eq!(paginated.len(), 1);
        assert_eq!(paginated[0].name.as_str(), "beta-session");
    }

    #[test]
    fn test_paginate_no_offset() {
        let sessions = create_test_sessions();
        let paginated = paginate_sessions(&sessions, None, Some(2));
        assert_eq!(paginated.len(), 2);
    }

    #[test]
    fn test_paginate_no_limit() {
        let sessions = create_test_sessions();
        let paginated = paginate_sessions(&sessions, Some(1), None);
        assert_eq!(paginated.len(), 2);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // QUERY COMPOSITION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_apply_query_full() {
        let sessions = create_test_sessions();
        let query = SessionQuery::new()
            .with_filter(SessionFilter::new().with_on_branch_only())
            .with_sort(SessionSort::by_name_asc())
            .with_offset(0)
            .with_limit(10);
        let result = apply_query(&sessions, &query);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|s| !s.branch.is_detached()));
    }

    #[test]
    fn test_query_builder_pattern() {
        let query = SessionQuery::new()
            .with_name_contains("alpha")
            .with_sort(SessionSort::by_name_desc());

        assert!(query.filter.name_contains.is_some());
        assert!(query.sort.is_some());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FILTER VALUE OBJECT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_filter_builder_pattern() {
        let filter = SessionFilter::new()
            .with_status(SessionState::Active)
            .with_branch("feature")
            .with_name_contains("test")
            .with_valid_workspace_only();

        assert!(filter.status.is_some());
        assert!(filter.branch.is_some());
        assert!(filter.name_contains.is_some());
        assert!(filter.valid_workspace_only);
    }

    #[test]
    fn test_filter_matches_all_when_empty() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new();

        for session in &sessions {
            assert!(
                filter.matches(session),
                "Filter should match all sessions when empty"
            );
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSIONFILTER STRUCT FIELD TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_filter_default() {
        let filter = SessionFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.branch.is_none());
        assert!(filter.name_contains.is_none());
        assert!(filter.workspace_prefix.is_none());
        assert!(!filter.valid_workspace_only);
        assert!(!filter.detached_only);
        assert!(!filter.on_branch_only);
    }

    #[test]
    fn test_session_filter_with_status() {
        let filter = SessionFilter::new().with_status(SessionState::Active);
        assert_eq!(filter.status, Some(SessionState::Active));
    }

    #[test]
    fn test_session_filter_with_branch() {
        let filter = SessionFilter::new().with_branch("main");
        assert_eq!(filter.branch, Some("main".to_string()));
    }

    #[test]
    fn test_session_filter_with_name_contains() {
        let filter = SessionFilter::new().with_name_contains("test");
        assert_eq!(filter.name_contains, Some("test".to_string()));
    }

    #[test]
    fn test_session_filter_valid_workspace_only() {
        let filter = SessionFilter::new().with_valid_workspace_only();
        assert!(filter.valid_workspace_only);
    }

    #[test]
    fn test_session_filter_detached_only() {
        let filter = SessionFilter::new().with_detached_only();
        assert!(filter.detached_only);
    }

    #[test]
    fn test_session_filter_on_branch_only() {
        let filter = SessionFilter::new().with_on_branch_only();
        assert!(filter.on_branch_only);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSIONSORT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_sort_by_name_asc() {
        let sort = SessionSort::by_name_asc();
        assert_eq!(sort.field, SessionSortField::Name);
        assert_eq!(sort.direction, SortDirection::Asc);
    }

    #[test]
    fn test_session_sort_by_name_desc() {
        let sort = SessionSort::by_name_desc();
        assert_eq!(sort.field, SessionSortField::Name);
        assert_eq!(sort.direction, SortDirection::Desc);
    }

    #[test]
    fn test_session_sort_new() {
        let sort = SessionSort::new(SessionSortField::Branch, SortDirection::Desc);
        assert_eq!(sort.field, SessionSortField::Branch);
        assert_eq!(sort.direction, SortDirection::Desc);
    }

    #[test]
    fn test_session_sort_default() {
        let sort = SessionSort::default();
        assert_eq!(sort.field, SessionSortField::Name);
        assert_eq!(sort.direction, SortDirection::Asc);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSIONQUERY TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_query_new() {
        let query = SessionQuery::new();
        assert!(query.filter.status.is_none());
        assert!(query.sort.is_none());
        assert!(query.offset.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_session_query_with_filter() {
        let filter = SessionFilter::new().with_name_contains("test");
        let query = SessionQuery::new().with_filter(filter.clone());
        assert_eq!(query.filter.name_contains, filter.name_contains);
    }

    #[test]
    fn test_session_query_with_sort() {
        let sort = SessionSort::by_name_desc();
        let query = SessionQuery::new().with_sort(sort.clone());
        assert_eq!(query.sort, Some(sort));
    }

    #[test]
    fn test_session_query_with_offset() {
        let query = SessionQuery::new().with_offset(10);
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_session_query_with_limit() {
        let query = SessionQuery::new().with_limit(5);
        assert_eq!(query.limit, Some(5));
    }

    #[test]
    fn test_session_query_builder_chaining() {
        let query = SessionQuery::new()
            .with_filter(SessionFilter::new().with_name_contains("test"))
            .with_sort(SessionSort::by_name_asc())
            .with_offset(0)
            .with_limit(100);

        assert!(query.filter.name_contains.is_some());
        assert!(query.sort.is_some());
        assert_eq!(query.offset, Some(0));
        assert_eq!(query.limit, Some(100));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FUNCTIONAL COMPOSITION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_filter_sessions_returns_vec() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_name_contains("alpha");
        let result: Vec<Session> = filter_sessions(&sessions, &filter);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_sort_sessions_returns_vec() {
        let sessions = create_test_sessions();
        let sort = SessionSort::by_name_asc();
        let result: Vec<Session> = sort_sessions(&sessions, &sort);
        assert_eq!(result.len(), sessions.len());
    }

    #[test]
    fn test_paginate_sessions_returns_vec() {
        let sessions = create_test_sessions();
        let result: Vec<Session> = paginate_sessions(&sessions, Some(0), Some(2));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_apply_query_returns_filtered_sorted_paginated() {
        let sessions = create_test_sessions();
        let query = SessionQuery::new()
            .with_filter(SessionFilter::new().with_on_branch_only())
            .with_sort(SessionSort::by_name_desc())
            .with_limit(1);
        let result = apply_query(&sessions, &query);
        assert!(result.len() <= 1);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EDGE CASE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_filter_empty_sessions() {
        let sessions: Vec<Session> = vec![];
        let filter = SessionFilter::new();
        let filtered = filter_sessions(&sessions, &filter);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_no_matches() {
        let sessions = create_test_sessions();
        let filter = SessionFilter::new().with_name_contains("nonexistent");
        let filtered = filter_sessions(&sessions, &filter);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_paginate_beyond_length() {
        let sessions = create_test_sessions();
        let paginated = paginate_sessions(&sessions, Some(10), Some(10));
        assert!(paginated.is_empty());
    }

    #[test]
    fn test_paginate_zero_limit() {
        let sessions = create_test_sessions();
        let paginated = paginate_sessions(&sessions, Some(0), Some(0));
        assert!(paginated.is_empty());
    }

    #[test]
    fn test_sort_preserves_all_sessions() {
        let sessions = create_test_sessions();
        let sort = SessionSort::by_name_asc();
        let sorted = sort_sessions(&sessions, &sort);
        assert_eq!(sorted.len(), sessions.len());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SERIALIZATION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_session_filter_serialize() {
        let filter = SessionFilter::new()
            .with_name_contains("test")
            .with_on_branch_only();
        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_session_filter_deserialize() {
        let json = r#"{"name_contains":"test","on_branch_only":true}"#;
        let filter: SessionFilter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.name_contains, Some("test".to_string()));
        assert!(filter.on_branch_only);
    }

    #[test]
    fn test_session_sort_serialize() {
        let sort = SessionSort::by_name_desc();
        let json = serde_json::to_string(&sort).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("desc"));
    }

    #[test]
    fn test_session_sort_deserialize() {
        let json = r#"{"field":"branch","direction":"asc"}"#;
        let sort: SessionSort = serde_json::from_str(json).unwrap();
        assert_eq!(sort.field, SessionSortField::Branch);
        assert_eq!(sort.direction, SortDirection::Asc);
    }

    #[test]
    fn test_session_query_serialize() {
        let query = SessionQuery::new()
            .with_filter(SessionFilter::new().with_name_contains("test"))
            .with_sort(SessionSort::by_name_asc())
            .with_offset(5)
            .with_limit(10);
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("name"));
    }

    #[test]
    fn test_session_query_deserialize() {
        let json = r#"{
            "filter": {"name_contains": "test", "on_branch_only": true},
            "sort": {"field": "name", "direction": "asc"},
            "offset": 5,
            "limit": 10
        }"#;
        let query: SessionQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.filter.name_contains, Some("test".to_string()));
        assert!(query.filter.on_branch_only);
        assert_eq!(query.sort.unwrap().field, SessionSortField::Name);
        assert_eq!(query.offset, Some(5));
        assert_eq!(query.limit, Some(10));
    }
}
