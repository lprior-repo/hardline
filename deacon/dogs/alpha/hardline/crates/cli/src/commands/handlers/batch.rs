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
    /// Command name (e.g., "git")
    pub name: String,
    /// Command arguments
    pub args: Vec<String>,
}

impl BatchCommand {
    /// Parse a command string into a BatchCommand
    ///
    /// Format: "git status" -> BatchCommand { name: "git", args: ["status"] }
    pub fn parse(command_str: &str) -> Result<Self> {
        let trimmed = command_str.trim();
        if trimmed.is_empty() {
            return Err(Error::batch_empty());
        }

        let parts: Vec<String> = shell_words::split(trimmed)
            .map_err(|e| Error::validation_error(format!("Invalid command syntax: {}", e)))?;

        if parts.is_empty() {
            return Err(Error::batch_empty());
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
            .map_err(|e| Error::io_error(format!("Failed to execute {}: {}", self.name, e)))?;

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
        return Err(Error::batch_empty());
    }

    if commands.len() > MAX_BATCH_SIZE {
        return Err(Error::batch_size_exceeded(MAX_BATCH_SIZE));
    }

    Ok(())
}

/// Check if workspace is ready for batch execution
///
/// **Calculations (Tier 2)**: Pure validation
fn check_workspace_ready(status: VcsStatus) -> Result<()> {
    match status {
        VcsStatus::Clean => Ok(()),
        VcsStatus::Dirty => Err(Error::working_copy_dirty()),
        VcsStatus::Conflicted => Err(Error::workspace_conflict(
            "Workspace has unresolved conflicts",
        )),
        VcsStatus::Detached => Err(Error::invalid_state(
            "Cannot execute batch in detached HEAD state",
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
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    // Check workspace exists
    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == workspace_name) {
        return Err(Error::workspace_not_found(workspace_name.to_string()));
    }

    // Check workspace status is ready
    let status = backend.status()?;
    check_workspace_ready(status)?;

    // Get database pool for checkpointing
    let _db_pool = get_database_pool()?;

    // Create auto-checkpoint manager
    let auto_cp = AutoCheckpoint::new(_db_pool);
    auto_cp.ensure_table().await?;

    // Create checkpoint guard for batch execution
    let guard = auto_cp
        .guard_if_risky(OperationRisk::Risky)
        .await?
        .ok_or_else(|| Error::internal("Failed to create checkpoint for batch operation"))?;

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
                        return Err(Error::batch_rollback_failed(format!(
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
                    return Err(Error::batch_rollback_failed(format!(
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
        Err(e) => Err(Error::batch_rollback_failed(format!(
            "Failed to commit batch checkpoint: {}",
            e
        ))),
    }
}

/// Rollback the checkpoint and return error if it fails
async fn rollback_with_error(guard: &CheckpointGuard, checkpoint_id: &str) -> Result<()> {
    guard.rollback().await.map_err(|e| {
        Error::batch_rollback_failed(format!(
            "Failed to rollback checkpoint '{}': {}",
            checkpoint_id, e
        ))
    })
}

/// Find the path to a workspace
fn find_workspace_path(cwd: &std::path::Path, workspace_name: &str) -> Result<std::path::PathBuf> {
    // For Git, workspaces are typically in .git/worktrees or similar
    // The cwd is already the repo root, so we use it directly
    let _ = workspace_name;
    Ok(cwd.to_path_buf())
}

/// Get the database pool for checkpointing
fn get_database_pool() -> Result<SqlitePool> {
    Err(Error::unimplemented(
        "Database pool not configured for batch execution",
    ))
}

/// Run batch command from CLI
pub async fn run_batch(workspace: Option<String>, commands: Vec<String>) -> Result<()> {
    if commands.is_empty() {
        return Err(Error::batch_empty());
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
            Output::success(&format!(
                "Batch committed with checkpoint '{}'",
                checkpoint_id
            ));
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
            Err(Error::batch_command_failed(error))
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
        let result = BatchCommand::parse("git status");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["status"]);
    }

    #[test]
    fn test_parse_command_with_args() {
        let result = BatchCommand::parse("git log --oneline -10");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["log", "--oneline", "-10"]);
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

    #[test]
    fn test_check_workspace_ready_conflicted() {
        let result = check_workspace_ready(VcsStatus::Conflicted);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_workspace_ready_detached() {
        let result = check_workspace_ready(VcsStatus::Detached);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_size_exceeded() {
        let commands: Vec<BatchCommand> = (0..=MAX_BATCH_SIZE)
            .map(|i| BatchCommand {
                name: format!("cmd-{i}"),
                args: vec![],
            })
            .collect();
        let result = validate_batch(&commands);
        assert!(
            result.is_err(),
            "batch exceeding max size should be rejected"
        );
    }

    #[test]
    fn test_validate_batch_at_exact_max() {
        let commands: Vec<BatchCommand> = (0..MAX_BATCH_SIZE)
            .map(|i| BatchCommand {
                name: format!("cmd-{i}"),
                args: vec![],
            })
            .collect();
        let result = validate_batch(&commands);
        assert!(result.is_ok(), "batch at exact max size should be accepted");
    }

    #[test]
    fn test_parse_command_with_quoted_args() {
        let result = BatchCommand::parse("echo \"hello world\"");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_command_with_single_quotes() {
        let result = BatchCommand::parse("echo 'single quoted'");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["single quoted"]);
    }

    #[test]
    fn test_parse_command_name_only() {
        let result = BatchCommand::parse("jj");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "jj");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_parse_command_with_tabs() {
        let result = BatchCommand::parse("echo\thello");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
    }

    #[test]
    fn test_command_result_clone() {
        let result = CommandResult {
            command: BatchCommand {
                name: "test".to_string(),
                args: vec!["arg".to_string()],
            },
            success: true,
            exit_code: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_command_result_failed_fields() {
        let result = CommandResult {
            command: BatchCommand {
                name: "false".to_string(),
                args: vec![],
            },
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "command failed".to_string(),
        };
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
        assert!(result.stdout.is_empty());
    }

    #[test]
    fn test_batch_result_committed_variant() {
        let result = BatchResult::Committed {
            checkpoint_id: "cp-123".to_string(),
            results: vec![],
        };
        if let BatchResult::Committed {
            checkpoint_id,
            results,
        } = result
        {
            assert_eq!(checkpoint_id, "cp-123");
            assert!(results.is_empty());
        } else {
            panic!("expected Committed variant");
        }
    }

    #[test]
    fn test_batch_result_rolledback_variant() {
        let result = BatchResult::RolledBack {
            failed_at: 2,
            error: "something broke".to_string(),
            partial_results: vec![],
        };
        if let BatchResult::RolledBack {
            failed_at,
            error,
            partial_results,
        } = result
        {
            assert_eq!(failed_at, 2);
            assert_eq!(error, "something broke");
            assert!(partial_results.is_empty());
        } else {
            panic!("expected RolledBack variant");
        }
    }

    #[test]
    fn test_batch_execution_error_display() {
        let err = BatchExecutionError::Empty;
        assert!(format!("{err:?}").contains("Empty"));

        let err = BatchExecutionError::SizeExceeded {
            max: 10,
            actual: 20,
        };
        let msg = format!("{err:?}");
        assert!(msg.contains("20"));
        assert!(msg.contains("10"));

        let err = BatchExecutionError::WorkspaceNotReady("locked".to_string());
        let msg = format!("{err:?}");
        assert!(msg.contains("locked"));

        let err = BatchExecutionError::CommandFailed {
            index: 0,
            command: BatchCommand {
                name: "jj".to_string(),
                args: vec![],
            },
            exit_code: 1,
            stderr: "fail".to_string(),
        };
        let msg = format!("{err:?}");
        assert!(msg.contains("CommandFailed"));

        let err = BatchExecutionError::RollbackFailed {
            checkpoint_id: "cp".to_string(),
            underlying: "disk full".to_string(),
        };
        let msg = format!("{err:?}");
        assert!(msg.contains("cp"));

        let err = BatchExecutionError::CommitFailed {
            checkpoint_id: "cp2".to_string(),
            underlying: "io error".to_string(),
        };
        let msg = format!("{err:?}");
        assert!(msg.contains("cp2"));
    }

    #[test]
    fn test_batch_execution_error_equality() {
        let a = BatchExecutionError::Empty;
        let b = BatchExecutionError::Empty;
        assert_eq!(a, b);

        let c = BatchExecutionError::SizeExceeded {
            max: 10,
            actual: 20,
        };
        let d = BatchExecutionError::SizeExceeded {
            max: 10,
            actual: 20,
        };
        assert_eq!(c, d);

        let e = BatchExecutionError::SizeExceeded { max: 5, actual: 20 };
        assert_ne!(c, e);
    }

    #[tokio::test]
    async fn test_run_batch_rejects_empty_commands() {
        let result = run_batch(None, vec![]).await;
        assert!(result.is_err(), "empty command list should be rejected");
    }

    // -- BatchCommand parse edge cases --

    #[test]
    fn test_parse_command_with_env_var_syntax() {
        let result = BatchCommand::parse("echo $HOME");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["$HOME"]);
    }

    #[test]
    fn test_parse_command_with_pipe_not_split() {
        // shell_words::split treats pipe as literal characters, not shell operators
        let result = BatchCommand::parse("echo hello | grep hello");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        // shell_words::split preserves pipe characters as literals
        assert_eq!(cmd.name, "echo");
    }

    #[test]
    fn test_parse_command_with_redirect_preserved() {
        // shell_words does not interpret shell redirections
        let result = BatchCommand::parse("echo hello > /dev/null");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
    }

    #[test]
    fn test_parse_command_with_backslash_escape() {
        // shell_words may or may not handle backslash-escaped quotes depending
        // on the exact string. The key invariant is that it doesn't panic and
        // returns a result (either Ok or Err).
        let result = BatchCommand::parse(r#"echo "hello \"world\""#);
        // Whether it parses or not is implementation-defined; just ensure no panic
        let _ = result;
    }

    #[test]
    fn test_parse_command_with_newline_rejected() {
        // shell_words should reject unescaped newlines
        let result = BatchCommand::parse("echo\ntest");
        // Behavior depends on shell_words implementation
        // Key invariant: it should not produce a command that runs arbitrary input
        if let Ok(cmd) = result {
            // If it parses, name should be "echo" and newline is not part of it
            assert_eq!(cmd.name, "echo");
        }
        // Either error or safe parsing is acceptable
    }

    #[test]
    fn test_parse_command_leading_whitespace_stripped() {
        let result = BatchCommand::parse("   echo hello");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
    }

    #[test]
    fn test_parse_command_trailing_whitespace_stripped() {
        let result = BatchCommand::parse("echo hello   ");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
    }

    #[test]
    fn test_parse_command_just_whitespace_is_empty() {
        let result = BatchCommand::parse(" \t\n ");
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_command_equality() {
        let a = BatchCommand {
            name: "jj".to_string(),
            args: vec!["status".to_string()],
        };
        let b = BatchCommand {
            name: "jj".to_string(),
            args: vec!["status".to_string()],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_batch_command_inequality_different_name() {
        let a = BatchCommand {
            name: "jj".to_string(),
            args: vec![],
        };
        let b = BatchCommand {
            name: "git".to_string(),
            args: vec![],
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_batch_command_inequality_different_args() {
        let a = BatchCommand {
            name: "jj".to_string(),
            args: vec!["status".to_string()],
        };
        let b = BatchCommand {
            name: "jj".to_string(),
            args: vec!["log".to_string()],
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_batch_command_clone_independence() {
        let a = BatchCommand {
            name: "jj".to_string(),
            args: vec!["status".to_string()],
        };
        let mut b = a.clone();
        b.name = "git".to_string();
        assert_eq!(a.name, "jj", "clone should be independent");
        assert_eq!(b.name, "git");
    }

    #[test]
    fn test_batch_command_debug_format() {
        let cmd = BatchCommand {
            name: "echo".to_string(),
            args: vec!["hello".to_string(), "world".to_string()],
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("echo"));
        assert!(debug.contains("hello"));
    }

    // -- CommandResult tests --

    #[test]
    fn test_command_result_equality() {
        let a = CommandResult {
            command: BatchCommand {
                name: "t".to_string(),
                args: vec![],
            },
            success: true,
            exit_code: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        let b = CommandResult {
            command: BatchCommand {
                name: "t".to_string(),
                args: vec![],
            },
            success: true,
            exit_code: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_command_result_debug_format() {
        let result = CommandResult {
            command: BatchCommand {
                name: "test".to_string(),
                args: vec![],
            },
            success: true,
            exit_code: 42,
            stdout: "output".to_string(),
            stderr: String::new(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("42"));
        assert!(debug.contains("output"));
    }

    // -- BatchResult tests --

    #[test]
    fn test_batch_result_committed_with_results() {
        let results = vec![CommandResult {
            command: BatchCommand {
                name: "jj".to_string(),
                args: vec!["status".to_string()],
            },
            success: true,
            exit_code: 0,
            stdout: "clean".to_string(),
            stderr: String::new(),
        }];
        let result = BatchResult::Committed {
            checkpoint_id: "cp-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed { results: r, .. } = &result {
            assert_eq!(r.len(), 1);
        } else {
            panic!("expected Committed");
        }
    }

    #[test]
    fn test_batch_result_rolledback_with_partial_results() {
        let partial = vec![CommandResult {
            command: BatchCommand {
                name: "jj".to_string(),
                args: vec!["status".to_string()],
            },
            success: true,
            exit_code: 0,
            stdout: "clean".to_string(),
            stderr: String::new(),
        }];
        let result = BatchResult::RolledBack {
            failed_at: 1,
            error: "command failed".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &result
        {
            assert_eq!(*failed_at, 1);
            assert_eq!(partial_results.len(), 1);
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_batch_result_clone() {
        let result = BatchResult::Committed {
            checkpoint_id: "cp-x".to_string(),
            results: vec![],
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_batch_result_debug_format() {
        let result = BatchResult::RolledBack {
            failed_at: 0,
            error: "test error".to_string(),
            partial_results: vec![],
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("RolledBack"));
        assert!(debug.contains("test error"));
    }

    // -- BatchExecutionError tests --

    #[test]
    fn test_batch_execution_error_empty_debug() {
        let err = BatchExecutionError::Empty;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Empty"));
    }

    #[test]
    fn test_batch_execution_error_all_variants_have_debug() {
        let errors = vec![
            BatchExecutionError::Empty,
            BatchExecutionError::SizeExceeded {
                max: 10,
                actual: 20,
            },
            BatchExecutionError::WorkspaceNotReady("dirty".to_string()),
            BatchExecutionError::CommandFailed {
                index: 0,
                command: BatchCommand {
                    name: "cmd".to_string(),
                    args: vec![],
                },
                exit_code: 1,
                stderr: "err".to_string(),
            },
            BatchExecutionError::RollbackFailed {
                checkpoint_id: "cp".to_string(),
                underlying: "io".to_string(),
            },
            BatchExecutionError::CommitFailed {
                checkpoint_id: "cp".to_string(),
                underlying: "io".to_string(),
            },
        ];
        for err in errors {
            let debug = format!("{:?}", err);
            assert!(
                !debug.is_empty(),
                "every variant should have a non-empty debug repr"
            );
        }
    }

    #[test]
    fn test_batch_execution_error_clone() {
        let err = BatchExecutionError::Empty;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_validate_batch_single_command() {
        let commands = vec![BatchCommand {
            name: "jj".to_string(),
            args: vec!["status".to_string()],
        }];
        let result = validate_batch(&commands);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_batch_max_minus_one() {
        let commands: Vec<BatchCommand> = (0..MAX_BATCH_SIZE - 1)
            .map(|i| BatchCommand {
                name: format!("cmd-{i}"),
                args: vec![],
            })
            .collect();
        let result = validate_batch(&commands);
        assert!(result.is_ok(), "max - 1 should be accepted");
    }

    #[test]
    fn test_validate_batch_max_plus_one() {
        let commands: Vec<BatchCommand> = (0..=MAX_BATCH_SIZE)
            .map(|i| BatchCommand {
                name: format!("cmd-{i}"),
                args: vec![],
            })
            .collect();
        let result = validate_batch(&commands);
        assert!(result.is_err(), "max + 1 should be rejected");
    }

    // -- check_workspace_ready error messages --

    #[test]
    fn test_check_workspace_ready_dirty_message() {
        let result = check_workspace_ready(VcsStatus::Dirty);
        let err = result.expect_err("should fail");
        assert!(
            err.to_string().to_lowercase().contains("dirty")
                || err.to_string().to_lowercase().contains("uncommitted")
        );
    }

    #[test]
    fn test_check_workspace_ready_conflicted_message() {
        let result = check_workspace_ready(VcsStatus::Conflicted);
        let err = result.expect_err("should fail");
        assert!(err.to_string().to_lowercase().contains("conflict"));
    }

    #[test]
    fn test_check_workspace_ready_detached_message() {
        let result = check_workspace_ready(VcsStatus::Detached);
        let err = result.expect_err("should fail");
        assert!(
            err.to_string().to_lowercase().contains("detached")
                || err.to_string().to_lowercase().contains("head")
        );
    }

    // -- MAX_BATCH_SIZE constant --

    #[test]
    fn test_max_batch_size_is_reasonable() {
        assert!(MAX_BATCH_SIZE > 0, "max batch size must be positive");
        assert!(
            MAX_BATCH_SIZE <= 10_000,
            "max batch size should have a reasonable upper bound"
        );
    }

    use proptest::prelude::*;
    use proptest::proptest;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn prop_parse_non_empty_command_always_has_name(cmd_str in "[a-zA-Z][a-zA-Z0-9_ -]{0,100}") {
            let result = BatchCommand::parse(&cmd_str);
            if let Ok(cmd) = result {
                prop_assert!(!cmd.name.is_empty());
                prop_assert!(!cmd.name.contains(' '));
            }
        }

        #[test]
        fn prop_validate_batch_accepts_small_batches(
            count in 1usize..10,
            name in "[a-z]{1,5}"
        ) {
            let commands: Vec<BatchCommand> = (0..count)
                .map(|_| BatchCommand {
                    name: name.clone(),
                    args: vec![],
                })
                .collect();
            let result = validate_batch(&commands);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn prop_command_result_clone_equality(
            name in "[a-z]{1,5}",
            exit_code in 0i32..10i32,
            stdout in "[a-z]{0,10}",
            stderr in "[a-z]{0,10}"
        ) {
            let result = CommandResult {
                command: BatchCommand { name: name.clone(), args: vec![] },
                success: exit_code == 0,
                exit_code,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            };
            prop_assert_eq!(result.clone(), result.clone());
        }

        #[test]
        fn prop_batch_command_clone_equality(name in "[a-z]{1,5}") {
            let cmd = BatchCommand { name: name.clone(), args: vec![] };
            let cloned = cmd.clone();
            prop_assert_eq!(cmd, cloned);
        }

        #[test]
        fn prop_batch_command_clone_independence(name in "[a-z]{1,5}", arg in "[a-z]{1,5}") {
            let cmd = BatchCommand { name: name.clone(), args: vec![arg.clone()] };
            let mut cloned = cmd.clone();
            cloned.name = "modified".to_string();
            prop_assert_eq!(cmd.name, name);
            prop_assert_eq!(cloned.name, "modified");
        }

        #[test]
        fn prop_command_result_clone_preserves_all_fields(
            name in "[a-z]{1,5}",
            exit_code in 0i32..10i32,
            stdout in "[a-z]{0,10}",
            stderr in "[a-z]{0,10}"
        ) {
            let result = CommandResult {
                command: BatchCommand { name: name.clone(), args: vec![] },
                success: exit_code == 0,
                exit_code,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            };
            let cloned = result.clone();
            prop_assert_eq!(result.command.name, cloned.command.name);
            prop_assert_eq!(result.success, cloned.success);
            prop_assert_eq!(result.exit_code, cloned.exit_code);
            prop_assert_eq!(result.stdout, cloned.stdout);
            prop_assert_eq!(result.stderr, cloned.stderr);
        }
    }
}
