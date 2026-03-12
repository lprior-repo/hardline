use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePath(PathBuf);

impl WorkspacePath {
    pub fn new(path: String) -> Result<Self, WorkspaceError> {
        if path.is_empty() {
            return Err(WorkspaceError::InvalidWorkspacePath("empty path".into()));
        }
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            Ok(Self(path_buf))
        } else {
            Ok(Self(
                std::env::current_dir()
                    .map_err(|e| WorkspaceError::InvalidWorkspacePath(e.to_string()))?
                    .join(path_buf),
            ))
        }
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap_or("")
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
