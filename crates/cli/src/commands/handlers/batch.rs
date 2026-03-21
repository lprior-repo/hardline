//! Atomic batch execution for workspace commands.
//!
//! Executes a batch of commands atomically - all succeed or all roll back.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: BatchCommand, BatchResult, CommandResult (inert, serializable)
//! - **Calculations**: validate_batch, parse_command_string (pure functions)
//! - **Actions**: execute_batch, checkpoint management (I/O)

use std::process::Command;

use scp_core::{
    checkpoint::{AutoCheckpoint, CheckpointGuard, OperationRisk},
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};
use shell_words;
use sqlx::SqlitePool;

/// Maximum number of commands in a single batch
const MAX_BATCH_SIZE: usize = 100;

/// A single command in a batch execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCommand {
    /// Command name (e.g., "jj", "git")
    pub name: String,
    /// Command arguments
    pub args: Vec<String>,
}

impl BatchCommand {
    /// Parse a command string into a BatchCommand
    ///
    /// Format: "jj status" -> BatchCommand { name: "jj", args: ["status"] }
    pub fn parse(command_str: &str) -> Result<Self> {
        let trimmed = command_str.trim();
        if trimmed.is_empty() {
            return Err(Error::BatchEmpty);
        }

        let parts: Vec<String> = shell_words::split(trimmed)
            .map_err(|e| Error::ValidationError(format!("Invalid command syntax: {}", e)))?;

        if parts.is_empty() {
            return Err(Error::BatchEmpty);
        }

        let name = parts[0].clone();
        let args = parts.into_iter().skip(1).collect();

        Ok(Self { name, args })
    }

    /// Execute this command in the given working directory
    fn execute(&self, cwd: &std::path::Path) -> Result<CommandResult> {
        let mut cmd = Command::new(&self.name);
        cmd.args(&self.args);
        cmd.current_dir(cwd);

        let output = cmd
            .output()
            .map_err(|e| Error::IoError(format!("Failed to execute {}: {}", self.name, e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandResult {
            command: self.clone(),
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        })
    }
}

/// Result of executing a single command
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub command: BatchCommand,
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Result of a batch execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchResult {
    /// All commands committed successfully
    Committed {
        checkpoint_id: String,
        results: Vec<CommandResult>,
    },
    /// Batch rolled back due to failure
    RolledBack {
        failed_at: usize,
        error: String,
        partial_results: Vec<CommandResult>,
    },
}

/// Batch execution errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchExecutionError {
    /// Empty batch provided
    Empty,
    /// Too many commands in batch
    SizeExceeded { max: usize, actual: usize },
    /// Workspace not ready (locked or dirty)
    WorkspaceNotReady(String),
    /// Command execution failed
    CommandFailed {
        index: usize,
        command: BatchCommand,
        exit_code: i32,
        stderr: String,
    },
    /// Rollback failed after command failure
    RollbackFailed {
        checkpoint_id: String,
        underlying: String,
    },
    /// Commit failed after all commands succeeded
    CommitFailed {
        checkpoint_id: String,
        underlying: String,
    },
}

/// Validate a batch of commands
///
/// **Calculations (Tier 2)**: Pure function, no I/O
fn validate_batch(commands: &[BatchCommand]) -> Result<()> {
    if commands.is_empty() {
        return Err(Error::BatchEmpty);
    }

    if commands.len() > MAX_BATCH_SIZE {
        return Err(Error::BatchSizeExceeded(MAX_BATCH_SIZE));
    }

    Ok(())
}

/// Check if workspace is ready for batch execution
///
/// **Calculations (Tier 2)**: Pure validation
fn check_workspace_ready(status: VcsStatus) -> Result<()> {
    match status {
        VcsStatus::Clean => Ok(()),
        VcsStatus::Dirty => Err(Error::WorkingCopyDirty),
        VcsStatus::Conflicted => Err(Error::WorkspaceConflict(
            "Workspace has unresolved conflicts".to_string(),
        )),
        VcsStatus::Detached => Err(Error::InvalidState(
            "Cannot execute batch in detached HEAD state".to_string(),
        )),
    }
}

/// Execute a batch of commands atomically
///
/// **Actions (Tier 3)**: I/O operations
///
/// # Contract
/// - All commands execute sequentially
/// - If any command fails, all previous changes are rolled back
/// - If rollback fails after a command failure, the error is propagated (not silently ignored)
/// - If all commands succeed, checkpoint is committed
pub async fn execute_batch(
    workspace_name: &str,
    commands: Vec<BatchCommand>,
) -> Result<BatchResult> {
    // TIER 2: Validate before any I/O
    validate_batch(&commands)?;

    // TIER 3: I/O - Get VCS backend
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // Check workspace exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == workspace_name) {
        return Err(Error::WorkspaceNotFound(workspace_name.to_string()));
    }

    // Check workspace status is ready
    let status = backend.status()?;
    check_workspace_ready(status)?;

    // Get database pool for checkpointing
    let db_pool = get_database_pool()?;

    // Create auto-checkpoint manager
    let auto_cp = AutoCheckpoint::new(db_pool);
    auto_cp.ensure_table().await?;

    // Create checkpoint guard for batch execution
    let guard = auto_cp
        .guard_if_risky(OperationRisk::Risky)
        .await?
        .ok_or_else(|| {
            Error::Internal("Failed to create checkpoint for batch operation".to_string())
        })?;

    let checkpoint_id = guard.id().to_string();

    // Execute commands sequentially
    let mut results: Vec<CommandResult> = Vec::new();

    for (index, command) in commands.into_iter().enumerate() {
        let workspace_path = find_workspace_path(&cwd, workspace_name)?;
        let result = command.execute(&workspace_path);

        match result {
            Ok(cmd_result) => {
                results.push(cmd_result);
                if !results.last().map(|r| r.success).unwrap_or(false) {
                    // Command failed - need to rollback
                    let failed_result = results.last().unwrap();

                    // First, try to rollback the checkpoint
                    if let Err(rollback_err) = rollback_with_error(&guard, &checkpoint_id).await {
                        // Rollback failed - this is critical and must be propagated
                        return Err(Error::BatchRollbackFailed(format!(
                            "Rollback failed after command {} failed: {}. Workspace may be in indeterminate state.",
                            index,
                            rollback_err
                        )));
                    }

                    return Ok(BatchResult::RolledBack {
                        failed_at: index,
                        error: format!(
                            "Command '{}' failed with exit code {}",
                            failed_result.command.name, failed_result.exit_code
                        ),
                        partial_results: results,
                    });
                }
            }
            Err(e) => {
                // Command execution error - need to rollback
                if let Err(rollback_err) = rollback_with_error(&guard, &checkpoint_id).await {
                    return Err(Error::BatchRollbackFailed(format!(
                        "Rollback failed after execution error at command {}: {}. Workspace may be in indeterminate state.",
                        index,
                        rollback_err
                    )));
                }

                return Err(e);
            }
        }
    }

    // All commands succeeded - commit the checkpoint
    match guard.commit().await {
        Ok(()) => Ok(BatchResult::Committed {
            checkpoint_id,
            results,
        }),
        Err(e) => Err(Error::BatchRollbackFailed(format!(
            "Failed to commit batch checkpoint: {}",
            e
        ))),
    }
}

/// Rollback the checkpoint and return error if it fails
async fn rollback_with_error(guard: &CheckpointGuard, checkpoint_id: &str) -> Result<()> {
    guard.rollback().await.map_err(|e| {
        Error::BatchRollbackFailed(format!(
            "Failed to rollback checkpoint '{}': {}",
            checkpoint_id, e
        ))
    })
}

/// Find the path to a workspace
fn find_workspace_path(cwd: &std::path::Path, workspace_name: &str) -> Result<std::path::PathBuf> {
    // For JJ, workspaces are typically in .jj/workspace or similar
    // The cwd is already the repo root, so we use it directly
    Ok(cwd.to_path_buf())
}

/// Get the database pool for checkpointing
fn get_database_pool() -> Result<SqlitePool> {
    Err(Error::Unimplemented(
        "Database pool not configured for batch execution".to_string(),
    ))
}

/// Run batch command from CLI
pub async fn run_batch(workspace: Option<String>, commands: Vec<String>) -> Result<()> {
    if commands.is_empty() {
        return Err(Error::BatchEmpty);
    }

    let workspace_name = workspace.unwrap_or_else(|| "default".to_string());

    // Parse all command strings into BatchCommands
    let batch_commands: Result<Vec<BatchCommand>> = commands
        .iter()
        .map(|cmd_str| BatchCommand::parse(cmd_str))
        .collect();

    let batch_commands = batch_commands?;

    Output::info(&format!(
        "Executing batch of {} commands on workspace '{}'...",
        batch_commands.len(),
        workspace_name
    ));

    let result = execute_batch(&workspace_name, batch_commands).await?;

    match result {
        BatchResult::Committed {
            checkpoint_id,
            results,
        } => {
            Output::success(&format!("Batch committed with checkpoint '{}'", checkpoint_id));
            for (i, cmd_result) in results.iter().enumerate() {
                if cmd_result.success {
                    Output::info(&format!("  [{}] {}: OK", i, cmd_result.command.name));
                } else {
                    Output::info(&format!(
                        "  [{}] {}: FAILED (exit {})",
                        i, cmd_result.command.name, cmd_result.exit_code
                    ));
                }
            }
            Ok(())
        }
        BatchResult::RolledBack {
            failed_at,
            error,
            partial_results,
        } => {
            Output::error(&format!("Batch rolled back at command {}", failed_at));
            Output::error(&error);
            for (i, cmd_result) in partial_results.iter().enumerate() {
                Output::info(&format!("  [{}] {}: executed", i, cmd_result.command.name));
            }
            Err(Error::BatchCommandFailed(error))
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_command() {
        let result = BatchCommand::parse("jj status");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "jj");
        assert_eq!(cmd.args, vec!["status"]);
    }

    #[test]
    fn test_parse_command_with_args() {
        let result = BatchCommand::parse("jj log -r @ -T commit_id");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "jj");
        assert_eq!(
            cmd.args,
            vec!["log", "-r", "@", "-T", "commit_id"]
        );
    }

    #[test]
    fn test_parse_empty_string() {
        let result = BatchCommand::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = BatchCommand::parse("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_empty() {
        let result = validate_batch(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_valid() {
        let commands = vec![
            BatchCommand {
                name: "jj".to_string(),
                args: vec!["status".to_string()],
            },
            BatchCommand {
                name: "jj".to_string(),
                args: vec!["log".to_string()],
            },
        ];
        let result = validate_batch(&commands);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_workspace_ready_clean() {
        let result = check_workspace_ready(VcsStatus::Clean);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_workspace_ready_dirty() {
        let result = check_workspace_ready(VcsStatus::Dirty);
        assert!(result.is_err());
    }
}
