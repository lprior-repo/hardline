use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePath(PathBuf);

fn validate_non_empty_path(path: &str) -> Result<(), WorkspaceError> {
    if path.is_empty() {
        Err(WorkspaceError::InvalidWorkspacePath("empty path".into()))
    } else {
        Ok(())
    }
}

fn to_path_buf(path: &str) -> PathBuf {
    PathBuf::from(path)
}

fn is_absolute_path(path_buf: &Path) -> bool {
    path_buf.is_absolute()
}

fn resolve_relative(path_buf: PathBuf) -> Result<PathBuf, WorkspaceError> {
    std::env::current_dir()
        .map_err(|e| WorkspaceError::InvalidWorkspacePath(e.to_string()))
        .map(|cwd| cwd.join(path_buf))
}

fn resolve_path(path_buf: PathBuf) -> Result<PathBuf, WorkspaceError> {
    if is_absolute_path(&path_buf) {
        Ok(path_buf)
    } else {
        resolve_relative(path_buf)
    }
}

impl WorkspacePath {
    pub fn new(path: String) -> Result<Self, WorkspaceError> {
        validate_non_empty_path(&path)?;
        let path_buf = to_path_buf(&path);
        resolve_path(path_buf).map(Self)
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.to_str()
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    pub fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_path_valid() {
        let path = WorkspacePath::new("/tmp/workspace".into());
        assert!(path.is_ok());
    }

    #[test]
    fn workspace_path_empty_fails() {
        let path = WorkspacePath::new("".into());
        assert!(path.is_err());
    }
}
