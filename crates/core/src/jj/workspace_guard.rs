//! Workspace guard - RAII cleanup for workspaces
//!
//! Provides automatic cleanup when workspaces go out of scope.

use std::path::{Path, PathBuf};

use crate::jj::command::{get_jj_command_sync, jj_command_error};
use crate::{Error, Result};

/// Guard that automatically cleans up a workspace on drop
pub struct WorkspaceGuard {
    name: String,
    path: PathBuf,
    active: bool,
}

impl WorkspaceGuard {
    /// Create a new workspace guard
    #[must_use]
    pub const fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            active: true,
        }
    }

    /// Disarm the guard (prevent automatic cleanup)
    pub const fn disarm(&mut self) {
        self.active = false;
    }

    /// Check if guard is active
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Get workspace name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get workspace path
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Async cleanup of the workspace
    pub async fn cleanup(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        self.active = false;
        let forget_result = crate::jj::workspace_ops::workspace_forget(&self.name).await;

        let remove_result = match tokio::fs::try_exists(&self.path).await {
            Ok(true) => tokio::fs::remove_dir_all(&self.path).await.map_err(|e| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remove workspace directory: {e}"),
                ))
            }),
            Ok(false) => Ok(()),
            Err(e) => Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to check workspace existence: {e}"),
            ))),
        };

        forget_result.and(remove_result)
    }

    /// Synchronous cleanup
    fn perform_cleanup_sync(&self) -> Result<()> {
        let forget_result = get_jj_command_sync()
            .args(["workspace", "forget", &self.name])
            .output()
            .map_err(|e| jj_command_error("forget workspace", &e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(Error::JjCommandError {
                        operation: "forget workspace".to_string(),
                        msg: stderr.to_string(),
                        is_not_found: false,
                    })
                }
            });

        let remove_result = match self.path.try_exists() {
            Ok(true) => std::fs::remove_dir_all(&self.path).map_err(|e| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remove workspace directory: {e}"),
                ))
            }),
            Ok(false) => Ok(()),
            Err(e) => Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to check workspace existence: {e}"),
            ))),
        };

        forget_result.and(remove_result)
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        if let Err(e) = self.perform_cleanup_sync() {
            tracing::warn!("Workspace cleanup failed for '{}': {e}", self.name);
            eprintln!(
                "Warning: Failed to cleanup workspace '{}': {}",
                self.name, e
            );
        }
    }
}
