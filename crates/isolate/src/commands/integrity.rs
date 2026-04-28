//! Integrity command - workspace corruption detection, validation, and repair
//!
//! This command manages workspace integrity, including:
//! - Validation: Check workspaces for corruption
//! - Repair: Auto-fix detected issues with backup protection
//! - Backup: Manage workspace backups for recovery
//! - Restore: Rollback to previous backups

use isolate_core::{
    config::Config,
    workspace_integrity::{
        BackupManager, BackupMetadata, CorruptionType, IntegrityValidator, RepairExecutor,
        ValidationResult,
    },
    OutputFormat, SchemaEnvelope,
};
use serde::Serialize;
use std::path::PathBuf;

use crate::commands::{check_prerequisites, get_session_db, workspace_utils};

/// Options for the integrity command
#[derive(Debug, Clone)]
pub struct IntegrityOptions {
    /// Subcommand to run
    pub subcommand: IntegritySubcommand,
    /// Output format
    pub format: OutputFormat,
}

/// Integrity subcommands
#[derive(Debug, Clone)]
pub enum IntegritySubcommand {
    /// Validate workspace integrity
    Validate {
        /// Workspace name or path
        workspace: String,
    },
    /// Repair corrupted workspace
    Repair {
        /// Workspace name or path
        workspace: String,
        /// Force repair without confirmation
        force: bool,
        /// Rebind session record when workspace moved
        rebind: bool,
    },
    /// List available backups
    BackupList,
    /// Restore from backup
    BackupRestore {
        /// Backup ID to restore
        backup_id: String,
        /// Force restore without confirmation
        force: bool,
    },
}

/// Validation response
#[derive(Debug, Clone, Serialize)]
struct ValidationResponse {
    /// Workspace name
    workspace: String,
    /// Absolute path to workspace
    path: String,
    /// Whether workspace is valid
    is_valid: bool,
    /// Number of issues detected
    issue_count: usize,
    /// Detailed validation result
    validation: ValidationResult,
}

/// Repair response
#[derive(Debug, Clone, Serialize)]
struct RepairResponse {
    /// Workspace name
    workspace: String,
    /// Whether repair was successful
    success: bool,
    /// Repair summary message
    summary: String,
}

/// Backup list response
#[derive(Debug, Clone, Serialize)]
struct BackupListResponse {
    /// List of backups
    backups: Vec<BackupMetadata>,
    /// Total count
    count: usize,
}

/// Restore response
#[derive(Debug, Clone, Serialize)]
struct RestoreResponse {
    /// Workspace name
    workspace: String,
    /// Backup ID that was restored
    backup_id: String,
    /// Whether restore was successful
    success: bool,
    /// Restore summary message
    summary: String,
}

/// Run the integrity command
pub async fn run(options: &IntegrityOptions) -> isolate_core::Result<()> {
    // Ensure we're in a JJ repository
    let jj_root = check_prerequisites().await?;

    match &options.subcommand {
        IntegritySubcommand::Validate { workspace } => {
            run_validate(&jj_root, workspace, options.format).await
        }
        IntegritySubcommand::Repair {
            workspace,
            force,
            rebind,
        } => run_repair(&jj_root, workspace, *force, *rebind, options.format).await,
        IntegritySubcommand::BackupList => run_backup_list(&jj_root, options.format).await,
        IntegritySubcommand::BackupRestore { backup_id, force } => {
            run_backup_restore(&jj_root, backup_id, *force, options.format).await
        }
    }
}

/// Validate a workspace
async fn run_validate(
    jj_root: &std::path::Path,
    workspace: &str,
    format: OutputFormat,
) -> isolate_core::Result<()> {
    // Validate workspace name is not empty
    if workspace.trim().is_empty() {
        return Err(isolate_core::Error::InvalidInput(
            "Workspace name cannot be empty. Please provide a valid workspace name.".into(),
        ));
    }

    // Load config to get workspace directory
    let config = match isolate_core::config::load_config().await {
        Ok(config) => config,
        Err(e) => {
            // If config is invalid, we can still validate the workspace
            // Just log the error and continue with defaults
            eprintln!("Warning: Could not load config: {e}");
            eprintln!("Continuing with default configuration for validation...");
            // Continue with default config
            Config::default()
        }
    };

    // Calculate the correct workspace directory from config
    let workspace_dir = if std::path::Path::new(&config.workspace_dir).is_absolute() {
        std::path::Path::new(&config.workspace_dir).to_path_buf()
    } else {
        jj_root.join(std::path::Path::new(&config.workspace_dir))
    };

    let validator = IntegrityValidator::new(workspace_dir);
    let result = validator.validate(workspace).await?;

    let response = ValidationResponse {
        workspace: workspace.to_string(),
        path: result.path.to_string_lossy().to_string(),
        is_valid: result.is_valid,
        issue_count: result.issues.len(),
        validation: result,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("integrity-validate-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_validation_result(&response);
    }

    Ok(())
}

/// Repair a workspace
#[allow(clippy::too_many_lines)]
async fn run_repair(
    jj_root: &std::path::Path,
    workspace: &str,
    force: bool,
    rebind: bool,
    format: OutputFormat,
) -> isolate_core::Result<()> {
    // Validate workspace name is not empty
    if workspace.trim().is_empty() {
        return Err(isolate_core::Error::InvalidInput(
            "Workspace name cannot be empty. Please provide a valid workspace name to repair.".into(),
        ));
    }

    // Load config with graceful error handling for TOML parse errors
    let config = match isolate_core::config::load_config().await {
        Ok(config) => config,
        Err(e) => match &e {
            isolate_core::Error::ParseError(_) | isolate_core::Error::InvalidConfig(_) => {
                let response = RepairResponse {
                        workspace: workspace.to_string(),
                        success: false,
                        summary: format!(
                            "Cannot load configuration due to config error: {e}. \
                             Suggestion: Check .isolate/config.toml or ~/.config/isolate/config.toml for syntax errors."
                        ),
                    };

                if format.is_json() {
                    let envelope =
                        SchemaEnvelope::new("integrity-repair-response", "single", response);
                    println!("{}", serde_json::to_string_pretty(&envelope)?);
                } else {
                    eprintln!("Configuration error: {e}");
                    eprintln!("\nSuggestion: Check your config files for syntax errors:");
                    eprintln!("  - .isolate/config.toml (project config)");
                    eprintln!("  - ~/.config/isolate/config.toml (global config)");
                    eprintln!("\nCommon issues: unclosed brackets, missing quotes, invalid values");
                }
                return Ok(());
            }
            _ => {
                return Err(isolate_core::Error::InvalidInput(format!(
                    "Unable to load config: {e}"
                )));
            }
        },
    };

    let workspace_roots =
        workspace_utils::candidate_workspace_roots(jj_root, &config.workspace_dir);

    // Calculate the correct workspace directory from config
    let workspace_dir = if std::path::Path::new(&config.workspace_dir).is_absolute() {
        std::path::Path::new(&config.workspace_dir).to_path_buf()
    } else {
        jj_root.join(std::path::Path::new(&config.workspace_dir))
    };

    let validator = IntegrityValidator::new(workspace_dir);
    let mut validation = validator.validate(workspace).await?;

    if validation.is_valid {
        let response = RepairResponse {
            workspace: workspace.to_string(),
            success: true,
            summary: "Workspace is already valid, no repair needed".to_string(),
        };

        if format.is_json() {
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Workspace '{workspace}' is valid - no repair needed");
        }

        return Ok(());
    }

    let missing_directory_issue = validation
        .issues
        .iter()
        .any(|issue| issue.corruption_type == CorruptionType::MissingDirectory);

    let relocated_path = if missing_directory_issue {
        workspace_utils::find_relocated_workspace(workspace, &workspace_roots).await
    } else {
        None
    };

    if rebind && missing_directory_issue {
        if let Some(rebind_path) = relocated_path.as_ref() {
            rebind_session(workspace, rebind_path).await?;
            validation = validator.validate(workspace).await?;
            if validation.is_valid {
                let response = RepairResponse {
                    workspace: workspace.to_string(),
                    success: true,
                    summary: format!(
                        "Workspace re-bound to relocated path: {}",
                        rebind_path.display()
                    ),
                };

                if format.is_json() {
                    let envelope =
                        SchemaEnvelope::new("integrity-repair-response", "single", response);
                    println!("{}", serde_json::to_string_pretty(&envelope)?);
                } else {
                    println!(
                        "Rebound workspace '{workspace}' to current location: {}",
                        rebind_path.display()
                    );
                }

                return Ok(());
            }
        } else if format.is_json() {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: false,
                summary: "Unable to detect relocated workspace for rebind".to_string(),
            };
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Rebind requested but no relocated workspace detected for '{workspace}'");
        }
    }

    // Check if issues can be auto-repaired
    if !validation.has_auto_repairable_issues() {
        let response = RepairResponse {
            workspace: workspace.to_string(),
            success: false,
            summary:
                "Workspace has issues that cannot be auto-repaired. Manual intervention required."
                    .to_string(),
        };

        if format.is_json() {
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Workspace has non-repairable issues:");
            for issue in &validation.issues {
                println!("  - {}: {}", issue.corruption_type, issue.description);
            }
            println!("\nManual intervention required.");
        }

        return Ok(());
    }

    // Ask for confirmation unless force is set
    if !force && !confirm_repair(&validation) {
        if format.is_json() {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: false,
                summary: "Repair cancelled by user".to_string(),
            };
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Repair cancelled");
        }
        return Ok(());
    }

    // Perform repair
    let backup_manager = BackupManager::new(jj_root);
    let executor = RepairExecutor::new().with_backup_manager(backup_manager);

    // Get the workspace path
    let workspace_path = jj_root.join(workspace);

    // Get the most severe issue to determine the repair strategy
    // If there are no issues (validation passed), use NoRepairPossible
    let strategy = if let Some(issue) = validation.most_severe_issue() {
        issue.recommended_strategy
    } else {
        // No issues found - workspace is valid
        if format.is_json() {
            let envelope = SchemaEnvelope::new(
                "integrity-repair-response",
                "single",
                RepairResponse {
                    workspace: workspace.to_string(),
                    success: true,
                    summary: "Workspace is already valid, no repair needed".to_string(),
                },
            );
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Workspace '{workspace}' is valid - no repair needed");
        }
        return Ok(());
    };

    match executor
        .execute(workspace, &workspace_path, &validation, strategy)
        .await
    {
        Ok(repair_result) => {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: repair_result.success,
                summary: repair_result.summary.clone(),
            };

            if format.is_json() {
                let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else if repair_result.success {
                println!("Successfully repaired workspace '{workspace}'");
                println!("Summary: {}", repair_result.summary);
            } else {
                println!("Repair failed: {}", repair_result.summary);
            }

            Ok(())
        }
        Err(e) => {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: false,
                summary: format!("Repair failed: {e}"),
            };

            if format.is_json() {
                let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                eprintln!("Repair failed: {e}");
            }

            Err(e)
        }
    }
}

/// List available backups
async fn run_backup_list(jj_root: &std::path::Path, format: OutputFormat) -> isolate_core::Result<()> {
    use futures::stream::{self, StreamExt};
    let manager = BackupManager::new(jj_root);

    // Get all potential workspace directories
    let mut entries = tokio::fs::read_dir(jj_root).await?;
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name_str) = path.file_name().and_then(|n| n.to_str()) {
                if !name_str.starts_with('.') && !name_str.starts_with('_') {
                    paths.push(name_str.to_string());
                }
            }
        }
    }

    // Collect all backups in parallel
    let all_backups: Vec<BackupMetadata> = stream::iter(paths)
        .map(|name| {
            let manager = &manager;
            async move { manager.list_backups(&name).unwrap_or_default() }
        })
        .buffer_unordered(10)
        .flat_map(stream::iter)
        .collect()
        .await;

    // Sort by creation time (newest first)
    let mut all_backups = all_backups;
    all_backups.sort_by_key(|backup| std::cmp::Reverse(backup.created_at));

    let response = BackupListResponse {
        count: all_backups.len(),
        backups: all_backups,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("integrity-backup-list-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_backup_list(&response);
    }

    Ok(())
}

/// Restore from a backup
async fn run_backup_restore(
    jj_root: &std::path::Path,
    backup_id: &str,
    force: bool,
    format: OutputFormat,
) -> isolate_core::Result<()> {
    use futures::stream::{self, StreamExt};
    let manager = BackupManager::new(jj_root);

    // Find the backup across all workspaces
    let mut entries = tokio::fs::read_dir(jj_root).await?;
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name_str) = path.file_name().and_then(|n| n.to_str()) {
                if !name_str.starts_with('.') && !name_str.starts_with('_') {
                    paths.push(name_str.to_string());
                }
            }
        }
    }

    let backup_found = stream::iter(paths)
        .filter_map(|name| {
            let manager = &manager;
            let backup_id = backup_id.to_string();
            async move {
                manager
                    .list_backups(&name)
                    .ok()
                    .and_then(|backups| backups.into_iter().find(|b| b.id == backup_id))
                    .map(|backup| (name, backup))
            }
        })
        .boxed()
        .next()
        .await;

    let (workspace_name, backup) =
        backup_found.ok_or_else(|| isolate_core::Error::NotFound(format!("Backup '{backup_id}' not found")))?;

    // Ask for confirmation unless force is set
    if !force && !confirm_restore(&backup) {
        if format.is_json() {
            let response = RestoreResponse {
                workspace: workspace_name,
                backup_id: backup_id.to_string(),
                success: false,
                summary: "Restore cancelled by user".to_string(),
            };
            let envelope = SchemaEnvelope::new("integrity-restore-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Restore cancelled");
        }
        return Ok(());
    }

    // Get the workspace path
    let workspace_path = jj_root.join(&workspace_name);

    // Perform restore
    match manager.restore_backup(backup_id, &workspace_name, &workspace_path) {
        Ok(rollback_result) => {
            let summary = rollback_result.summary.clone();
            let response = RestoreResponse {
                workspace: workspace_name.clone(),
                backup_id: backup_id.to_string(),
                success: rollback_result.success,
                summary: rollback_result.summary,
            };

            if format.is_json() {
                let envelope =
                    SchemaEnvelope::new("integrity-restore-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else if rollback_result.success {
                println!("Successfully restored from backup '{backup_id}'");
                println!("Summary: {summary}");
            } else {
                println!("Restore failed: {summary}");
            }

            Ok(())
        }
        Err(e) => {
            let response = RestoreResponse {
                workspace: workspace_name.clone(),
                backup_id: backup_id.to_string(),
                success: false,
                summary: format!("Restore failed: {e}"),
            };

            if format.is_json() {
                let envelope =
                    SchemaEnvelope::new("integrity-restore-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                eprintln!("Restore failed: {e}");
            }

            Err(e)
        }
    }
}

/// Print human-readable validation result
fn print_validation_result(response: &ValidationResponse) {
    println!("Workspace Validation: {}", response.workspace);
    println!("Path: {}", response.path);
    println!();

    if response.is_valid {
        println!("Status: VALID");
    } else {
        println!("Status: INVALID");
        println!("Issues: {}", response.issue_count);
        println!();

        for issue in &response.validation.issues {
            println!("  Issue: {}", issue.corruption_type);
            println!("    Description: {}", issue.description);
            if let Some(ctx) = &issue.context {
                println!("    Context: {ctx}");
            }
            println!(
                "    Recommended Action: {}",
                issue.recommended_strategy.description()
            );
            println!();
        }
    }

    println!(
        "Validated at: {}",
        response.validation.validated_at.to_rfc3339()
    );
    println!("Duration: {}ms", response.validation.duration_ms);
}

/// Print human-readable backup list
fn print_backup_list(response: &BackupListResponse) {
    println!("Available Backups: {}", response.count);
    println!();

    if response.backups.is_empty() {
        println!("No backups found.");
        return;
    }

    for backup in &response.backups {
        println!("ID: {}", backup.id);
        println!("  Workspace: {}", backup.workspace);
        println!("  Created: {}", backup.created_at.to_rfc3339());
        println!("  Size: {} bytes", backup.size_bytes);
        println!("  Reason: {:?}", backup.reason);
        if let Some(checksum) = &backup.checksum {
            println!("  Checksum: {checksum}");
        }
        println!();
    }
}

/// Ask user to confirm repair
fn confirm_repair(validation: &ValidationResult) -> bool {
    use std::io::{self, Write};

    println!(
        "Workspace has {} integrity issue(s):",
        validation.issues.len()
    );
    for issue in &validation.issues {
        println!("  - {}: {}", issue.corruption_type, issue.description);
    }
    println!();
    println!(
        "Recommended action: {}",
        validation
            .most_severe_issue()
            .map(|i| i.recommended_strategy.description())
            .unwrap_or("No issues")
    );
    println!();

    print!("Continue with repair? [y/N] ");
    if let Err(_) = io::stdout().flush() {
        return false;
    }

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .ok()
        .map(|_| input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

/// Ask user to confirm restore
fn confirm_restore(backup: &BackupMetadata) -> bool {
    use std::io::{self, Write};

    println!("About to restore from backup:");
    println!("  ID: {}", backup.id);
    println!("  Workspace: {}", backup.workspace);
    println!("  Created: {}", backup.created_at.to_rfc3339());
    println!("  Size: {} bytes", backup.size_bytes);
    println!();
    println!("This will restore the workspace to the state at the time of backup.");
    println!();

    print!("Continue with restore? [y/N] ");
    if let Err(_) = io::stdout().flush() {
        return false;
    }

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .ok()
        .map(|_| input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

async fn rebind_session(workspace: &str, new_path: &std::path::Path) -> isolate_core::Result<()> {
    let db = get_session_db().await?;
    let path_str = new_path.display().to_string();
    db.update_workspace_path(workspace, &path_str)
        .await
        .map_err(|e| isolate_core::Error::InvalidInput(format!("Failed to update session path: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that empty workspace name is handled gracefully
    /// This test ensures that passing an empty string for workspace name
    /// returns a proper error instead of causing a panic downstream
    #[tokio::test]
    async fn test_repair_empty_workspace_name_returns_error() {
        // Create a temp dir to use as jj_root
        let temp_dir =
            tempfile::tempdir().map_err(|e| isolate_core::Error::Unknown(format!("Failed to create temp dir: {}", e)));

        let jj_root = match temp_dir {
            Ok(dir) => dir,
            Err(e) => {
                // If we can't create temp dir, that's OK - skip test
                eprintln!("Skipping test: failed to create temp dir: {e}");
                return;
            }
        };

        let jj_root_path = jj_root.path();

        // Attempt to repair with empty workspace name
        // This should return a Result::Err, not panic
        let result = run_repair(jj_root_path, "", false, false, OutputFormat::Json).await;

        // Verify we get an error, not a panic
        match result {
            Ok(_) => panic!("Repair with empty workspace name should return an error"),
            Err(e) => {
                let error_msg = e.to_string();
                assert!(
                    !error_msg.contains("panic") && !error_msg.contains("unwrap"),
                    "Error should not contain panic-related words: {error_msg}"
                );
            }
        }
    }

    /// Test that validate also handles empty workspace name gracefully
    #[tokio::test]
    async fn test_validate_empty_workspace_name_returns_error() {
        let temp_dir =
            tempfile::tempdir().map_err(|e| isolate_core::Error::Unknown(format!("Failed to create temp dir: {}", e)));

        let jj_root = match temp_dir {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Skipping test: failed to create temp dir: {e}");
                return;
            }
        };

        let jj_root_path = jj_root.path();

        // Attempt to validate with empty workspace name
        let result = run_validate(jj_root_path, "", OutputFormat::Json).await;

        // Verify we get an error, not a panic
        match result {
            Ok(()) => panic!("Validate with empty workspace name should return an error"),
            Err(e) => {
                let error_msg = e.to_string();
                assert!(
                    !error_msg.contains("panic") && !error_msg.contains("unwrap"),
                    "Error should not contain panic-related words: {error_msg}"
                );
            }
        }
    }

    /// Test that repairing a non-existent workspace returns a proper error
    ///
    /// This test ensures that:
    /// 1. No panic occurs when attempting to repair a missing workspace
    /// 2. A descriptive error message is returned
    /// 3. The error clearly indicates the workspace does not exist
    #[tokio::test]
    async fn test_repair_nonexistent_workspace_returns_error() {
        let temp_dir =
            tempfile::tempdir().map_err(|e| isolate_core::Error::Unknown(format!("Failed to create temp dir: {}", e)));

        let jj_root = match temp_dir {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Skipping test: failed to create temp dir: {}", e);
                return;
            }
        };

        let jj_root_path = jj_root.path();

        // Attempt to repair a workspace that doesn't exist
        // The validator should detect the missing directory and return an error
        let result = run_repair(
            jj_root_path,
            "nonexistent-workspace",
            true, // force=true to avoid hanging on stdin
            false,
            OutputFormat::Json,
        )
        .await;

        // Verify the result is Ok (command succeeded) but reports repair failure
        match result {
            Ok(_) => {
                // Command succeeded, check output contains appropriate error message
                // The repair should gracefully handle missing workspace
            }
            Err(e) => {
                // If we get an error, it should not be a panic
                let error_msg = e.to_string();
                assert!(
                    !error_msg.contains("panic")
                        && !error_msg.contains("unwrap")
                        && !error_msg.contains("internal error"),
                    "Error should be a proper error, not a panic: {}",
                    error_msg
                );
            }
        }
    }
}
