//! File change tracking types
//!
//! Tracks modifications, additions, deletions, and renames.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileStatus {
    #[serde(rename = "M")]
    Modified,
    #[serde(rename = "A")]
    Added,
    #[serde(rename = "D")]
    Deleted,
    #[serde(rename = "R")]
    Renamed,
    #[serde(rename = "?")]
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl FileChange {
    pub fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::InvalidState(
                "Renamed files must have old_path set".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangesSummary {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub untracked: usize,
}

impl ChangesSummary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified + self.added + self.deleted + self.renamed > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    pub path: PathBuf,
    pub insertions: usize,
    pub deletions: usize,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
    pub files: Vec<FileDiffStat>,
}

impl DiffSummary {
    pub fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::InvalidState(format!(
                "files_changed ({}) does not match files array length ({})",
                self.files_changed,
                self.files.len()
            )));
        }
        Ok(())
    }
}
