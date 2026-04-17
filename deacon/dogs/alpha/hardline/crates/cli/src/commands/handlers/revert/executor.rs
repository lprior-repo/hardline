//! Git command executor re-export for the revert handler.
//!
//! Re-uses the GitExecutor trait and RealGitExecutor from the done handler
//! to avoid duplication and maintain consistency across handlers.

pub use crate::commands::handlers::done::executor::{ExecutorError, GitExecutor, RealGitExecutor};
