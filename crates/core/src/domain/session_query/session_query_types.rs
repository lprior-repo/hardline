//! Session query types
//!
//! Provides the complete query specification combining filter, sort, and pagination.
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionQuery` - complete query specification with builder pattern

use serde::{Deserialize, Serialize};

use super::{session_filter::SessionFilter, session_sort::SessionSort};

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
    pub const fn with_sort(mut self, sort: SessionSort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Add offset
    #[must_use]
    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add limit
    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
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
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            query.sort.unwrap().field,
            super::super::SessionSortField::Name
        );
        assert_eq!(query.offset, Some(5));
        assert_eq!(query.limit, Some(10));
    }
}
