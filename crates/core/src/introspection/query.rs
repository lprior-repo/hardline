//! Query result types
//!
//! This module provides types for state queries including
//! session existence, counts, and capability checks.

use serde::{Deserialize, Serialize};

/// Error information for failed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Query result for session existence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExistsQuery {
    /// Whether the session exists (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// Session details if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Basic session information for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name
    pub name: String,
    /// Session status
    pub status: String,
}

/// Query result for session count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCountQuery {
    /// Number of sessions matching filter (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Filter that was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Query result for "can run" check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanRunQuery {
    /// Whether the command can be run
    pub can_run: bool,
    /// Command being checked
    pub command: String,
    /// Prerequisites that are blocking execution
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,
    /// Number of prerequisites met
    pub prerequisites_met: usize,
    /// Total number of prerequisites
    pub prerequisites_total: usize,
}

/// A prerequisite that is blocking command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Check name
    pub check: String,
    /// Check status (should be false)
    pub status: bool,
    /// Human-readable message
    pub message: String,
}

/// Query result for name suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestNameQuery {
    /// Pattern used
    pub pattern: String,
    /// Suggested name
    pub suggested: String,
    /// Next available number in sequence
    pub next_available_n: usize,
    /// Existing names matching pattern
    pub existing_matches: Vec<String>,
}
