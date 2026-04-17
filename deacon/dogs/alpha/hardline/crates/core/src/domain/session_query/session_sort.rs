//! Session sort types
//!
//! Provides sort field, direction, and specification types.
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionSortField` - enumerated sort fields
//! - `SortDirection` - ascending/descending
//! - `SessionSort` - complete sort specification

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSort {
    pub field: SessionSortField,
    pub direction: SortDirection,
}

impl SessionSort {
    /// Create a new sort specification
    #[must_use]
    pub const fn new(field: SessionSortField, direction: SortDirection) -> Self {
        Self { field, direction }
    }

    /// Sort by name ascending
    #[must_use]
    pub const fn by_name_asc() -> Self {
        Self {
            field: SessionSortField::Name,
            direction: SortDirection::Asc,
        }
    }

    /// Sort by name descending
    #[must_use]
    pub const fn by_name_desc() -> Self {
        Self {
            field: SessionSortField::Name,
            direction: SortDirection::Desc,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
