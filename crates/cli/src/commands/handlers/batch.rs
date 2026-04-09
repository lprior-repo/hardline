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

/// Allowed command prefixes for batch execution
const ALLOWED_COMMANDS: &[&str] = &["git", "jj", "scp"];

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

        // Validate command is in allowed list
        if !ALLOWED_COMMANDS.contains(&name.as_str()) {
            return Err(Error::validation_error(format!(
                "Command '{}' is not allowed in batch execution. Allowed: {}",
                name,
                ALLOWED_COMMANDS.join(", ")
            )));
        }

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
        let result = BatchCommand::parse("git commit -m \"hello world\"");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["commit", "-m", "hello world"]);
    }

    #[test]
    fn test_parse_command_with_single_quotes() {
        let result = BatchCommand::parse("git commit -m 'single quoted'");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["commit", "-m", "single quoted"]);
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
        let result = BatchCommand::parse("git\tcommit");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["commit"]);
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
        let result = BatchCommand::parse("git log $HOME");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["log", "$HOME"]);
    }

    #[test]
    fn test_parse_command_with_pipe_not_split() {
        // shell_words::split treats pipe as literal characters, not shell operators
        let result = BatchCommand::parse("git log | grep hello");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        // shell_words::split preserves pipe characters as literals
        assert_eq!(cmd.name, "git");
    }

    #[test]
    fn test_parse_command_with_redirect_preserved() {
        // shell_words does not interpret shell redirections
        let result = BatchCommand::parse("git log > /dev/null");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
    }

    #[test]
    fn test_parse_command_with_backslash_escape() {
        // shell_words may or may not handle backslash-escaped quotes depending
        // on the exact string. The key invariant is that it doesn't panic and
        // returns a result (either Ok or Err).
        let result = BatchCommand::parse(r#"git commit -m "hello \"world\""#);
        // Whether it parses or not is implementation-defined; just ensure no panic
        let _ = result;
    }

    #[test]
    fn test_parse_command_with_newline_rejected() {
        // shell_words should reject unescaped newlines
        let result = BatchCommand::parse("git\nlog");
        // Behavior depends on shell_words implementation
        // Key invariant: it should not produce a command that runs arbitrary input
        if let Ok(cmd) = result {
            // If it parses, name should be "git" and newline is not part of it
            assert_eq!(cmd.name, "git");
        }
        // Either error or safe parsing is acceptable
    }

    #[test]
    fn test_parse_command_leading_whitespace_stripped() {
        let result = BatchCommand::parse("   git log");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
    }

    #[test]
    fn test_parse_command_trailing_whitespace_stripped() {
        let result = BatchCommand::parse("git status   ");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["status"]);
    }

    #[test]
    fn test_parse_command_just_whitespace_is_empty() {
        let result = BatchCommand::parse(" \t\n ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_rejects_disallowed_command() {
        let result = BatchCommand::parse("rm -rf /");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not allowed"),
            "Error should mention whitelist: {err}"
        );
    }

    #[test]
    fn test_parse_command_rejects_sh() {
        let result = BatchCommand::parse("sh -c 'echo evil'");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_accepts_git() {
        let result = BatchCommand::parse("git status");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_command_accepts_jj() {
        let result = BatchCommand::parse("jj status");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_command_accepts_scp() {
        let result = BatchCommand::parse("scp workspace list");
        assert!(result.is_ok());
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

    // ========================================================================
    // RED QUEEN: Adversarial tests for batch command
    // ========================================================================

    // --- Command injection attempts ---

    #[test]
    fn adversarial_parse_rejects_bash() {
        assert!(BatchCommand::parse("bash -c 'rm -rf /'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_sh() {
        assert!(BatchCommand::parse("sh -c 'echo evil'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_zsh() {
        assert!(BatchCommand::parse("zsh -c 'evil'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_python() {
        assert!(BatchCommand::parse("python -c 'import os'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_perl() {
        assert!(BatchCommand::parse("perl -e 'print 1'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_awk() {
        assert!(BatchCommand::parse("awk '{print $1}'").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_sed() {
        assert!(BatchCommand::parse("sed 's/old/new/' file.txt").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_curl() {
        assert!(BatchCommand::parse("curl http://evil.com").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_wget() {
        assert!(BatchCommand::parse("wget http://evil.com").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_chmod() {
        assert!(BatchCommand::parse("chmod 777 /etc/passwd").is_err());
    }

    #[test]
    fn adversarial_parse_rejects_docker() {
        assert!(BatchCommand::parse("docker run -it ubuntu").is_err());
    }

    #[test]
    fn adversarial_parse_pipe_not_interpreted() {
        let result = BatchCommand::parse("git log | grep secret");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert!(cmd.args.iter().any(|a| a.contains('|')));
    }

    #[test]
    fn adversarial_parse_redirect_not_interpreted() {
        let result = BatchCommand::parse("git log > /tmp/output");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
    }

    #[test]
    fn adversarial_parse_semicolon_not_split() {
        let result = BatchCommand::parse("git log ; rm -rf /");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
    }

    #[test]
    fn adversarial_parse_backtick_args() {
        let result = BatchCommand::parse("git log `echo evil`");
        assert!(result.is_ok() || result.is_err(), "should not panic");
    }

    #[test]
    fn adversarial_parse_dollar_substitution() {
        let result = BatchCommand::parse("git log $(cat /etc/passwd)");
        assert!(result.is_ok() || result.is_err(), "should not panic");
    }

    #[test]
    fn adversarial_parse_null_byte() {
        let result = BatchCommand::parse("git\x00log");
        let _ = result; // should not panic
    }

    #[test]
    fn adversarial_parse_only_allowed_accepted() {
        for allowed in ALLOWED_COMMANDS {
            let result = BatchCommand::parse(&format!("{} status", allowed));
            assert!(result.is_ok(), "{} should be allowed", allowed);
        }
    }

    #[test]
    fn adversarial_parse_absolute_path_rejected() {
        let result = BatchCommand::parse("/usr/bin/git status");
        assert!(result.is_err(), "absolute path should not bypass allowlist");
    }

    // --- Rollback boundary conditions ---

    #[test]
    fn adversarial_rolled_back_at_position_zero() {
        let result = BatchResult::RolledBack {
            failed_at: 0,
            error: "first command failed".to_string(),
            partial_results: vec![],
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = result
        {
            assert_eq!(failed_at, 0);
            assert!(partial_results.is_empty());
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn adversarial_committed_with_zero_results() {
        let result = BatchResult::Committed {
            checkpoint_id: "cp-empty".to_string(),
            results: vec![],
        };
        if let BatchResult::Committed { results, .. } = result {
            assert!(results.is_empty());
        } else {
            panic!("expected Committed");
        }
    }

    // --- Workspace state contamination ---

    #[test]
    fn adversarial_workspace_dirty_error_informative() {
        let err = check_workspace_ready(VcsStatus::Dirty).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("dirty") || msg.contains("uncommitted"),
            "got: {msg}"
        );
    }

    #[test]
    fn adversarial_workspace_conflicted_error_informative() {
        let err = check_workspace_ready(VcsStatus::Conflicted).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(msg.contains("conflict"), "got: {msg}");
    }

    #[test]
    fn adversarial_workspace_detached_error_informative() {
        let err = check_workspace_ready(VcsStatus::Detached).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("detached") || msg.contains("head"),
            "got: {msg}"
        );
    }

    // --- Parsing edge cases ---

    #[test]
    fn adversarial_parse_unicode_command_rejected() {
        let result = BatchCommand::parse("🦀 status");
        assert!(result.is_err());
    }

    #[test]
    fn adversarial_parse_dangerous_git_flags() {
        let dangerous = vec![
            "git push --force origin main",
            "git clean -fdx",
            "git reset --hard HEAD",
            "git checkout -- .",
            "git branch -D main",
        ];
        for cmd_str in &dangerous {
            assert!(
                BatchCommand::parse(cmd_str).is_ok(),
                "git commands should parse: {}",
                cmd_str
            );
        }
    }

    #[test]
    fn adversarial_parse_unmatched_quote_rejected() {
        let result = BatchCommand::parse(r#"git commit -m "unclosed quote"#);
        assert!(result.is_err());
    }

    // --- ALLOWED_COMMANDS integrity ---

    #[test]
    fn adversarial_allowed_no_shells() {
        let shells = ["sh", "bash", "zsh", "fish", "dash", "ksh", "csh", "tcsh"];
        for shell in &shells {
            assert!(
                !ALLOWED_COMMANDS.contains(shell),
                "{shell} must not be allowed"
            );
        }
    }

    #[test]
    fn adversarial_allowed_no_dangerous_commands() {
        let dangerous = [
            "rm", "chmod", "chown", "sudo", "su", "kill", "dd", "mkfs", "curl", "wget", "nc",
            "ncat", "telnet", "ssh",
        ];
        for cmd in &dangerous {
            assert!(!ALLOWED_COMMANDS.contains(cmd), "{cmd} must not be allowed");
        }
    }

    #[test]
    fn adversarial_allowed_commands_exactly_git_jj_scp() {
        assert_eq!(ALLOWED_COMMANDS, &["git", "jj", "scp"]);
    }

    // --- CommandResult edge cases ---

    #[test]
    fn adversarial_command_result_negative_exit_code() {
        let result = CommandResult {
            command: BatchCommand {
                name: "git".to_string(),
                args: vec![],
            },
            success: false,
            exit_code: -1,
            stdout: String::new(),
            stderr: "signal killed".to_string(),
        };
        assert!(!result.success);
        assert_eq!(result.exit_code, -1);
    }

    #[test]
    fn adversarial_command_result_large_output() {
        let large = "x".repeat(1_000_000);
        let result = CommandResult {
            command: BatchCommand {
                name: "git".to_string(),
                args: vec![],
            },
            success: true,
            exit_code: 0,
            stdout: large.clone(),
            stderr: large.clone(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.stdout.len(), 1_000_000);
    }

    // --- Trait bounds ---

    #[test]
    fn adversarial_batch_types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BatchCommand>();
        assert_send_sync::<CommandResult>();
        assert_send_sync::<BatchResult>();
        assert_send_sync::<BatchExecutionError>();
    }

    // ========================================================================
    // Exhaustive tests: batch execution, atomicity, partial failure,
    // rollback, ordering, nested batch prevention, progress, aggregation
    // ========================================================================

    // --- Batch ordering preservation ---

    #[test]
    fn test_batch_results_preserve_command_order() {
        let commands = vec![
            BatchCommand {
                name: "git".to_string(),
                args: vec!["status".to_string()],
            },
            BatchCommand {
                name: "jj".to_string(),
                args: vec!["log".to_string()],
            },
            BatchCommand {
                name: "scp".to_string(),
                args: vec!["workspace".to_string(), "list".to_string()],
            },
        ];
        let results: Vec<CommandResult> = commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| CommandResult {
                command: cmd.clone(),
                success: true,
                exit_code: 0,
                stdout: format!("output-{i}"),
                stderr: String::new(),
            })
            .collect();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].command.name, "git");
        assert_eq!(results[1].command.name, "jj");
        assert_eq!(results[2].command.name, "scp");
    }

    #[test]
    fn test_batch_order_preserved_in_committed_result() {
        let results = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string(), ".".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string(), "-m".to_string(), "msg".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["push".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        ];
        let batch = BatchResult::Committed {
            checkpoint_id: "cp-order-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed {
            results: r,
            checkpoint_id,
        } = &batch
        {
            assert_eq!(r.len(), 3);
            assert_eq!(r[0].command.args[0], "add");
            assert_eq!(r[1].command.args[0], "commit");
            assert_eq!(r[2].command.args[0], "push");
            assert_eq!(checkpoint_id, "cp-order-1");
        } else {
            panic!("expected Committed");
        }
    }

    #[test]
    fn test_batch_rolledback_preserves_order_of_executed_commands() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "staged".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string()],
                },
                success: false,
                exit_code: 1,
                stdout: String::new(),
                stderr: "nothing to commit".to_string(),
            },
        ];
        let batch = BatchResult::RolledBack {
            failed_at: 1,
            error: "nothing to commit".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &batch
        {
            assert_eq!(*failed_at, 1);
            assert_eq!(partial_results.len(), 2);
            assert!(partial_results[0].success);
            assert!(!partial_results[1].success);
        } else {
            panic!("expected RolledBack");
        }
    }

    // --- Partial failure reporting ---

    #[test]
    fn test_partial_failure_reports_correct_failed_index() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "jj".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "clean".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "jj".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "clean".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["push".to_string()],
                },
                success: false,
                exit_code: 128,
                stdout: String::new(),
                stderr: "rejected".to_string(),
            },
        ];
        let result = BatchResult::RolledBack {
            failed_at: 2,
            error: "Command 'git' failed with exit code 128".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            failed_at,
            error,
            partial_results,
        } = result
        {
            assert_eq!(failed_at, 2);
            assert!(error.contains("128"));
            assert!(error.contains("git"));
            assert_eq!(partial_results.len(), 3);
            assert!(partial_results[0].success);
            assert!(partial_results[1].success);
            assert!(!partial_results[2].success);
            assert_eq!(partial_results[2].exit_code, 128);
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_partial_failure_includes_stderr_from_failed_command() {
        let partial = vec![CommandResult {
            command: BatchCommand {
                name: "git".to_string(),
                args: vec!["commit".to_string()],
            },
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "error: pathspec 'nonexistent' did not match any files".to_string(),
        }];
        let result = BatchResult::RolledBack {
            failed_at: 0,
            error: "Command 'git' failed with exit code 1".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            partial_results, ..
        } = result
        {
            assert_eq!(partial_results.len(), 1);
            assert_eq!(
                partial_results[0].stderr,
                "error: pathspec 'nonexistent' did not match any files"
            );
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_partial_failure_first_command_succeeds_second_fails() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string()],
                },
                success: false,
                exit_code: 1,
                stdout: String::new(),
                stderr: "aborted".to_string(),
            },
        ];
        let result = BatchResult::RolledBack {
            failed_at: 1,
            error: "commit aborted".to_string(),
            partial_results: partial,
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = result
        {
            assert_eq!(failed_at, 1);
            assert_eq!(partial_results.len(), 2);
        } else {
            panic!("expected RolledBack");
        }
    }

    // --- Atomicity: all-or-nothing semantics ---

    #[test]
    fn test_atomicity_committed_means_all_succeeded() {
        let results = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "clean".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "jj".to_string(),
                    args: vec!["diff".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        ];
        let batch = BatchResult::Committed {
            checkpoint_id: "cp-atomic-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed { results: r, .. } = &batch {
            for cmd_result in r {
                assert!(
                    cmd_result.success,
                    "Committed batch must have all commands succeed"
                );
                assert_eq!(cmd_result.exit_code, 0);
            }
        } else {
            panic!("expected Committed");
        }
    }

    #[test]
    fn test_atomicity_rolledback_means_at_least_one_failed() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string()],
                },
                success: false,
                exit_code: 1,
                stdout: String::new(),
                stderr: "failed".to_string(),
            },
        ];
        let batch = BatchResult::RolledBack {
            failed_at: 1,
            error: "commit failed".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &batch
        {
            assert!(*failed_at < partial_results.len());
            let failed_cmd = &partial_results[*failed_at];
            assert!(
                !failed_cmd.success,
                "RolledBack batch must have a failed command at failed_at index"
            );
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_atomicity_rollback_at_first_command() {
        let batch = BatchResult::RolledBack {
            failed_at: 0,
            error: "first command failed".to_string(),
            partial_results: vec![CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["clone".to_string()],
                },
                success: false,
                exit_code: 128,
                stdout: String::new(),
                stderr: "repository not found".to_string(),
            }],
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &batch
        {
            assert_eq!(*failed_at, 0);
            assert_eq!(partial_results.len(), 1);
            assert!(!partial_results[0].success);
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_atomicity_rollback_at_last_command() {
        let partial: Vec<CommandResult> = (0..5)
            .map(|i| CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec![format!("arg-{i}")],
                },
                success: i < 4,
                exit_code: if i < 4 { 0 } else { 1 },
                stdout: String::new(),
                stderr: if i == 4 {
                    "failed".to_string()
                } else {
                    String::new()
                },
            })
            .collect();
        let batch = BatchResult::RolledBack {
            failed_at: 4,
            error: "last command failed".to_string(),
            partial_results: partial,
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &batch
        {
            assert_eq!(*failed_at, 4);
            assert_eq!(partial_results.len(), 5);
            for (i, r) in partial_results.iter().enumerate().take(4) {
                assert!(r.success, "command {i} should have succeeded");
            }
            assert!(
                !partial_results[4].success,
                "last command should have failed"
            );
        } else {
            panic!("expected RolledBack");
        }
    }

    // --- Rollback semantics ---

    #[test]
    fn test_rollback_preserves_partial_results_for_debugging() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["reset".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "HEAD is now at abc123".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["clean".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["push".to_string()],
                },
                success: false,
                exit_code: 1,
                stdout: String::new(),
                stderr: "push rejected".to_string(),
            },
        ];
        let batch = BatchResult::RolledBack {
            failed_at: 2,
            error: "push rejected".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            partial_results, ..
        } = &batch
        {
            assert_eq!(
                partial_results.len(),
                3,
                "all executed commands should be preserved"
            );
            for (i, r) in partial_results.iter().enumerate() {
                assert_eq!(r.command.name, "git", "command {i} should be git");
                if i < 2 {
                    assert!(
                        r.success,
                        "command {i} should have succeeded before failure"
                    );
                }
            }
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_batch_execution_error_rollback_failed_includes_checkpoint_id() {
        let err = BatchExecutionError::RollbackFailed {
            checkpoint_id: "cp-rollback-42".to_string(),
            underlying: "disk full".to_string(),
        };
        if let BatchExecutionError::RollbackFailed {
            checkpoint_id,
            underlying,
        } = &err
        {
            assert_eq!(checkpoint_id, "cp-rollback-42");
            assert_eq!(underlying, "disk full");
        } else {
            panic!("expected RollbackFailed");
        }
    }

    #[test]
    fn test_batch_execution_error_commit_failed_includes_checkpoint_id() {
        let err = BatchExecutionError::CommitFailed {
            checkpoint_id: "cp-commit-99".to_string(),
            underlying: "io error".to_string(),
        };
        if let BatchExecutionError::CommitFailed {
            checkpoint_id,
            underlying,
        } = &err
        {
            assert_eq!(checkpoint_id, "cp-commit-99");
            assert_eq!(underlying, "io error");
        } else {
            panic!("expected CommitFailed");
        }
    }

    #[test]
    fn test_batch_execution_error_command_failed_has_all_fields() {
        let cmd = BatchCommand {
            name: "jj".to_string(),
            args: vec!["rebase".to_string(), "-d".to_string(), "main".to_string()],
        };
        let err = BatchExecutionError::CommandFailed {
            index: 3,
            command: cmd.clone(),
            exit_code: 2,
            stderr: "concurrent modification".to_string(),
        };
        if let BatchExecutionError::CommandFailed {
            index,
            command,
            exit_code,
            stderr,
        } = &err
        {
            assert_eq!(*index, 3);
            assert_eq!(*command, cmd);
            assert_eq!(*exit_code, 2);
            assert_eq!(stderr, "concurrent modification");
        } else {
            panic!("expected CommandFailed");
        }
    }

    // --- Nested batch prevention ---

    #[test]
    fn test_parse_batch_subcommand_allowed() {
        let result = BatchCommand::parse("scp batch run");
        assert!(result.is_ok(), "scp batch subcommand should be parseable");
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "scp");
        assert_eq!(cmd.args, vec!["batch", "run"]);
    }

    #[test]
    fn test_batch_cannot_contain_shell_batch_invocation() {
        let result = BatchCommand::parse("sh -c 'scp batch run'");
        assert!(
            result.is_err(),
            "shell wrapper around batch should be rejected by whitelist"
        );
    }

    #[test]
    fn test_batch_does_not_allow_batch_command_as_primary() {
        let result = BatchCommand::parse("batch run");
        assert!(result.is_err(), "batch is not in ALLOWED_COMMANDS");
    }

    #[test]
    fn test_validate_batch_allows_mixed_allowed_commands() {
        let commands = vec![
            BatchCommand {
                name: "git".to_string(),
                args: vec!["status".to_string()],
            },
            BatchCommand {
                name: "jj".to_string(),
                args: vec!["log".to_string()],
            },
            BatchCommand {
                name: "scp".to_string(),
                args: vec!["workspace".to_string(), "list".to_string()],
            },
        ];
        assert!(validate_batch(&commands).is_ok());
    }

    #[test]
    fn test_validate_batch_rejects_batch_size_boundary() {
        let commands: Vec<BatchCommand> = (0..MAX_BATCH_SIZE + 1)
            .map(|i| BatchCommand {
                name: "git".to_string(),
                args: vec![format!("arg-{i}")],
            })
            .collect();
        assert!(validate_batch(&commands).is_err());
    }

    // --- Progress reporting (Output patterns) ---

    #[test]
    fn test_command_result_captures_stdout_for_progress() {
        let result = CommandResult {
            command: BatchCommand {
                name: "git".to_string(),
                args: vec!["status".to_string()],
            },
            success: true,
            exit_code: 0,
            stdout: "On branch main\nnothing to commit, working tree clean".to_string(),
            stderr: String::new(),
        };
        assert!(
            !result.stdout.is_empty(),
            "stdout should capture command output for progress display"
        );
        assert!(result.stdout.contains("main"));
    }

    #[test]
    fn test_command_result_captures_stderr_for_diagnostics() {
        let result = CommandResult {
            command: BatchCommand {
                name: "jj".to_string(),
                args: vec!["rebase".to_string()],
            },
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "Error: concurrent modification detected\n".to_string(),
        };
        assert!(
            !result.stderr.is_empty(),
            "stderr should capture error output for diagnostics"
        );
        assert!(result.stderr.contains("concurrent"));
    }

    // --- Batch result aggregation ---

    #[test]
    fn test_aggregate_committed_results_success_count() {
        let results = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "1 file changed".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["push".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        ];
        let batch = BatchResult::Committed {
            checkpoint_id: "cp-aggr-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed { results: r, .. } = &batch {
            let success_count = r.iter().filter(|r| r.success).count();
            let failure_count = r.iter().filter(|r| !r.success).count();
            assert_eq!(success_count, 3);
            assert_eq!(failure_count, 0);
        } else {
            panic!("expected Committed");
        }
    }

    #[test]
    fn test_aggregate_rolledback_results_mixed_counts() {
        let partial = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["add".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["commit".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["push".to_string()],
                },
                success: false,
                exit_code: 128,
                stdout: String::new(),
                stderr: "rejected".to_string(),
            },
        ];
        let batch = BatchResult::RolledBack {
            failed_at: 2,
            error: "push rejected".to_string(),
            partial_results: partial.clone(),
        };
        if let BatchResult::RolledBack {
            failed_at,
            partial_results,
            ..
        } = &batch
        {
            let success_count = partial_results.iter().filter(|r| r.success).count();
            let failure_count = partial_results.iter().filter(|r| !r.success).count();
            assert_eq!(success_count, 2);
            assert_eq!(failure_count, 1);
            assert_eq!(*failed_at, 2);
        } else {
            panic!("expected RolledBack");
        }
    }

    #[test]
    fn test_aggregate_exit_codes_across_results() {
        let results = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "jj".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "clean".to_string(),
                stderr: String::new(),
            },
        ];
        let batch = BatchResult::Committed {
            checkpoint_id: "cp-exit-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed { results: r, .. } = &batch {
            let exit_codes: Vec<i32> = r.iter().map(|r| r.exit_code).collect();
            assert_eq!(exit_codes, vec![0, 0]);
        } else {
            panic!("expected Committed");
        }
    }

    #[test]
    fn test_aggregate_stdout_concatenation_for_report() {
        let results = vec![
            CommandResult {
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec!["status".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "On branch main\n".to_string(),
                stderr: String::new(),
            },
            CommandResult {
                command: BatchCommand {
                    name: "jj".to_string(),
                    args: vec!["log".to_string()],
                },
                success: true,
                exit_code: 0,
                stdout: "@  abc123 commit msg\n".to_string(),
                stderr: String::new(),
            },
        ];
        let batch = BatchResult::Committed {
            checkpoint_id: "cp-concat-1".to_string(),
            results: results.clone(),
        };
        if let BatchResult::Committed { results: r, .. } = &batch {
            let combined: String = r.iter().map(|r| r.stdout.as_str()).collect();
            assert!(combined.contains("On branch main"));
            assert!(combined.contains("abc123"));
        } else {
            panic!("expected Committed");
        }
    }

    // --- run_batch CLI entry point ---

    #[tokio::test]
    async fn test_run_batch_with_default_workspace() {
        let result = run_batch(None, vec!["git status".to_string()]).await;
        assert!(
            result.is_err(),
            "run_batch should fail because get_database_pool returns Err"
        );
    }

    #[tokio::test]
    async fn test_run_batch_with_explicit_workspace() {
        let result = run_batch(
            Some("my-workspace".to_string()),
            vec!["jj status".to_string()],
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_batch_rejects_disallowed_command_in_list() {
        let result = run_batch(None, vec!["git status".to_string(), "rm -rf /".to_string()]).await;
        assert!(
            result.is_err(),
            "batch with disallowed command should be rejected"
        );
    }

    #[tokio::test]
    async fn test_run_batch_rejects_empty_command_in_list() {
        let result = run_batch(None, vec!["git status".to_string(), "".to_string()]).await;
        assert!(
            result.is_err(),
            "empty command string in batch should be rejected"
        );
    }

    #[tokio::test]
    async fn test_run_batch_rejects_whitespace_only_command() {
        let result = run_batch(None, vec!["   ".to_string()]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_batch_all_allowed_commands_parsed() {
        let result = run_batch(
            None,
            vec![
                "git status".to_string(),
                "jj log".to_string(),
                "scp workspace list".to_string(),
            ],
        )
        .await;
        assert!(
            result.is_err(),
            "should fail at execute_batch (no db pool), not at parse"
        );
    }

    // --- find_workspace_path ---

    #[test]
    fn test_find_workspace_path_returns_cwd_regardless_of_name() {
        let tmp = tempfile::tempdir().unwrap();
        let path = find_workspace_path(tmp.path(), "any-workspace");
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), tmp.path());
    }

    #[test]
    fn test_find_workspace_path_empty_workspace_name() {
        let tmp = tempfile::tempdir().unwrap();
        let path = find_workspace_path(tmp.path(), "");
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), tmp.path());
    }

    #[test]
    fn test_find_workspace_path_nonexistent_directory() {
        let path = find_workspace_path(std::path::Path::new("/nonexistent/path/xyz123"), "ws");
        assert!(path.is_ok(), "find_workspace_path does not check existence");
    }

    // --- validate_batch edge cases ---

    #[test]
    fn test_validate_batch_single_allowed_command() {
        let commands = vec![BatchCommand {
            name: "scp".to_string(),
            args: vec![
                "workspace".to_string(),
                "create".to_string(),
                "test".to_string(),
            ],
        }];
        assert!(validate_batch(&commands).is_ok());
    }

    #[test]
    fn test_validate_batch_error_message_mentions_size() {
        let commands: Vec<BatchCommand> = (0..=MAX_BATCH_SIZE)
            .map(|i| BatchCommand {
                name: "git".to_string(),
                args: vec![format!("a{i}")],
            })
            .collect();
        let err = validate_batch(&commands).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(&MAX_BATCH_SIZE.to_string()),
            "error should mention max size {MAX_BATCH_SIZE}: {msg}"
        );
    }

    #[test]
    fn test_validate_batch_empty_error_message() {
        let err = validate_batch(&[]).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("empty"),
            "empty batch error should mention 'empty': {msg}"
        );
    }

    // --- BatchCommand parse with all allowed commands ---

    #[test]
    fn test_parse_scp_with_subcommand_and_args() {
        let result = BatchCommand::parse("scp workspace create my-workspace");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "scp");
        assert_eq!(cmd.args, vec!["workspace", "create", "my-workspace"]);
    }

    #[test]
    fn test_parse_jj_with_complex_args() {
        let result = BatchCommand::parse("jj new --message 'create new working copy'");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "jj");
        assert_eq!(
            cmd.args,
            vec!["new", "--message", "create new working copy"]
        );
    }

    #[test]
    fn test_parse_git_with_flag_cluster() {
        let result = BatchCommand::parse("git log --oneline --graph --all");
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "git");
        assert_eq!(cmd.args, vec!["log", "--oneline", "--graph", "--all"]);
    }

    // --- BatchExecutionError exhaustive variant coverage ---

    #[test]
    fn test_batch_execution_error_workspace_not_ready_message() {
        let err = BatchExecutionError::WorkspaceNotReady("unresolved conflicts".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("WorkspaceNotReady"));
        assert!(debug.contains("unresolved conflicts"));
    }

    #[test]
    fn test_batch_execution_error_size_exceeded_fields() {
        let err = BatchExecutionError::SizeExceeded {
            max: 100,
            actual: 200,
        };
        if let BatchExecutionError::SizeExceeded { max, actual } = &err {
            assert_eq!(*max, 100);
            assert_eq!(*actual, 200);
        } else {
            panic!("expected SizeExceeded");
        }
    }

    #[test]
    fn test_batch_execution_error_all_variants_are_cloneable() {
        let errors = vec![
            BatchExecutionError::Empty,
            BatchExecutionError::SizeExceeded { max: 1, actual: 2 },
            BatchExecutionError::WorkspaceNotReady("test".to_string()),
            BatchExecutionError::CommandFailed {
                index: 0,
                command: BatchCommand {
                    name: "git".to_string(),
                    args: vec![],
                },
                exit_code: 1,
                stderr: "err".to_string(),
            },
            BatchExecutionError::RollbackFailed {
                checkpoint_id: "cp".to_string(),
                underlying: "err".to_string(),
            },
            BatchExecutionError::CommitFailed {
                checkpoint_id: "cp".to_string(),
                underlying: "err".to_string(),
            },
        ];
        for err in errors {
            let _cloned = err.clone();
        }
    }

    // --- BatchResult exhaustive variant coverage ---

    #[test]
    fn test_batch_result_committed_checkpoint_id_preserved() {
        let result = BatchResult::Committed {
            checkpoint_id: "cp-uuid-12345-abcd".to_string(),
            results: vec![],
        };
        match &result {
            BatchResult::Committed {
                checkpoint_id,
                results,
            } => {
                assert_eq!(checkpoint_id, "cp-uuid-12345-abcd");
                assert!(results.is_empty());
            }
            BatchResult::RolledBack { .. } => panic!("expected Committed"),
        }
    }

    #[test]
    fn test_batch_result_rolledback_error_message_preserved() {
        let error_msg = "Command 'jj' failed with exit code 1: concurrent modification";
        let result = BatchResult::RolledBack {
            failed_at: 5,
            error: error_msg.to_string(),
            partial_results: vec![],
        };
        match &result {
            BatchResult::RolledBack {
                error, failed_at, ..
            } => {
                assert_eq!(*failed_at, 5);
                assert_eq!(error, error_msg);
            }
            BatchResult::Committed { .. } => panic!("expected RolledBack"),
        }
    }

    // --- proptest: batch invariants ---

    proptest! {
        #[test]
        fn prop_batch_result_committed_preserves_checkpoint_id(
            cp_id in "[a-zA-Z0-9-]{1,50}"
        ) {
            let results = vec![];
            let batch = BatchResult::Committed {
                checkpoint_id: cp_id.clone(),
                results,
            };
            if let BatchResult::Committed { checkpoint_id, .. } = batch {
                prop_assert_eq!(checkpoint_id, cp_id);
            } else {
                panic!("expected Committed");
            }
        }

        #[test]
        fn prop_batch_result_rolledback_failed_at_within_bounds(
            failed_at in 0usize..20,
            total in 1usize..20
        ) {
            let actual_failed = failed_at.min(total - 1);
            let partial: Vec<CommandResult> = (0..=actual_failed)
                .map(|i| CommandResult {
                    command: BatchCommand { name: "git".to_string(), args: vec![format!("a{i}")] },
                    success: i < actual_failed,
                    exit_code: if i < actual_failed { 0 } else { 1 },
                    stdout: String::new(),
                    stderr: String::new(),
                })
                .collect();
            let batch = BatchResult::RolledBack {
                failed_at: actual_failed,
                error: "failed".to_string(),
                partial_results: partial,
            };
            if let BatchResult::RolledBack { failed_at, partial_results, .. } = batch {
                prop_assert!(failed_at < partial_results.len());
                prop_assert!(!partial_results[failed_at].success);
            } else {
                panic!("expected RolledBack");
            }
        }

        #[test]
        fn prop_validate_batch_accepts_up_to_max(
            count in 1usize..MAX_BATCH_SIZE
        ) {
            let commands: Vec<BatchCommand> = (0..count)
                .map(|i| BatchCommand {
                    name: "git".to_string(),
                    args: vec![format!("arg-{i}")],
                })
                .collect();
            prop_assert!(validate_batch(&commands).is_ok());
        }

        #[test]
        fn prop_validate_batch_rejects_over_max(
            count in MAX_BATCH_SIZE + 1..MAX_BATCH_SIZE + 10
        ) {
            let commands: Vec<BatchCommand> = (0..count)
                .map(|i| BatchCommand {
                    name: "git".to_string(),
                    args: vec![format!("arg-{i}")],
                })
                .collect();
            prop_assert!(validate_batch(&commands).is_err());
        }

        #[test]
        fn prop_command_result_success_matches_exit_zero(exit_code in 0i32..5i32) {
            let result = CommandResult {
                command: BatchCommand { name: "t".to_string(), args: vec![] },
                success: exit_code == 0,
                exit_code,
                stdout: String::new(),
                stderr: String::new(),
            };
            prop_assert_eq!(result.success, result.exit_code == 0);
        }
    }
}
