//! Action functions for the integrity command handler (Tier 3).
//!
//! I/O operations that delegate to `scp_core::workspace_integrity` for
//! validation, repair, backup listing, and backup restoration.

use std::path::Path;

use scp_core::output::Output;
use scp_core::workspace_integrity::{BackupManager, IntegrityValidator, RepairExecutor};
use scp_core::{Error, Result};

use super::data::{
    BackupListResponse, IntegritySubcommand, RepairResponse, RestoreResponse, ValidationResponse,
};

/// Execute the integrity command with the given subcommand.
///
/// # Errors
///
/// Returns errors for workspace validation failures, repair failures,
/// or backup operation failures.
pub fn run_integrity(subcommand: &IntegritySubcommand) -> Result<()> {
    match subcommand {
        IntegritySubcommand::Validate { workspace } => {
            let response = run_validate(workspace)?;
            Output::info(&format!(
                "Workspace '{}' validation: {}",
                response.workspace,
                if response.is_valid { "PASS" } else { "FAIL" }
            ));
            if response.issue_count > 0 {
                Output::info(&format!("  Issues found: {}", response.issue_count));
            }
            Ok(())
        }
        IntegritySubcommand::Repair { workspace, force } => {
            let response = run_repair(workspace, *force)?;
            if response.success {
                Output::success(&format!(
                    "Workspace '{}' repaired: {}",
                    response.workspace, response.summary
                ));
            } else {
                Output::info(&format!(
                    "Workspace '{}' repair: {}",
                    response.workspace, response.summary
                ));
            }
            Ok(())
        }
        IntegritySubcommand::BackupList => {
            let response = run_backup_list()?;
            Output::info(&format!("Backups available: {}", response.count));
            Ok(())
        }
        IntegritySubcommand::BackupRestore { backup_id, force } => {
            let response = run_backup_restore(backup_id, *force)?;
            if response.success {
                Output::success(&format!(
                    "Workspace '{}' restored from backup '{}': {}",
                    response.workspace, response.backup_id, response.summary
                ));
            } else {
                return Err(Error::invalid_state(format!(
                    "Restore failed: {}",
                    response.summary
                )));
            }
            Ok(())
        }
    }
}

/// Validate workspace integrity.
fn run_validate(workspace: &str) -> Result<ValidationResponse> {
    let cwd = std::env::current_dir()?;
    let validator = IntegrityValidator::new(&cwd);
    let workspace_path = cwd.join(workspace);

    let validation = tokio_block_on(validator.validate(workspace))?;

    let issue_count = validation.issues.len();
    let is_valid = validation.is_valid;

    Ok(ValidationResponse {
        workspace: workspace.to_string(),
        path: workspace_path.to_string_lossy().to_string(),
        is_valid,
        issue_count,
        validation,
    })
}

/// Repair a corrupted workspace.
fn run_repair(workspace: &str, force: bool) -> Result<RepairResponse> {
    if !force {
        Output::info(&format!(
            "Repairing workspace '{}' (use --force to skip confirmation)",
            workspace
        ));
    }

    let cwd = std::env::current_dir()?;
    let validator = IntegrityValidator::new(&cwd);
    let validation = tokio_block_on(validator.validate(workspace))?;
    let repair_executor = RepairExecutor::new();

    let result = tokio_block_on(repair_executor.repair(&validation))?;

    Ok(RepairResponse {
        workspace: workspace.to_string(),
        success: result.success,
        summary: result.summary,
    })
}

/// List available backups.
fn run_backup_list() -> Result<BackupListResponse> {
    let cwd = std::env::current_dir()?;
    let manager = BackupManager::new(&cwd);
    let backups = manager.list_backups("")?;

    Ok(BackupListResponse {
        count: backups.len(),
        backups,
    })
}

/// Restore from a backup.
fn run_backup_restore(backup_id: &str, force: bool) -> Result<RestoreResponse> {
    if !force {
        Output::info(&format!(
            "Restoring backup '{}' (use --force to skip confirmation)",
            backup_id
        ));
    }

    let cwd = std::env::current_dir()?;
    let manager = BackupManager::new(&cwd);
    let workspace_path = Path::new(".");

    let result = manager.restore_backup(backup_id, "", workspace_path)?;

    Ok(RestoreResponse {
        workspace: result.workspace,
        backup_id: backup_id.to_string(),
        success: result.success,
        summary: result.summary,
    })
}

fn tokio_block_on<F: std::future::Future>(fut: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(fut),
        Err(_) => {
            let runtime =
                tokio::runtime::Runtime::new().expect("Failed to create runtime for block_on");
            runtime.block_on(fut)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::integrity::data::IntegrityOptions;

    // ---- IntegrityOptions construction tests ----

    #[test]
    fn integrity_options_validate_construction() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::Validate {
                workspace: "my-ws".to_string(),
            },
        };
        assert!(matches!(
            opts.subcommand,
            IntegritySubcommand::Validate { .. }
        ));
    }

    #[test]
    fn integrity_options_repair_construction() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::Repair {
                workspace: "broken".to_string(),
                force: true,
            },
        };
        assert!(matches!(
            opts.subcommand,
            IntegritySubcommand::Repair { force: true, .. }
        ));
    }

    #[test]
    fn integrity_options_backup_list_construction() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::BackupList,
        };
        assert!(matches!(opts.subcommand, IntegritySubcommand::BackupList));
    }

    #[test]
    fn integrity_options_backup_restore_construction() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::BackupRestore {
                backup_id: "bk-1".to_string(),
                force: false,
            },
        };
        assert!(matches!(
            opts.subcommand,
            IntegritySubcommand::BackupRestore { .. }
        ));
    }
}
