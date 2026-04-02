//! JJ operation tracking and workspace creation
//!
//! Provides operation info retrieval and synchronized workspace creation.

#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(unused)]

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

use super::jj_lock::acquire_cross_process_lock;
use super::jj_path::get_jj_command;

/// Current repository operation information
#[derive(Debug, Clone)]
pub struct RepoOperationInfo {
    /// The operation ID
    pub operation_id: String,
    /// The repository root path
    pub repo_root: PathBuf,
}

/// Get the current operation ID and repo root for a working copy
///
/// # Errors
///
/// Returns an error if the `jj` command fails or returns invalid output.
pub async fn get_current_operation(root: &Path) -> Result<RepoOperationInfo> {
    let output = get_jj_command()
        .args(["op", "log", "--no-graph", "--limit", "1", "-T", "id"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| crate::error_jj::JjErrorKind::CommandError {
            operation: "get current operation".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "get current operation".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    let operation_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if operation_id.is_empty() {
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "get current operation".to_string(),
            msg: "Empty operation ID returned".to_string(),
            is_not_found: false,
        }
        .into());
    }

    let root_output = get_jj_command()
        .args(["root"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| crate::error_jj::JjErrorKind::CommandError {
            operation: "get repo root".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !root_output.status.success() {
        let stderr = String::from_utf8_lossy(&root_output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "get repo root".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    let repo_root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_string();

    Ok(RepoOperationInfo {
        operation_id,
        repo_root: PathBuf::from(repo_root),
    })
}

/// Create a workspace with cross-process and in-process locking to prevent
/// operation graph corruption from concurrent creations.
///
/// # Errors
///
/// Returns an error if the workspace name is empty, locking fails, or the
/// `jj workspace add` command fails.
pub async fn create_workspace_synced(name: &str, path: &Path, repo_root: &Path) -> Result<()> {
    if name.is_empty() {
        return Err(crate::error_config::ConfigErrorKind::Invalid(
            "workspace name cannot be empty".into(),
        )
        .into());
    }

    let _lock = super::jj_lock::acquire_lock_with_backoff().await?;

    let _cross_process_lock = acquire_cross_process_lock(repo_root).await?;

    // Create .isolate data directory WHILE holding the cross-process lock.
    // This eliminates the TOCTOU race where .isolate was created before
    // the lock was acquired (phantom directory on crash).
    super::jj_lock::ensure_data_directory(repo_root).await?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::io_error(format!("Failed to create workspace directory: {e}")))?;
    }

    let _ = get_current_operation(repo_root).await?;

    let output = get_jj_command()
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .current_dir(repo_root)
        .output()
        .await
        .map_err(|e| crate::error_jj::JjErrorKind::CommandError {
            operation: "create workspace".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "create workspace".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    super::jj_workspace::verify_workspace_consistency(name, path).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::significant_drop_tightening,
        clippy::unnecessary_cast,
        clippy::assertions_on_constants,
        clippy::suspicious_open_options
    )]

    use super::*;

    #[test]
    fn given_repo_operation_info_when_cloned_then_deep_copy() {
        let info = RepoOperationInfo {
            operation_id: "abc123".into(),
            repo_root: PathBuf::from("/tmp/repo"),
        };
        let cloned = info;
        assert_eq!(cloned.operation_id, "abc123");
        assert_eq!(cloned.repo_root, PathBuf::from("/tmp/repo"));
    }

    #[test]
    fn given_repo_operation_info_when_formatted_then_shows_fields() {
        let info = RepoOperationInfo {
            operation_id: "xyz789".into(),
            repo_root: PathBuf::from("/test/path"),
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("xyz789"));
        assert!(debug_str.contains("/test/path"));
    }

    #[tokio::test]
    async fn test_empty_workspace_name_returns_error() {
        let temp_dir = std::env::temp_dir().join("test-empty-name");
        let repo_root = std::env::temp_dir().join("test-repo-root");
        let result = create_workspace_synced("", &temp_dir, &repo_root).await;

        match result {
            Err(Error::Config(crate::error_config::ConfigError { .. })) => {}
            Ok(()) => panic!("Expected Config error, but got Ok"),
            Err(other) => panic!("Expected Config error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_workspace_without_parent_returns_error() {
        let workspace_path = PathBuf::from("/");
        let repo_root = std::env::temp_dir().join("test-repo-root");
        let result = create_workspace_synced("test", &workspace_path, &repo_root).await;

        match result {
            Err(Error::Jj(crate::error_jj::JjError { .. })) => {}
            Err(Error::Config(crate::error_config::ConfigError { .. })) => {}
            Err(Error::Io(_)) => {}
            Err(other) => {
                panic!("Expected Jj, Config, or Io error, got: {other:?}")
            }
            Ok(()) => {
                panic!("Expected error when workspace path has no parent, but got Ok")
            }
        }
    }
}
