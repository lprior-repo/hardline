//! Gitoxide Stash Operations

use crate::error::{GitError, GitResult};

/// List stashes
pub fn list(_repo: &gix::Repository) -> GitResult<Vec<StashEntry>> {
    Err(GitError::InvalidRef {
        name: "list".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Save stash
pub fn save(
    _repo: &gix::Repository,
    _message: Option<&str>,
    _include_untracked: bool,
) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "save".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Pop stash
pub fn pop(_repo: &gix::Repository, _index: usize) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "pop".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Drop stash
pub fn drop(_repo: &gix::Repository, _index: usize) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "drop".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Show stash
pub fn show(_repo: &gix::Repository, _index: usize) -> GitResult<String> {
    Err(GitError::InvalidRef {
        name: "show".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}
