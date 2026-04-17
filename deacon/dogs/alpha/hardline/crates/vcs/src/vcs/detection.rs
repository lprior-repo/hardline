//! VCS backend detection
//!
//! This module provides the `detect_backend` function for identifying the VCS type
//! at a given filesystem path.

use std::path::Path;

use super::errors::VcsError;
use super::BackendType;

// ============================================================================
// Detection Function
// ============================================================================

/// Detect the VCS backend type at a given path
///
/// # Preconditions
/// - Path must exist
/// - Path must be a directory
/// - `.git` must exist in path hierarchy
///
/// # Detection Order
/// - Checks for `.git` in path hierarchy
/// - Returns `NoVcsFound` if `.git` does not exist
///
/// # Errors
/// - `VcsError::PathNotFound` if path does not exist
/// - `VcsError::PathNotDirectory` if path is not a directory
/// - `VcsError::NoVcsFound` if no VCS detected in path hierarchy
pub fn detect_backend(path: impl AsRef<Path>) -> Result<BackendType, VcsError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(VcsError::PathNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(VcsError::PathNotDirectory(path.to_path_buf()));
    }

    path.ancestors()
        .find_map(|current| {
            if current.join(".git").exists() {
                Some(BackendType::Git)
            } else {
                None
            }
        })
        .ok_or_else(|| VcsError::NoVcsFound(path.to_path_buf()))
}
