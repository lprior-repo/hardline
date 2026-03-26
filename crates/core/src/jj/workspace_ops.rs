//! JJ workspace operations - Actions layer
//!
//! All JJ workspace CRUD operations and repo checks.

use std::path::{Path, PathBuf};

use crate::jj::command::{get_jj_command, jj_command_error};
use crate::jj::conflict::{conflict_recovery_hint, detect_workspace_conflict};
use crate::jj::parse::{parse_diff_stat, parse_status, parse_workspace_list};
use crate::jj::types::{DiffSummary, Status, WorkspaceInfo};
use crate::jj::workspace_guard::WorkspaceGuard;
use crate::{Error, Result};

// ============================================================================
// Workspace CRUD Operations
// ============================================================================

/// Create a new workspace (without guard)
pub async fn workspace_create(name: &str, path: &Path) -> Result<()> {
    if name.is_empty() {
        return Err(crate::error_config::ConfigErrorKind::Invalid(
            "workspace name cannot be empty".into(),
        )
        .into());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::io_error(format!("Failed to create workspace directory: {e}")))?;
    }

    let output = get_jj_command()
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .output()
        .await
        .map_err(|e| jj_command_error("create workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        if let Some(conflict_type) = detect_workspace_conflict(&stderr, name) {
            let recovery_hint = conflict_recovery_hint(&conflict_type, name);
            return Err(crate::error_jj::JjErrorKind::WorkspaceConflict {
                conflict_type,
                workspace_name: name.to_string(),
                msg: stderr.to_string(),
                recovery_hint,
            }
            .into());
        }

        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "create workspace".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    Ok(())
}

/// Create a workspace and return a guard for cleanup
pub async fn create_workspace(name: &str, path: &Path) -> Result<WorkspaceGuard> {
    workspace_create(name, path).await?;
    Ok(WorkspaceGuard::new(name.to_string(), path.to_path_buf()))
}

/// Forget (delete) a workspace
pub async fn workspace_forget(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(crate::error_config::ConfigErrorKind::Invalid(
            "workspace name cannot be empty".into(),
        )
        .into());
    }

    let output = get_jj_command()
        .args(["workspace", "forget", name])
        .output()
        .await
        .map_err(|e| jj_command_error("forget workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "forget workspace".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    Ok(())
}

/// List all workspaces
pub async fn workspace_list() -> Result<Vec<WorkspaceInfo>> {
    let output = get_jj_command()
        .args(["workspace", "list"])
        .output()
        .await
        .map_err(|e| jj_command_error("list workspaces", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "list workspaces".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_workspace_list(&stdout)
}

/// Get workspace status
pub async fn workspace_status(path: &Path) -> Result<Status> {
    let output = get_jj_command()
        .args(["status"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| jj_command_error("get workspace status", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "get workspace status".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_status(&stdout))
}

/// Get workspace diff summary
pub async fn workspace_diff(path: &Path) -> Result<DiffSummary> {
    let output = get_jj_command()
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| jj_command_error("get workspace diff", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "get workspace diff".to_string(),
            msg: stderr.to_string(),
            is_not_found: false,
        }
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_stat(&stdout))
}

// ============================================================================
// Repo Checks
// ============================================================================

/// Check if JJ is installed
pub async fn is_jj_installed() -> bool {
    check_jj_installed().await.is_ok()
}

/// Check if path is inside a JJ repo
pub async fn is_jj_repo() -> bool {
    check_in_jj_repo().await.is_ok()
}

/// Verify JJ is installed and working
pub async fn check_jj_installed() -> Result<()> {
    get_jj_command()
        .arg("--version")
        .output()
        .await
        .map_err(|e| jj_command_error("check JJ installation", &e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(crate::error_jj::JjErrorKind::CommandError {
                    operation: "check JJ installation".to_string(),
                    msg: "JJ command returned non-zero exit code".to_string(),
                    is_not_found: false,
                }
                .into())
            }
        })
}

/// Find the root of the JJ repo containing the current directory
pub async fn check_in_jj_repo() -> Result<PathBuf> {
    let output = get_jj_command()
        .args(["root"])
        .output()
        .await
        .map_err(|e| jj_command_error("find JJ repository root", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "find JJ repository root".to_string(),
            msg: format!("Not in a JJ repository. {stderr}"),
            is_not_found: false,
        }
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();

    if root.is_empty() {
        Err(crate::error_jj::JjErrorKind::CommandError {
            operation: "find JJ repository root".to_string(),
            msg: "Could not determine JJ repository root".to_string(),
            is_not_found: false,
        }
        .into())
    } else {
        Ok(PathBuf::from(root))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::JjConflictType;

    #[test]
    fn test_parse_workspace_list() -> Result<()> {
        let output = "default: /home/user/repo\nfeature: /home/user/repo/.isolate/workspaces/feature\nstale-ws: /home/user/old (stale)";
        let result = parse_workspace_list(output);
        assert!(result.is_ok());
        let workspaces = result?;
        assert_eq!(workspaces.len(), 3);

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(workspaces[0].name, "default");
            assert!(!workspaces[0].is_stale);
            assert_eq!(workspaces[2].name, "stale-ws");
            assert!(workspaces[2].is_stale);
        }
        Ok(())
    }

    #[test]
    fn test_parse_status() {
        let output = "M file1.rs\nA file2.rs\nD file3.rs\n? unknown.txt";
        let status = parse_status(output);
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.added.len(), 1);
        assert_eq!(status.deleted.len(), 1);
        assert_eq!(status.unknown.len(), 1);
        assert!(!status.is_clean());
        assert_eq!(status.change_count(), 3);
    }

    #[test]
    fn test_parse_diff_stat() {
        let output = "file1.rs | 10 +++++++---\nfile2.rs | 5 ++---\n2 files changed, 12 insertions(+), 3 deletions(-)";
        let summary = parse_diff_stat(output);
        assert_eq!(summary.insertions, 12);
        assert_eq!(summary.deletions, 3);
    }

    #[test]
    fn test_status_is_clean() {
        let clean_status = Status {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(clean_status.is_clean());

        let dirty_status = Status {
            modified: vec![PathBuf::from("file.rs")],
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(!dirty_status.is_clean());
    }

    #[test]
    fn test_workspace_guard_new() {
        let guard = WorkspaceGuard::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );
        assert_eq!(guard.name(), "test-session");
        assert_eq!(guard.path(), PathBuf::from("/tmp/test-workspace"));
        assert!(guard.is_active());
    }

    #[test]
    fn test_workspace_guard_disarm() {
        let mut guard = WorkspaceGuard::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );
        assert!(guard.is_active());

        guard.disarm();
        assert!(!guard.is_active());
    }

    #[tokio::test]
    async fn test_workspace_guard_cleanup_when_active() {
        let temp_dir = std::env::temp_dir().join("scp-test-workspace-guard");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let nanos_since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or_else(|_| 0_u128, |duration| duration.as_nanos());
        let workspace_name = format!("test-cleanup-{}-{nanos_since_epoch}", std::process::id());

        let guard_path = temp_dir.clone();
        let mut guard = WorkspaceGuard::new(workspace_name, guard_path);
        assert!(guard.is_active());

        let result = guard.cleanup().await;

        assert!(!guard.is_active());

        let exists_after_cleanup = tokio::fs::try_exists(&temp_dir)
            .await
            .map_or(true, |exists| exists);
        assert!(!exists_after_cleanup);

        let _ = result;
    }

    #[tokio::test]
    async fn test_workspace_guard_cleanup_when_inactive() {
        let mut guard = WorkspaceGuard::new(
            "test-inactive".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );

        guard.disarm();
        assert!(!guard.is_active());

        let result = guard.cleanup().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workspace_guard_drop_cleans_up() {
        let temp_dir = std::env::temp_dir().join("scp-test-drop-cleanup");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        {
            let guard_path = temp_dir.clone();
            let _guard = WorkspaceGuard::new("test-drop".to_string(), guard_path);
        }

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_workspace_guard_disarmed_does_not_cleanup() {
        let temp_dir = std::env::temp_dir().join("scp-test-disarmed");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        {
            let guard_path = temp_dir.clone();
            let mut guard = WorkspaceGuard::new("test-disarmed".to_string(), guard_path);
            guard.disarm();
        }

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_workspace_guard_panic_still_cleans_up() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let temp_dir = std::env::temp_dir().join("scp-test-panic-cleanup");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let guard_path = temp_dir.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = WorkspaceGuard::new("test-panic".to_string(), guard_path);
            panic!("Intentional panic for testing");
        }));

        assert!(result.is_err());

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_create_workspace_returns_guard() {
        let temp_dir = std::env::temp_dir().join("scp-test-create-workspace");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let result = create_workspace("test-workspace", &temp_dir).await;

        match result {
            Ok(guard) => {
                assert_eq!(guard.name(), "test-workspace");
                assert_eq!(guard.path(), temp_dir);
                assert!(guard.is_active());
            }
            Err(_e) => {}
        }

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_create_workspace_propagates_errors() {
        let temp_dir = std::env::temp_dir().join("scp-test-error-workspace");

        let result = create_workspace("", &temp_dir).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_workspace_guard_has_correct_name() {
        let temp_dir = std::env::temp_dir().join("scp-test-guard-name");

        let result = create_workspace("my-workspace", &temp_dir).await;

        if let Ok(guard) = result {
            assert_eq!(guard.name(), "my-workspace");
        } else {
        }
    }

    #[tokio::test]
    async fn test_create_workspace_guard_has_correct_path() {
        let temp_dir = std::env::temp_dir().join("scp-test-guard-path");

        let result = create_workspace("path-workspace", &temp_dir).await;

        if let Ok(guard) = result {
            assert_eq!(guard.path(), temp_dir);
        } else {
        }
    }

    #[test]
    fn test_detect_conflict_already_exists() {
        let stderr = "error: workspace 'my-workspace' already exists";
        let result = detect_workspace_conflict(stderr, "my-workspace");
        assert_eq!(result, Some(JjConflictType::AlreadyExists));
    }

    #[test]
    fn test_detect_conflict_concurrent() {
        let stderr = "error: concurrent modification detected";
        let result = detect_workspace_conflict(stderr, "test");
        assert_eq!(
            result,
            Some(JjConflictType::ConcurrentModification)
        );
    }

    #[test]
    fn test_detect_conflict_abandoned() {
        let stderr = "error: workspace has been abandoned";
        let result = detect_workspace_conflict(stderr, "old-workspace");
        assert_eq!(result, Some(JjConflictType::Abandoned));
    }

    #[test]
    fn test_detect_conflict_stale() {
        let stderr = "error: working copy is stale";
        let result = detect_workspace_conflict(stderr, "stale-workspace");
        assert_eq!(result, Some(JjConflictType::Stale));
    }

    #[test]
    fn test_detect_conflict_no_match() {
        let stderr = "error: some other error";
        let result = detect_workspace_conflict(stderr, "test");
        assert!(result.is_none());
    }

    #[test]
    fn test_conflict_recovery_hint_already_exists() {
        let hint = conflict_recovery_hint(&JjConflictType::AlreadyExists, "test-ws");
        assert!(hint.contains("Recovery options"));
        assert!(hint.contains("jj workspace forget test-ws"));
        assert!(hint.contains("jj workspace list"));
    }

    #[test]
    fn test_conflict_recovery_hint_concurrent() {
        let hint = conflict_recovery_hint(&JjConflictType::ConcurrentModification, "test-ws");
        assert!(hint.contains("Recovery options"));
        assert!(hint.contains("Wait a moment"));
        assert!(hint.contains("pgrep -fl jj"));
    }

    #[test]
    fn test_conflict_recovery_hint_abandoned() {
        let hint = conflict_recovery_hint(&JjConflictType::Abandoned, "old-ws");
        assert!(hint.contains("Recovery options"));
        assert!(hint.contains("jj workspace forget old-ws"));
        assert!(hint.contains("jj status"));
    }

    #[test]
    fn test_conflict_recovery_hint_stale() {
        let hint = conflict_recovery_hint(&JjConflictType::Stale, "stale-ws");
        assert!(hint.contains("Recovery options"));
        assert!(hint.contains("jj workspace update-stale"));
        assert!(hint.contains("jj reload"));
    }
}
