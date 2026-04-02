//! Gitoxide Worktree Operations

use crate::error::{GitError, GitResult};
use std::path::PathBuf;

/// Add worktree
pub fn add(_repo: &gix::Repository, path: &PathBuf, _branch: Option<&str>) -> GitResult<()> {
    // Create the worktree directory
    std::fs::create_dir_all(path).map_err(GitError::Io)?;

    // Stub - worktree support is complex
    Err(GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "worktree add not fully implemented".to_string(),
    })
}

/// List worktrees
pub fn list(repo: &gix::Repository) -> GitResult<Vec<Worktree>> {
    let mut worktrees = Vec::new();

    // Add the main worktree
    if let Some(workdir) = repo.workdir() {
        worktrees.push(Worktree {
            path: workdir.to_path_buf(),
            is_main: true,
            branch: repo
                .head_name()
                .ok()
                .flatten()
                .map(|n| n.shorten().to_string()),
        });
    }

    Ok(worktrees)
}

/// Remove worktree
pub fn remove(_repo: &gix::Repository, _path: &PathBuf, _force: bool) -> GitResult<()> {
    // Stub
    Err(GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "worktree remove not implemented".to_string(),
    })
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub is_main: bool,
    pub branch: Option<String>,
}
