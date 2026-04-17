//! Repository path type
//!
//! This module provides `RepositoryPath` - absolute path to a version-controlled directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::vcs::errors::VcsError;

/// Absolute path to a VCS repository
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryPath(PathBuf);

impl RepositoryPath {
    /// Create from any path (converts to absolute)
    ///
    /// # Errors
    /// - `VcsError::PathNotFound` if path does not exist
    /// - `VcsError::PathNotDirectory` if path is not a directory
    pub fn new(path: impl AsRef<Path>) -> Result<Self, VcsError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(VcsError::PathNotFound(path.to_path_buf()));
        }

        if !path.is_dir() {
            return Err(VcsError::PathNotDirectory(path.to_path_buf()));
        }

        let canonical = path.canonicalize().map_err(|e| VcsError::CommandFailed {
            message: format!("Failed to canonicalize path: {}", path.display()),
            source: Some(e),
        })?;

        Ok(Self(canonical))
    }

    /// Create without validation (for testing only)
    #[must_use]
    pub const fn new_unchecked(path: PathBuf) -> Self {
        Self(path)
    }

    /// Get the path as a reference
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}
