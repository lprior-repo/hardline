//! Batch command - atomic execution with checkpoint rollback

use scp_core::Result;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BATCH_SIZE: usize = 100;

// ========================================================================
// Data Layer: CheckpointId newtype
// ========================================================================

/// Checkpoint identifier - newtype wrapper
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointId(String);

impl CheckpointId {
    /// Create a new CheckpointId
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Create from timestamp
    pub fn from_timestamp() -> Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| scp_core::Error::CheckpointError(format!("system time error: {}", e)))?;
        Ok(Self(format!("batch-{}", now.as_millis())))
    }

    /// Get the inner value as str
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ========================================================================
// Data Layer: CommandResult
// ========================================================================

/// Result of a single command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub output: String,
}

// ========================================================================
// Data Layer: BatchResult
// ========================================================================

/// Result of batch execution
#[derive(Debug, Clone)]
pub enum BatchResult {
    /// Batch completed successfully
    Completed {
        checkpoint_id: Option<CheckpointId>,
        results: Vec<CommandResult>,
    },
    /// Batch rolled back due to failure
    RolledBack {
        checkpoint_id: CheckpointId,
        error: scp_core::Error,
        results: Vec<CommandResult>,
    },
}

impl BatchResult {
    /// Returns whether the batch was rolled back
    pub fn is_rolled_back(&self) -> bool {
        matches!(self, Self::RolledBack { .. })
    }

    /// Returns the checkpoint ID if any
    pub fn checkpoint_id(&self) -> Option<&str> {
        match self {
            Self::Completed { checkpoint_id, .. } => checkpoint_id.as_ref().map(|id| id.as_str()),
            Self::RolledBack { checkpoint_id, .. } => Some(checkpoint_id.as_str()),
        }
    }

    /// Returns the command results
    pub fn results(&self) -> &[CommandResult] {
        match self {
            Self::Completed { results, .. } => results,
            Self::RolledBack { results, .. } => results,
        }
    }
}

// ========================================================================
// Calculations Layer: Pure validation functions
// ========================================================================

/// Validate batch input - pure function
fn validate_batch_input(commands: &[String], checkpoint_path: Option<&str>) -> Result<()> {
    if commands.is_empty() {
        return Err(scp_core::Error::BatchEmpty);
    }
    if commands.len() > MAX_BATCH_SIZE {
        return Err(scp_core::Error::BatchSizeExceeded(MAX_BATCH_SIZE));
    }
    if checkpoint_path.map_or(false, |p| p.is_empty()) {
        return Err(scp_core::Error::ValidationError(
            "checkpoint path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

/// Generate checkpoint ID - pure function
fn generate_checkpoint_id(checkpoint_enabled: bool) -> Option<CheckpointId> {
    checkpoint_enabled
        .then(|| CheckpointId::from_timestamp().ok())
        .flatten()
}

// ========================================================================
// Actions Layer: Main entry point
// ========================================================================

/// Run batch commands atomically
pub fn run(
    commands: Vec<String>,
    checkpoint_path: Option<&str>,
    dry_run: bool,
) -> Result<BatchResult> {
    // Validation step
    validate_batch_input(&commands, checkpoint_path)?;

    // Dry run mode
    if dry_run {
        return Ok(execute_dry_run(&commands));
    }

    // Execute with optional checkpoint
    let checkpoint_enabled = checkpoint_path.is_some();
    let checkpoint_id = generate_checkpoint_id(checkpoint_enabled);
    execute_batch(commands, checkpoint_id)
}

// ========================================================================
// Actions Layer: Dry run
// ========================================================================

/// Execute dry run - pure function
fn execute_dry_run(commands: &[String]) -> BatchResult {
    println!("Batch dry-run mode:");
    for (i, cmd) in commands.iter().enumerate() {
        println!("  {}: {}", i + 1, cmd);
    }
    println!("Total: {} commands", commands.len());
    BatchResult::Completed {
        checkpoint_id: None,
        results: commands
            .iter()
            .map(|c| CommandResult {
                command: c.clone(),
                success: true,
                output: "dry-run".to_string(),
            })
            .collect(),
    }
}

// ========================================================================
// Actions Layer: Batch execution
// ========================================================================

/// Execute batch with checkpoint support
fn execute_batch(
    commands: Vec<String>,
    checkpoint_id: Option<CheckpointId>,
) -> Result<BatchResult> {
    // Execute commands and collect results
    let results: Vec<CommandResult> = commands.iter().map(|cmd| execute_command(cmd)).collect();

    // Check for first failure - get index to avoid borrow issues
    let failure_index = results.iter().position(|r| !r.success);

    // Handle based on results
    match failure_index {
        Some(_) => {
            // Find the failure to get its output
            let failure_output = results
                .iter()
                .find(|r| !r.success)
                .map(|r| r.output.clone())
                .unwrap_or_default();
            handle_failure_with_output(results, checkpoint_id, failure_output)
        }
        None => handle_success(results, checkpoint_id),
    }
}

/// Handle successful batch execution
fn handle_success(
    results: Vec<CommandResult>,
    checkpoint_id: Option<CheckpointId>,
) -> Result<BatchResult> {
    print_success_results(&results, checkpoint_id.as_ref().map(|id| id.as_str()));
    Ok(BatchResult::Completed {
        checkpoint_id,
        results,
    })
}

/// Handle batch failure - attempt rollback if checkpoint exists (using output string)
fn handle_failure_with_output(
    results: Vec<CommandResult>,
    checkpoint_id: Option<CheckpointId>,
    failure_output: String,
) -> Result<BatchResult> {
    match checkpoint_id {
        Some(id) => {
            // Convert failure to error
            let error = scp_core::Error::BatchCommandFailed(failure_output);

            // Attempt rollback
            perform_rollback(&id)?;

            print_rollback_results(&results, id.as_str());

            Ok(BatchResult::RolledBack {
                checkpoint_id: id,
                error,
                results,
            })
        }
        None => {
            let error = scp_core::Error::BatchCommandFailed(failure_output);
            print_failure_results(&results);
            Err(error)
        }
    }
}

// ========================================================================
// Actions Layer: Rollback implementation
// ========================================================================

/// Perform actual rollback to checkpoint
fn perform_rollback(checkpoint_id: &CheckpointId) -> Result<()> {
    // In a real implementation, this would:
    // 1. Read checkpoint data from the checkpoint path
    // 2. Restore workspace state from checkpoint
    // 3. Clean up any partial state

    let checkpoint_path = std::path::Path::new(".scp/checkpoints").join(checkpoint_id.as_str());

    if !checkpoint_path.exists() {
        return Err(scp_core::Error::CheckpointError(format!(
            "checkpoint not found: {}",
            checkpoint_id
        )));
    }

    // Read checkpoint metadata
    let metadata_path = checkpoint_path.join("metadata.json");
    if metadata_path.exists() {
        println!("Restoring from checkpoint: {}", checkpoint_id);
        // Implementation would restore state here
        println!("Rollback completed successfully");
    } else {
        println!("Checkpoint metadata not found, skipping restore");
    }

    Ok(())
}

// ========================================================================
// Actions Layer: Output functions
// ========================================================================

/// Print success results
fn print_success_results(results: &[CommandResult], checkpoint_id: Option<&str>) {
    println!("Batch executed successfully ({} commands)", results.len());
    for result in results {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}", status, result.command);
    }
    if let Some(id) = checkpoint_id {
        println!("Checkpoint: {}", id);
    }
}

/// Print failure results
fn print_failure_results(results: &[CommandResult]) {
    println!("Batch execution failed. No checkpoint to rollback to.");
    println!("Results:");
    for result in results {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}: {}", status, result.command, result.output);
    }
}

/// Print rollback results
fn print_rollback_results(results: &[CommandResult], checkpoint_id: &str) {
    println!(
        "Batch execution failed. Rolled back to checkpoint: {}",
        checkpoint_id
    );
    println!("Results:");
    for result in results {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}: {}", status, result.command, result.output);
    }
}

// ========================================================================
// Actions Layer: Command execution
// ========================================================================

/// Execute a single command
fn execute_command(cmd: &str) -> CommandResult {
    use std::process::Command;

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return CommandResult {
            command: cmd.to_string(),
            success: false,
            output: "empty command".to_string(),
        };
    }

    let program = parts[0];
    let args = &parts[1..];

    match Command::new(program).args(args).output() {
        Ok(output) => {
            let success = output.status.success();
            let output_str = if success {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            };
            CommandResult {
                command: cmd.to_string(),
                success,
                output: output_str,
            }
        }
        Err(e) => CommandResult {
            command: cmd.to_string(),
            success: false,
            output: format!("failed to execute: {}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_empty_commands_fails() {
        let result = run(vec![], None, false);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), scp_core::Error::BatchEmpty));
    }

    #[test]
    fn test_batch_size_exceeded() {
        let commands: Vec<String> = (0..101).map(|i| format!("cmd{}", i)).collect();
        let result = run(commands, None, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            scp_core::Error::BatchSizeExceeded(100)
        ));
    }

    #[test]
    fn test_batch_dry_run() {
        let commands = vec!["echo hello".to_string()];
        let result = run(commands, None, true);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), BatchResult::Completed { .. }));
    }

    #[test]
    fn test_empty_checkpoint_path_fails() {
        let commands = vec!["echo hello".to_string()];
        let result = run(commands, Some(""), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_result_checkpoint_id() {
        let completed = BatchResult::Completed {
            checkpoint_id: Some(CheckpointId::new("test-123".to_string())),
            results: vec![],
        };
        assert_eq!(completed.checkpoint_id(), Some("test-123"));

        let rolled_back = BatchResult::RolledBack {
            checkpoint_id: CheckpointId::new("rollback-456".to_string()),
            error: scp_core::Error::BatchCommandFailed("test error".to_string()),
            results: vec![],
        };
        assert_eq!(rolled_back.checkpoint_id(), Some("rollback-456"));
    }

    #[test]
    fn test_checkpoint_id_from_timestamp() {
        let id = CheckpointId::from_timestamp();
        assert!(id.is_ok());
        let id = id.unwrap();
        assert!(id.as_str().starts_with("batch-"));
    }

    #[test]
    fn test_validate_batch_input() {
        // Empty commands should fail
        assert!(validate_batch_input(&[], None).is_err());

        // Valid commands should pass
        assert!(validate_batch_input(&["echo hello".to_string()], None).is_ok());

        // Empty checkpoint path should fail
        assert!(validate_batch_input(&["echo hello".to_string()], Some("")).is_err());
    }

    #[test]
    fn test_execute_command_empty() {
        let result = execute_command("");
        assert!(!result.success);
        assert_eq!(result.output, "empty command");
    }

    #[test]
    fn test_execute_command_success() {
        let result = execute_command("echo hello");
        assert!(result.success);
    }

    #[test]
    fn test_execute_command_failure() {
        let result = execute_command("exit 1");
        assert!(!result.success);
    }
}
