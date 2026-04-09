//! Worktree type definitions
//!
//! This module provides `WorktreeInfo` - information about a Git worktree.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::vcs::errors::VcsError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub is_main: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
}

impl WorktreeInfo {
    pub fn new(path: PathBuf, is_main: bool, branch: Option<String>, head: Option<String>) -> Self {
        Self {
            path,
            is_main,
            branch,
            head,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_main(&self) -> bool {
        self.is_main
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_info_basic() {
        let wt = WorktreeInfo::new(
            PathBuf::from("/path/to/worktree"),
            false,
            Some("main".to_string()),
            Some("abc123".to_string()),
        );
        assert_eq!(wt.path(), Path::new("/path/to/worktree"));
        assert!(!wt.is_main());
        assert_eq!(wt.branch(), Some("main"));
        assert_eq!(wt.head(), Some("abc123"));
    }

    #[test]
    fn worktree_info_main() {
        let wt = WorktreeInfo::new(
            PathBuf::from("/path/to/repo"),
            true,
            Some("main".to_string()),
            Some("abc123".to_string()),
        );
        assert!(wt.is_main());
    }

    #[test]
    fn worktree_info_detached() {
        let wt = WorktreeInfo::new(
            PathBuf::from("/path/to/worktree"),
            false,
            None,
            Some("def456".to_string()),
        );
        assert!(!wt.is_main());
        assert_eq!(wt.branch(), None);
        assert_eq!(wt.head(), Some("def456"));
    }

    #[test]
    fn worktree_info_serde() {
        let wt = WorktreeInfo::new(
            PathBuf::from("/path/to/worktree"),
            false,
            Some("feature".to_string()),
            Some("abc123".to_string()),
        );
        let json = serde_json::to_string(&wt).expect("serialize");
        let deserialized: WorktreeInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(wt, deserialized);
    }
}
