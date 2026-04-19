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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_path_new_unchecked() {
        let path = PathBuf::from("/some/path");
        let rp = RepositoryPath::new_unchecked(path.clone());
        assert_eq!(rp.as_path(), path);
    }

    #[test]
    fn repository_path_clone() {
        let rp = RepositoryPath::new_unchecked(PathBuf::from("/test"));
        let cloned = rp.clone();
        assert_eq!(rp, cloned);
    }

    #[test]
    fn repository_path_eq() {
        let a = RepositoryPath::new_unchecked(PathBuf::from("/a"));
        let b = RepositoryPath::new_unchecked(PathBuf::from("/a"));
        let c = RepositoryPath::new_unchecked(PathBuf::from("/c"));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn repository_path_hash() {
        use std::collections::HashSet;
        let a = RepositoryPath::new_unchecked(PathBuf::from("/same"));
        let b = RepositoryPath::new_unchecked(PathBuf::from("/same"));
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn repository_path_debug() {
        let rp = RepositoryPath::new_unchecked(PathBuf::from("/debug"));
        let debug = format!("{rp:?}");
        assert!(debug.contains("/debug"));
    }

    #[test]
    fn repository_path_serde_roundtrip() {
        let rp = RepositoryPath::new_unchecked(PathBuf::from("/repo/project"));
        let json = serde_json::to_string(&rp).expect("serialize");
        let deserialized: RepositoryPath = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rp, deserialized);
    }

    #[test]
    fn repository_path_serde_contains_path() {
        let rp = RepositoryPath::new_unchecked(PathBuf::from("/home/user/code"));
        let json = serde_json::to_string(&rp).expect("serialize");
        assert!(json.contains("/home/user/code"));
    }

    #[test]
    fn repository_path_serde_preserves_unicode_path() {
        let rp = RepositoryPath::new_unchecked(PathBuf::from("/home/用户/项目"));
        let json = serde_json::to_string(&rp).expect("serialize");
        let deserialized: RepositoryPath = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rp, deserialized);
    }
}
