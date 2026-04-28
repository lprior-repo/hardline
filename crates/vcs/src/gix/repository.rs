//! Gitoxide Repository Operations
use std::path::PathBuf;

use crate::error::{GitError, GitResult};

/// Open an existing git repository at the given path.
pub fn open(path: impl Into<PathBuf>) -> GitResult<gix::Repository> {
    let path = path.into();
    gix::discover(&path).map_err(GitError::GixDiscover)
}

/// Initialize a new git repository at the given path.
pub fn init(path: impl Into<PathBuf>) -> GitResult<gix::Repository> {
    let path = path.into();
    gix::init(&path).map_err(GitError::GixInit)
}

/// Open an existing repository or initialize a new one if none exists.
pub fn open_or_init(path: impl Into<PathBuf>) -> GitResult<gix::Repository> {
    let path = path.into();
    open(&path).or_else(|_| init(&path))
}

/// Get the working directory path of a repository.
pub fn workdir(repo: &gix::Repository) -> Option<PathBuf> {
    repo.workdir().map(|p| p.to_path_buf())
}
