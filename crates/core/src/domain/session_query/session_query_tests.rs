//! Session query tests
//!
//! Consolidated tests for session query functionality.

use std::path::PathBuf;

use crate::domain::{
    identifiers::SessionName,
    repository::Session,
    session::BranchState,
    session_query::{
        apply_query, filter_sessions, paginate_sessions, sort_sessions, SessionFilter,
        SessionQuery, SessionSort, SessionSortField, SortDirection,
    },
};

// ============================================================================
// TEST HELPERS
// ============================================================================

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

// ============================================================================
// SESSION FILTER TESTS
// ============================================================================

#[test]
fn test_empty_filter_matches_all() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new();
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 3);
}

#[test]
fn test_filter_by_name_contains() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_name_contains("alpha");
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name.as_str(), "alpha-session");
}

#[test]
fn test_filter_by_name_case_insensitive() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_name_contains("ALPHA");
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_filter_detached_only() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_detached_only();
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].branch.is_detached());
}

#[test]
fn test_filter_on_branch_only() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_on_branch_only();
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|s| !s.branch.is_detached()));
}

#[test]
fn test_filter_by_branch_name() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_branch("main");
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].branch.branch_name(), Some("main"));
}

#[test]
fn test_filter_combined() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new()
        .with_on_branch_only()
        .with_name_contains("session");
    let filtered: Vec<&Session> = sessions.iter().filter(|s| filter.matches(s)).collect();
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_builder_pattern() {
    let filter = SessionFilter::new()
        .with_status(crate::session_state::SessionState::Active)
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
    let filter = SessionFilter::new().with_status(crate::session_state::SessionState::Active);
    assert_eq!(
        filter.status,
        Some(crate::session_state::SessionState::Active)
    );
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

// ============================================================================
// SESSION SORT TESTS
// ============================================================================

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

// ============================================================================
// SESSION QUERY TESTS
// ============================================================================

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

#[test]
fn test_query_builder_pattern() {
    let query = SessionQuery::new()
        .with_name_contains("alpha")
        .with_sort(SessionSort::by_name_desc());

    assert!(query.filter.name_contains.is_some());
    assert!(query.sort.is_some());
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

// ============================================================================
// FILTER/SORT/PAGINATE FUNCTION TESTS
// ============================================================================

#[test]
fn test_filter_sessions_returns_vec() {
    let sessions = create_test_sessions();
    let filter = SessionFilter::new().with_name_contains("alpha");
    let result: Vec<Session> = filter_sessions(&sessions, &filter);
    assert!(!result.is_empty());
}

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

#[test]
fn test_sort_sessions_returns_vec() {
    let sessions = create_test_sessions();
    let sort = SessionSort::by_name_asc();
    let result: Vec<Session> = sort_sessions(&sessions, &sort);
    assert_eq!(result.len(), sessions.len());
}

#[test]
fn test_sort_preserves_all_sessions() {
    let sessions = create_test_sessions();
    let sort = SessionSort::by_name_asc();
    let sorted = sort_sessions(&sessions, &sort);
    assert_eq!(sorted.len(), sessions.len());
}

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

#[test]
fn test_paginate_sessions_returns_vec() {
    let sessions = create_test_sessions();
    let result: Vec<Session> = paginate_sessions(&sessions, Some(0), Some(2));
    assert_eq!(result.len(), 2);
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

// ============================================================================
// QUERY COMPOSITION TESTS
// ============================================================================

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
fn test_apply_query_returns_filtered_sorted_paginated() {
    let sessions = create_test_sessions();
    let query = SessionQuery::new()
        .with_filter(SessionFilter::new().with_on_branch_only())
        .with_sort(SessionSort::by_name_desc())
        .with_limit(1);
    let result = apply_query(&sessions, &query);
    assert!(result.len() <= 1);
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

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
