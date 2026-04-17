//! Database domain types - Newtypes for validated domain concepts

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// Constant for in-memory database path
pub const IN_MEMORY_PATH: &str = ":memory:";

/// Newtype for database path with validation
/// Ensures path is non-empty and makes illegal states unrepresentable
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabasePath(String);

impl DatabasePath {
    /// Create a new validated database path
    ///
    /// Returns Error::InvalidConfig if path is empty
    pub fn new(path: impl Into<String>) -> Result<Self, Error> {
        let path = path.into();
        if path.is_empty() {
            return Err(Error::config_invalid(
                "Database path cannot be empty".to_string(),
            ));
        }
        Ok(Self(path))
    }

    /// Create an in-memory database path
    pub fn in_memory() -> Self {
        Self(IN_MEMORY_PATH.to_string())
    }

    /// Check if this is an in-memory database
    pub fn is_in_memory(&self) -> bool {
        self.0 == IN_MEMORY_PATH
    }

    /// Get the underlying path string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DatabasePath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for DatabasePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype for max connections with validation
/// Ensures max_connections > 0, making illegal states unrepresentable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaxConnections(u32);

impl MaxConnections {
    /// Create a new validated max connections value
    ///
    /// Returns Error::InvalidConfig if value is 0
    pub fn new(value: u32) -> Result<Self, Error> {
        if value == 0 {
            return Err(Error::config_invalid(
                "Max connections must be greater than 0".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Create with default value of 5
    pub fn default_value() -> Self {
        Self(5)
    }

    /// Get the underlying value
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for MaxConnections {
    fn default() -> Self {
        Self::default_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DatabasePath construction
    // =========================================================================

    #[test]
    fn given_valid_path_when_new_then_succeeds() {
        let path = DatabasePath::new("/tmp/test.db");
        assert!(path.is_ok());
        assert_eq!(path.unwrap().as_str(), "/tmp/test.db");
    }

    #[test]
    fn given_relative_path_when_new_then_succeeds() {
        let path = DatabasePath::new("data/test.db");
        assert!(path.is_ok());
        assert_eq!(path.unwrap().as_str(), "data/test.db");
    }

    #[test]
    fn given_empty_path_when_new_then_returns_config_error() {
        let result = DatabasePath::new("");
        assert!(result.is_err());
    }

    #[test]
    fn given_whitespace_path_when_new_then_succeeds() {
        let path = DatabasePath::new("  ");
        assert!(path.is_ok());
        assert_eq!(path.unwrap().as_str(), "  ");
    }

    #[test]
    fn given_unicode_path_when_new_then_succeeds() {
        let path = DatabasePath::new("/tmp/cafe\u{00e9}.db");
        assert!(path.is_ok());
        assert_eq!(path.unwrap().as_str(), "/tmp/cafe\u{00e9}.db");
    }

    // =========================================================================
    // DatabasePath in_memory
    // =========================================================================

    #[test]
    fn given_in_memory_when_is_in_memory_then_true() {
        let path = DatabasePath::in_memory();
        assert!(path.is_in_memory());
        assert_eq!(path.as_str(), ":memory:");
    }

    #[test]
    fn given_regular_path_when_is_in_memory_then_false() {
        let path = DatabasePath::new("/tmp/test.db").unwrap();
        assert!(!path.is_in_memory());
    }

    // =========================================================================
    // DatabasePath FromStr
    // =========================================================================

    #[test]
    fn given_valid_str_when_from_str_then_succeeds() {
        let path: DatabasePath = "/tmp/test.db".parse().unwrap();
        assert_eq!(path.as_str(), "/tmp/test.db");
    }

    #[test]
    fn given_empty_str_when_from_str_then_fails() {
        let result: Result<DatabasePath, _> = "".parse();
        assert!(result.is_err());
    }

    // =========================================================================
    // DatabasePath Display
    // =========================================================================

    #[test]
    fn given_path_when_display_then_returns_inner_string() {
        let path = DatabasePath::new("/tmp/test.db").unwrap();
        assert_eq!(format!("{path}"), "/tmp/test.db");
    }

    #[test]
    fn given_in_memory_when_display_then_returns_memory_string() {
        let path = DatabasePath::in_memory();
        assert_eq!(format!("{path}"), ":memory:");
    }

    // =========================================================================
    // DatabasePath Clone / Eq / Hash
    // =========================================================================

    #[test]
    fn given_path_when_cloned_then_equal() {
        let path = DatabasePath::new("/tmp/test.db").unwrap();
        let cloned = path.clone();
        assert_eq!(path, cloned);
    }

    #[test]
    fn given_different_paths_when_compared_then_not_equal() {
        let a = DatabasePath::new("/tmp/a.db").unwrap();
        let b = DatabasePath::new("/tmp/b.db").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn given_path_when_hashed_then_consistent() {
        use std::collections::HashSet;
        let a = DatabasePath::new("/tmp/a.db").unwrap();
        let b = DatabasePath::new("/tmp/b.db").unwrap();
        let set: HashSet<DatabasePath> = [a.clone(), b.clone()].into_iter().collect();
        assert!(set.contains(&a));
        assert!(set.contains(&b));
        assert_eq!(set.len(), 2);
    }

    // =========================================================================
    // DatabasePath Serialize / Deserialize
    // =========================================================================

    #[test]
    fn given_path_when_serialized_then_roundtrips() {
        let path = DatabasePath::new("/tmp/test.db").unwrap();
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: DatabasePath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, deserialized);
    }

    #[test]
    fn given_in_memory_path_when_serialized_then_roundtrips() {
        let path = DatabasePath::in_memory();
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: DatabasePath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, deserialized);
    }

    // =========================================================================
    // MaxConnections construction
    // =========================================================================

    #[test]
    fn given_positive_value_when_new_then_succeeds() {
        let mc = MaxConnections::new(1).unwrap();
        assert_eq!(mc.value(), 1);
    }

    #[test]
    fn given_large_value_when_new_then_succeeds() {
        let mc = MaxConnections::new(u32::MAX).unwrap();
        assert_eq!(mc.value(), u32::MAX);
    }

    #[test]
    fn given_zero_when_new_then_returns_config_error() {
        let result = MaxConnections::new(0);
        assert!(result.is_err());
    }

    // =========================================================================
    // MaxConnections default
    // =========================================================================

    #[test]
    fn given_default_then_value_is_five() {
        let mc = MaxConnections::default();
        assert_eq!(mc.value(), 5);
    }

    #[test]
    fn given_default_value_then_same_as_default() {
        let mc = MaxConnections::default_value();
        assert_eq!(mc.value(), MaxConnections::default().value());
    }

    // =========================================================================
    // MaxConnections Clone / Eq
    // =========================================================================

    #[test]
    fn given_max_connections_when_cloned_then_equal() {
        let mc = MaxConnections::new(10).unwrap();
        let cloned = mc.clone();
        assert_eq!(mc, cloned);
    }

    #[test]
    fn given_different_max_connections_when_compared_then_not_equal() {
        let a = MaxConnections::new(1).unwrap();
        let b = MaxConnections::new(2).unwrap();
        assert_ne!(a, b);
    }

    // =========================================================================
    // MaxConnections Copy (since it derives Copy)
    // =========================================================================

    #[test]
    fn given_max_connections_when_copied_then_same_value() {
        let mc = MaxConnections::new(7).unwrap();
        let copy = mc;
        assert_eq!(mc.value(), copy.value());
    }

    // =========================================================================
    // MaxConnections Serialize / Deserialize
    // =========================================================================

    #[test]
    fn given_max_connections_when_serialized_then_roundtrips() {
        let mc = MaxConnections::new(10).unwrap();
        let json = serde_json::to_string(&mc).unwrap();
        let deserialized: MaxConnections = serde_json::from_str(&json).unwrap();
        assert_eq!(mc, deserialized);
    }

    // =========================================================================
    // IN_MEMORY_PATH constant
    // =========================================================================

    #[test]
    fn given_in_memory_path_constant_then_is_memory_string() {
        assert_eq!(IN_MEMORY_PATH, ":memory:");
    }

    // =========================================================================
    // Debug formatting
    // =========================================================================

    #[test]
    fn given_database_path_when_debug_then_contains_path() {
        let path = DatabasePath::new("/tmp/test.db").unwrap();
        let debug = format!("{path:?}");
        assert!(debug.contains("/tmp/test.db"));
    }

    #[test]
    fn given_max_connections_when_debug_then_contains_value() {
        let mc = MaxConnections::new(5).unwrap();
        let debug = format!("{mc:?}");
        assert!(debug.contains("5"));
    }
}
