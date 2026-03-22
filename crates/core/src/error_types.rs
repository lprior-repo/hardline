//! Shared error types and re-exports.
//!
//! This module provides JjConflictType and Result type alias.

/// Types of JJ workspace conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JjConflictType {
    /// Workspace already exists
    AlreadyExists,
    /// Concurrent modification detected
    ConcurrentModification,
    /// Workspace has been abandoned
    Abandoned,
    /// Working copy is stale
    Stale,
}

/// Result type alias using our custom error
pub type Result<T> = std::result::Result<T, crate::error::Error>;
