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
            return Err(Error::InvalidConfig("Database path cannot be empty".to_string()));
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
            return Err(Error::InvalidConfig(
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
