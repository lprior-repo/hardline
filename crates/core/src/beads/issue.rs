//! Issue aggregate root.
//!
//! This module defines the `Issue` aggregate root which encapsulates
//! the domain logic for issue management.
//!
//! # Modules
//!
//! - `issue_data`: Issue struct definition (data)
//! - `issue_methods`: Issue methods (constructors, state transitions, field updates)
//! - `issue_builder`: IssueBuilder for constructing issues
//! - `issue_tests`: Unit tests

// Use path attributes to locate submodules in the same directory (beads/)
// instead of in a subdirectory (issue/).
// This allows us to split the file without creating a new directory.
#[path = "issue_data.rs"]
mod issue_data;
#[path = "issue_methods.rs"]
mod issue_methods;
#[path = "issue_builder.rs"]
mod issue_builder;

#[cfg(test)]
#[path = "issue_tests.rs"]
mod issue_tests;

// Re-export for convenience
pub use issue_data::Issue;
pub use issue_builder::IssueBuilder;
