use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::WorkspaceError;

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

fn is_absolute_path(path_buf: &std::path::Path) -> bool {
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

    pub const fn as_path(&self) -> &PathBuf {
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

    #[test]
    fn workspace_path_absolute_returns_same_path() {
        let path = WorkspacePath::new("/home/user/workspace".into()).unwrap();
        assert!(path.as_path().is_absolute());
        assert!(path.as_str().unwrap().starts_with("/"));
    }

    #[test]
    fn workspace_path_relative_resolves_from_cwd() {
        // Relative paths get resolved to absolute via cwd
        let path = WorkspacePath::new("relative/path".into()).unwrap();
        assert!(path.as_path().is_absolute());
    }

    #[test]
    fn workspace_path_as_str_for_valid_utf8() {
        let path = WorkspacePath::new("/tmp/workspace".into()).unwrap();
        assert_eq!(path.as_str(), Some("/tmp/workspace"));
    }

    #[test]
    fn workspace_path_exists_for_tmp() {
        let path = WorkspacePath::new("/tmp".into()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn workspace_path_is_dir_for_tmp() {
        let path = WorkspacePath::new("/tmp".into()).unwrap();
        assert!(path.is_dir());
    }

    #[test]
    fn workspace_path_not_exists_for_random_path() {
        let path = WorkspacePath::new("/tmp/nonexistent_workspace_xyz_123".into()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn workspace_path_clone() {
        let path = WorkspacePath::new("/tmp/clone-test".into()).unwrap();
        let path2 = path.clone();
        assert_eq!(path, path2);
    }

    #[test]
    fn workspace_path_equality() {
        let a = WorkspacePath::new("/tmp/same".into()).unwrap();
        let b = WorkspacePath::new("/tmp/same".into()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn workspace_path_serialization_roundtrip() {
        let path = WorkspacePath::new("/tmp/serde-test".into()).unwrap();
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: WorkspacePath = serde_json::from_str(&json).unwrap();
        assert_eq!(path, deserialized);
    }

    #[test]
    fn workspace_path_debug_format() {
        let path = WorkspacePath::new("/tmp/debug-test".into()).unwrap();
        let debug_str = format!("{path:?}");
        assert!(debug_str.contains("/tmp/debug-test"));
    }

    #[test]
    fn workspace_path_multiple_instances_of_same_path_are_equal() {
        let a = WorkspacePath::new("/tmp/hash-test".into()).unwrap();
        let b = WorkspacePath::new("/tmp/hash-test".into()).unwrap();
        assert_eq!(a, b);
    }

    // --- Additional unit tests ---

    #[test]
    fn workspace_path_different_paths_not_equal() {
        let a = WorkspacePath::new("/tmp/a".into()).unwrap();
        let b = WorkspacePath::new("/tmp/b".into()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn workspace_path_not_is_dir_for_file() {
        let path = WorkspacePath::new("/etc/hosts".into()).unwrap();
        // /etc/hosts is a file, not a dir
        assert!(!path.is_dir());
    }

    #[test]
    fn workspace_path_empty_error_type() {
        let result = WorkspacePath::new("".into());
        match result.err() {
            Some(WorkspaceError::InvalidWorkspacePath(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidWorkspacePath, got {other:?}"),
        }
    }

    #[test]
    fn workspace_path_deep_path() {
        let path = WorkspacePath::new("/a/b/c/d/e/f/g".into()).unwrap();
        assert!(path.as_path().is_absolute());
    }

    #[test]
    fn workspace_path_with_trailing_slash() {
        let path = WorkspacePath::new("/tmp/trailing/".into()).unwrap();
        assert!(path.as_path().is_absolute());
    }

    #[test]
    fn workspace_path_home_directory() {
        let path = WorkspacePath::new("/home".into()).unwrap();
        assert!(path.is_dir());
        assert!(path.exists());
    }

    #[test]
    fn workspace_path_serialization_contains_path() {
        let path = WorkspacePath::new("/tmp/serde-content".into()).unwrap();
        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("/tmp/serde-content"));
    }

    #[test]
    fn workspace_path_display_not_implemented_but_debug_works() {
        let path = WorkspacePath::new("/tmp/debug2".into()).unwrap();
        let debug_str = format!("{path:?}");
        assert!(debug_str.contains("/tmp"));
    }

    #[test]
    fn workspace_path_dot_relative() {
        let path = WorkspacePath::new(".".into()).unwrap();
        assert!(path.as_path().is_absolute()); // gets resolved to cwd
    }

    #[test]
    fn workspace_path_dotdot_relative() {
        let path = WorkspacePath::new("..".into()).unwrap();
        assert!(path.as_path().is_absolute()); // gets resolved to parent of cwd
    }

    #[test]
    fn workspace_path_is_not_file() {
        let path = WorkspacePath::new("/tmp".into()).unwrap();
        // /tmp is a directory
        assert!(path.is_dir());
        // No "is_file" method, but we can check it exists and is_dir
        assert!(path.exists());
    }

    #[test]
    fn workspace_path_relative_subdirectory() {
        let path = WorkspacePath::new("relative/sub/dir".into()).unwrap();
        assert!(path.as_path().is_absolute());
        assert!(path.as_str().unwrap().ends_with("relative/sub/dir"));
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::{prelude::*, prop_assert, prop_assert_eq};

        use super::*;

        proptest! {
            #[test]
            fn workspace_path_absolute_always_succeeds(
                segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
            ) {
                let path_str = format!("/{}", segments.join("/"));
                let result = WorkspacePath::new(path_str);
                prop_assert!(result.is_ok());
                prop_assert!(result.unwrap().as_path().is_absolute());
            }

            #[test]
            fn workspace_path_relative_gets_resolved(
                segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
            ) {
                let path_str = segments.join("/");
                let result = WorkspacePath::new(path_str);
                prop_assert!(result.is_ok());
                // Relative paths get resolved to absolute
                prop_assert!(result.unwrap().as_path().is_absolute());
            }

            #[test]
            fn workspace_path_empty_always_fails(s in ".{0}") {
                let result = WorkspacePath::new(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_path_serialization_roundtrip(
                segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
            ) {
                let path_str = format!("/{}", segments.join("/"));
                let path = WorkspacePath::new(path_str).unwrap();
                let json = serde_json::to_string(&path).unwrap();
                let deserialized: WorkspacePath = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(path, deserialized);
            }

            #[test]
            fn workspace_path_equality_for_same_absolute(
                segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
            ) {
                let path_str = format!("/{}", segments.join("/"));
                let a = WorkspacePath::new(path_str.clone()).unwrap();
                let b = WorkspacePath::new(path_str).unwrap();
                prop_assert_eq!(a, b);
            }
        }
    }
}
