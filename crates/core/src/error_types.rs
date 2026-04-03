//! Shared error types and re-exports.
//!
//! This module provides the Result type alias.

/// Result type alias using our custom error
pub type Result<T> = std::result::Result<T, crate::error::Error>;
