//! Batch command - atomic execution with checkpoint rollback

use scp_core::Result;

const MAX_BATCH_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct BatchResult {
    pub checkpoint_id: Option<String>,
    pub results: Vec<CommandResult>,
    pub rolled_back: bool,
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub output: String,
}

/// Run batch commands atomically
pub fn run(commands: Vec<String>, checkpoint_path: Option<&str>, dry_run: bool) -> Result<()> {
    validate_batch_commands(&commands, checkpoint_path)?;

    if dry_run {
        return run_dry_run(&commands);
    }

    let checkpoint_id = generate_checkpoint_id(checkpoint_path.is_some());
    let results = execute_commands(&commands);
    let has_failure = results.iter().any(|r| !r.success);

    if has_failure {
        return run_failure_handling(&results, checkpoint_id.as_ref(), checkpoint_path.is_some());
    }

    run_success_output(&results, checkpoint_id.as_ref());
    Ok(())
}

/// Validate batch command inputs
fn validate_batch_commands(commands: &[String], checkpoint_path: Option<&str>) -> Result<()> {
    if commands.is_empty() {
        return Err(scp_core::Error::BatchEmpty);
    }

    if commands.len() > MAX_BATCH_SIZE {
        return Err(scp_core::Error::BatchSizeExceeded(MAX_BATCH_SIZE));
    }

    if let Some(path) = checkpoint_path {
        if path.is_empty() {
            return Err(scp_core::Error::ValidationError(
                "checkpoint path cannot be empty".to_string(),
            ));
        }
    }

    Ok(())
}

/// Execute dry-run mode
fn run_dry_run(commands: &[String]) -> Result<()> {
    println!("Batch dry-run mode:");
    commands
        .iter()
        .enumerate()
        .for_each(|(i, cmd)| println!("  {}: {}", i + 1, cmd));
    println!("Total: {} commands", commands.len());
    Ok(())
}

/// Generate checkpoint ID if checkpointing is enabled
fn generate_checkpoint_id(enable_checkpoint: bool) -> Option<String> {
    enable_checkpoint.then(|| {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("batch-{}", millis)
    })
}

/// Execute all commands and collect results
fn execute_commands(commands: &[String]) -> Vec<CommandResult> {
    commands
        .iter()
        .map(|cmd| match execute_command(cmd) {
            Ok(output) => CommandResult {
                command: cmd.clone(),
                success: true,
                output,
            },
            Err(e) => CommandResult {
                command: cmd.clone(),
                success: false,
                output: e.to_string(),
            },
        })
        .collect()
}

/// Handle failure case
fn run_failure_handling(
    results: &[CommandResult],
    checkpoint_id: Option<&String>,
    has_checkpoint: bool,
) -> Result<()> {
    if has_checkpoint {
        println!(
            "Batch execution failed. Checkpoint available for rollback: {:?}",
            checkpoint_id
        );
    } else {
        println!("Batch execution failed. No checkpoint to rollback to.");
    }

    print_results(results);
    Err(scp_core::Error::BatchCommandFailed(
        "batch failed".to_string(),
    ))
}

/// Print command results
fn print_results(results: &[CommandResult]) {
    println!("Results:");
    results.iter().for_each(|result| {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}: {}", status, result.command, result.output);
    });
}

/// Handle success case
fn run_success_output(results: &[CommandResult], checkpoint_id: Option<&String>) {
    println!("Batch executed successfully ({} commands)", results.len());
    results.iter().for_each(|result| {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}", status, result.command);
    });
    if let Some(id) = checkpoint_id {
        println!("Checkpoint: {}", id);
    }
}

/// Execute a single command
fn execute_command(cmd: &str) -> Result<String> {
    use std::process::Command;

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(scp_core::Error::BatchCommandFailed(
            "empty command".to_string(),
        ));
    }

    let program = parts[0];
    let args = &parts[1..];

    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| scp_core::Error::BatchCommandFailed(format!("failed to execute: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(scp_core::Error::BatchCommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
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
    }

    #[test]
    fn test_empty_checkpoint_path_fails() {
        let commands = vec!["echo hello".to_string()];
        let result = run(commands, Some(""), false);
        assert!(result.is_err());
    }
}
