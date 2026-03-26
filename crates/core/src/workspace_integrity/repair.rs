//! Workspace repair operations
//!
//! Provides RepairExecutor for fixing workspace issues.

use std::path::Path;

use crate::workspace_integrity::backup::BackupManager;
use crate::workspace_integrity::types::RepairStrategy;
use crate::workspace_integrity::validation_result::ValidationResult;
use crate::workspace_integrity::repair_result::RepairResult;
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR EXECUTOR
// ═══════════════════════════════════════════════════════════════════════════

/// Executes repairs for detected integrity issues
///
/// # ADVERSARIAL DEFENSE
///
/// The executor enforces consistency: if `always_backup` is true, a `BackupManager`
/// MUST be provided. This prevents runtime errors where repairs fail due to missing
/// backup configuration.
#[derive(Clone)]
pub struct RepairExecutor {
    /// Backup configuration
    backup_config: BackupConfig,
}

/// Backup configuration for repair operations
#[derive(Clone)]
enum BackupConfig {
    /// No backups - repair operations are destructive
    NoBackup,
    /// Always backup before repair
    WithBackup(BackupManager),
}

impl RepairExecutor {
    /// Create a new repair executor with default safety (no backups)
    ///
    /// NOTE: Defaults to NO BACKUP for safety. Use `with_backup_manager()`
    /// to enable backups before destructive operations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            backup_config: BackupConfig::NoBackup,
        }
    }

    /// Enable backups with a backup manager
    ///
    /// This is the RECOMMENDED way to create a repair executor for production use.
    /// Backups protect against data loss during repair operations.
    #[must_use]
    pub fn with_backup_manager(mut self, backup_manager: BackupManager) -> Self {
        self.backup_config = BackupConfig::WithBackup(backup_manager);
        self
    }

    /// Disable backups explicitly (for testing or trusted environments)
    ///
    /// WARNING: Without backups, repair operations are destructive and cannot
    /// be rolled back. Use with caution.
    #[must_use]
    pub fn without_backup(mut self) -> Self {
        self.backup_config = BackupConfig::NoBackup;
        self
    }

    /// Check if this executor creates backups before repair
    #[must_use]
    pub const fn creates_backups(&self) -> bool {
        matches!(self.backup_config, BackupConfig::WithBackup(_))
    }

    /// Execute repair (legacy name for compatibility)
    pub async fn execute(
        &self,
        _workspace_name: &str,
        _workspace_path: &Path,
        validation: &ValidationResult,
        _strategy: RepairStrategy,
    ) -> Result<RepairResult> {
        self.repair(validation).await
    }

    /// Repair a workspace based on validation results
    pub async fn repair(&self, validation: &ValidationResult) -> Result<RepairResult> {
        if validation.is_valid {
            return Ok(RepairResult::success(
                &validation.workspace,
                RepairStrategy::NoRepair,
                "Workspace is already healthy",
            ));
        }

        // Determine the overall repair strategy
        // We pick the most aggressive (highest risk) strategy among all issues
        let strategy = validation
            .issues
            .iter()
            .map(|i| i.recommended_strategy)
            .max_by_key(|s| match s {
                RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible => 0,
                RepairStrategy::ClearLocks => 1,
                RepairStrategy::FixJjDir => 2,
                RepairStrategy::RecreateWorkspace => 3,
                RepairStrategy::ForgetAndRecreate => 4,
            })
            .ok_or_else(|| Error::invalid_state("No issues found in validation result".to_string()))?;

        if matches!(
            strategy,
            RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible
        ) {
            return Ok(RepairResult::failure(
                &validation.workspace,
                RepairStrategy::NoRepair,
                "No automated repair possible for detected issues",
            ));
        }

        // CRITICAL: Check if workspace directory exists before attempting repair
        // For missing directories, we cannot repair automatically
        let workspace_exists = tokio::fs::try_exists(&validation.path).await.map_err(|e| Error::io_error(e.to_string()))?;
        if !workspace_exists {
            return Ok(RepairResult::failure(
                &validation.workspace,
                strategy,
                format!(
                    "Workspace directory '{}' does not exist. Cannot repair missing workspace.",
                    validation.path.display()
                ),
            ));
        }

        // Create backup if configured (ADVERSARIAL: type-safe backup guarantee)
        let backup_id = match &self.backup_config {
            BackupConfig::WithBackup(manager) => {
                let meta = manager
                    .create_backup(&validation.workspace, "Auto-repair")
                    .await?;
                Some(meta.id)
            }
            BackupConfig::NoBackup => None,
        };

        // Execute the repair
        let result = match strategy {
            RepairStrategy::ClearLocks => {
                Self::clear_locks(&validation.path).await.map(|()| {
                    RepairResult::success(
                        &validation.workspace,
                        strategy,
                        "Cleared stale lock files",
                    )
                })
            }
            RepairStrategy::ForgetAndRecreate => {
                Self::forget_and_recreate(&validation.workspace, &validation.path).await
            }
            _ => {
                // Not fully implemented yet
                Ok(RepairResult::failure(
                    &validation.workspace,
                    strategy,
                    format!("Repair strategy '{strategy}' not yet implemented"),
                ))
            }
        }?;

        Ok(if let Some(id) = backup_id {
            result.with_backup(id)
        } else {
            result
        })
    }

    /// Clear lock files in a workspace
    ///
    /// ADVERSARIAL: Idempotent operation - safe to call multiple times even if
    /// locks were already removed by another process. This prevents race conditions
    /// in concurrent repair scenarios.
    async fn clear_locks(workspace_path: &Path) -> Result<()> {
        let lock_file = workspace_path.join(".jj").join("working_copy").join("lock");

        // Try to remove the lock file, ignoring "not found" errors (idempotent)
        let result = tokio::fs::remove_file(&lock_file).await;

        match result {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File already removed by another process - OK
                Ok(())
            }
            Err(e) => Err(Error::io_error(format!(
                "Failed to remove lock file {}: {e}",
                lock_file.display()
            ))),
        }
    }

    /// Forget workspace in JJ and recreate
    async fn forget_and_recreate(
        workspace_name: &str,
        workspace_path: &Path,
    ) -> Result<RepairResult> {
        use crate::jj::get_jj_command;

        let root = workspace_path
            .parent()
            .and_then(|p| p.parent()) // .isolate/workspaces -> root
            .ok_or_else(|| Error::invalid_state("Could not determine repository root".to_string()))?;

        // Forget the workspace
        let forget_output = get_jj_command()
            .args(["workspace", "forget", workspace_name])
            .current_dir(root)
            .output()
            .await
            .map_err(|e| Error::from(crate::error_jj::JjErrorKind::CommandError {
                operation: "forget workspace".to_string(),
                msg: format!("Failed to forget workspace: {e}"),
                is_not_found: false,
            }))?;

        if !forget_output.status.success() {
            let stderr = String::from_utf8_lossy(&forget_output.stderr);
            return Ok(RepairResult::failure(
                workspace_name,
                RepairStrategy::ForgetAndRecreate,
                format!("Failed to forget workspace: {stderr}"),
            ));
        }

        // If directory is corrupted but exists, remove it
        let workspace_exists = tokio::fs::try_exists(workspace_path).await.map_err(|e| Error::io_error(e.to_string()))?;
        if workspace_exists {
            tokio::fs::remove_dir_all(workspace_path)
                .await
                .map_err(|e| {
                    Error::io_error(format!(
                        "Failed to remove corrupted workspace directory {}: {e}",
                        workspace_path.display()
                    ))
                })?;
        }

        Ok(RepairResult::success(
            workspace_name,
            RepairStrategy::ForgetAndRecreate,
            "Workspace forgotten and directory removed. Re-run 'isolate spawn' to recreate.",
        ))
    }
}

impl Default for RepairExecutor {
    fn default() -> Self {
        Self::new()
    }
}
