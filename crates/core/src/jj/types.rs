//! JJ Workspace types - Data layer
//!
//! Pure domain types for JJ workspace representation.

use std::path::PathBuf;

/// Information about a JJ workspace
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace name
    pub name: String,
    /// Absolute path to workspace
    pub path: PathBuf,
    /// Whether the workspace is stale
    pub is_stale: bool,
}

/// Summary of changes in a diff
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

/// Status of a JJ workspace
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Modified files
    pub modified: Vec<PathBuf>,
    /// Added files
    pub added: Vec<PathBuf>,
    /// Deleted files
    pub deleted: Vec<PathBuf>,
    /// Renamed files (old path, new path)
    pub renamed: Vec<(PathBuf, PathBuf)>,
    /// Unknown/tracked files
    pub unknown: Vec<PathBuf>,
}

impl Status {
    /// Check if workspace has no changes
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.renamed.is_empty()
    }

    /// Total number of changes
    #[must_use]
    pub const fn change_count(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len() + self.renamed.len()
    }
}
