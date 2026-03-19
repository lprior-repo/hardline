//! Gitoxide Status Operations

use crate::domain::value_objects::VcsStatus;
use crate::error::GitResult;
use std::path::PathBuf;

/// Get repository status using gix
pub fn status(repo: &::gix::Repository) -> GitResult<VcsStatus> {
    let platform = repo.status(gix::progress::Discard)?;

    // Use the index_worktree_iter to get index-to-worktree changes
    let mut iw_iter = platform.into_index_worktree_iter(None::<gix::bstr::BString>)?;

    // Just check if there are any items - any items mean changes
    let has_changes = iw_iter.next().is_some();

    if has_changes {
        Ok(VcsStatus::Dirty)
    } else {
        Ok(VcsStatus::Clean)
    }
}

/// Get detailed status
pub fn detailed_status(_repo: &::gix::Repository) -> GitResult<Vec<(PathBuf, StatusKind)>> {
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
