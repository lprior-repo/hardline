//! Git backend type definitions
//!
// #![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Mutex;

use gix::Repository;

use crate::vcs::{RepositoryPath, VcsError};

// Minimum required Git CLI version (major, minor)
const MIN_GIT_VERSION: (u32, u32) = (2, 38);

/// Git backend implementation using gix for read operations
///
/// # Invariants
/// - Repository is always a non-bare Git repository
/// - Repository path is absolute and canonical
/// - `gix::Repository` is opened once and cached
/// - Thread-safe for read operations via Mutex
pub struct GitBackend {
    /// Absolute path to the repository root
    path: RepositoryPath,
    /// Cached gix repository handle (wrapped in Mutex for thread safety)
    repo: Mutex<Repository>,
}

/// Configuration for `GitBackend` creation
#[derive(Debug, Clone)]
pub struct GitBackendConfig {
    /// Verify Git CLI version on open (default: true)
    pub verify_cli_version: bool,
}

impl Default for GitBackendConfig {
    fn default() -> Self {
        Self {
            verify_cli_version: true,
        }
    }
}
