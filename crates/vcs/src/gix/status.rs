//! Gitoxide Status Operations

use crate::domain::value_objects::VcsStatus;
use crate::error::GitResult;
use std::path::PathBuf;

/// Get repository status
pub fn status(repo: &gix::Repository) -> GitResult<VcsStatus> {
    // Check if repository has a working directory
    if repo.workdir().is_none() {
        return Ok(VcsStatus::Clean);
    }
    
    // For now, return Clean - a full implementation would compare with HEAD
    Ok(VcsStatus::Clean)
}

/// Get detailed status
pub fn detailed_status(_repo: &gix::Repository) -> GitResult<Vec<(PathBuf, StatusKind)>> {
    // Return empty for now
    Ok(Vec::new())
}

#[derive(Debug, Clone)]
pub enum StatusKind {
    Modified,
    Added,
    Deleted,
    Conflicted,
    Untracked,
    Ignored,
}
