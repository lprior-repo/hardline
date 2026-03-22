//! Git backend implementation using gix (pure Rust)
//!
//! This module provides:
//! - `GitBackend` - VCS backend implementation using gix (pure Rust)
//! - `GitBackendConfig` - Configuration for `GitBackend` creation
//!
//! # Design
//! - Uses gix for ALL operations (status, branches, commits, checkout, rebase)
//! - No shell commands - 100% pure Rust via gix
//! - Caches the `gix::Repository` handle for performance
//! - Thread-safe for read operations via Mutex

pub mod backend;
pub mod helpers;
pub mod open;
pub mod sync;
pub mod tests;
pub mod types;

// Re-export types for convenience
pub use types::{GitBackend, GitBackendConfig};
