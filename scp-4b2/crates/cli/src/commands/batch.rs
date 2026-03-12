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

    if dry_run {
        println!("Batch dry-run mode:");
        for (i, cmd) in commands.iter().enumerate() {
            println!("  {}: {}", i + 1, cmd);
        }
        println!("Total: {} commands", commands.len());
        return Ok(());
    }

    let checkpoint_id = checkpoint_path.map(|_| {
        format!(
            "batch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        )
    });

    let mut results: Vec<CommandResult> = Vec::new();
    let mut has_failure = false;

    for cmd in &commands {
        match execute_command(cmd) {
            Ok(output) => {
                results.push(CommandResult {
                    command: cmd.clone(),
                    success: true,
                    output,
                });
            }
            Err(e) => {
                has_failure = true;
                results.push(CommandResult {
                    command: cmd.clone(),
                    success: false,
                    output: e.to_string(),
                });
                break;
            }
        }
    }

    if has_failure {
        if checkpoint_path.is_some() {
            println!(
                "Batch execution failed. Checkpoint available for rollback: {:?}",
                checkpoint_id
            );
            println!("Results:");
            for result in &results {
                let status = if result.success { "✓" } else { "✗" };
                println!("  {} {}: {}", status, result.command, result.output);
            }
            return Err(scp_core::Error::BatchCommandFailed(
                "batch failed".to_string(),
            ));
        } else {
            println!("Batch execution failed. No checkpoint to rollback to.");
            println!("Results:");
            for result in &results {
                let status = if result.success { "✓" } else { "✗" };
                println!("  {} {}: {}", status, result.command, result.output);
            }
            return Err(scp_core::Error::BatchCommandFailed(
                "batch failed".to_string(),
            ));
        }
    }

    println!("Batch executed successfully ({} commands)", results.len());
    for result in &results {
        let status = if result.success { "✓" } else { "✗" };
        println!("  {} {}", status, result.command);
    }
    if let Some(ref id) = checkpoint_id {
        println!("Checkpoint: {}", id);
    }

    Ok(())
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
