#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Atomic batch command - multi-operation with transactional rollback
//!
//! Provides atomic execution semantics: all operations succeed OR all roll back.
//!
//! # Usage
//!
//! ```ignore
//! let request = BatchRequest {
//!     atomic: true,
//!     operations: vec![
//!         BatchOperation { command: "add", args: vec!["session-1"], .. },
//!         BatchOperation { command: "sync", args: vec!["session-1"], .. },
//!     ],
//! };
//!
//! let response = execute_batch(request, &db).await?;
//! ```
//!
//! # Invariants (`DbC`)
//!
//! - **Pre**: All operations in the request are valid commands
//! - **Post**: Either all operations succeeded, or all were rolled back to checkpoint
//!
//! # EARS Requirements
//!
//! - **When**: `{cmd:batch, atomic:true, ops:[...]}`
//! - **Then**: Execute all operations or rollback all using checkpoint
//! - **Invariant**: Atomic transactions, checkpoint before execution

#[cfg(test)]
mod tests;

use futures::StreamExt;
use isolate_core::{
    json::SchemaEnvelope,
    OutputFormat,
    checkpoint::AutoCheckpoint,
    checkpoint::OperationRisk,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::{IsolateError, Result};

/// Request for atomic batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    /// Enable atomic mode (all succeed or all rollback)
    #[serde(default)]
    pub atomic: bool,

    /// Preview execution without changes
    #[serde(default)]
    pub dry_run: bool,

    /// Operations to execute in order
    pub operations: Vec<BatchOperation>,
}

/// A single operation within a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOperation {
    /// Command name (without 'isolate' prefix)
    pub command: String,

    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Optional ID for referencing in results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Whether this operation is optional (continue on failure)
    #[serde(default)]
    pub optional: bool,
}

/// Response from atomic batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponse {
    /// Overall success (all non-optional operations succeeded in atomic mode)
    pub success: bool,

    /// Total operations
    pub total: usize,

    /// Operations that succeeded
    pub succeeded: usize,

    /// Operations that failed
    pub failed: usize,

    /// Operations that were skipped
    pub skipped: usize,

    /// Individual operation results
    pub results: Vec<BatchItemResult>,

    /// Whether this was executed in atomic mode
    pub atomic: bool,

    /// Checkpoint ID created (if atomic mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,

    /// Whether rollback was performed
    pub rolled_back: bool,
}

/// Result of a single batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResult {
    /// Operation ID if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Full command string
    pub command: String,

    /// Whether this operation succeeded
    pub success: bool,

    /// Status of operation execution
    pub status: BatchItemStatus,

    /// Output from operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Status of a batch item execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BatchItemStatus {
    /// Operation succeeded
    Succeeded,
    /// Operation failed
    Failed,
    /// Operation was skipped due to previous failure
    Skipped,
    /// Operation was rolled back
    RolledBack,
    /// Operation was previewed (dry run)
    DryRun,
}

/// Execute atomic batch with checkpoint-based rollback.
///
/// # EARS
///
/// - **When**: atomic=true with valid operations
/// - **Then**: Execute all operations, rollback all on failure
/// - **Invariant**: Checkpoint created before any operation executes
///
/// # `DbC` (Design by Contract)
///
/// - **Pre**: All operations in `request.operations` are valid isolate commands
/// - **Post**: Either all non-optional operations succeeded, or state is restored to checkpoint
///
/// # Returns
///
/// - `Ok(BatchResponse)` with results and rollback status
/// - `Err(IsolateError)` if checkpoint creation fails or critical error occurs
#[allow(clippy::too_many_lines)]
pub async fn execute_batch(
    request: BatchRequest,
    db: &SqlitePool,
    format: OutputFormat,
) -> Result<BatchResponse> {
    // Phase 1: Create checkpoint if atomic mode (and not dry-run)
    let checkpoint_guard = if request.atomic && !request.dry_run {
        let auto_cp = AutoCheckpoint::new(db.clone());
        auto_cp
            .guard_if_risky(OperationRisk::Risky)
            .await
            .map_err(|e| IsolateError::OperationFailed(format!("Failed to create checkpoint: {e}")))?
    } else {
        None
    };

    // Phase 2: Execute operations in order
    let atomic = request.atomic;
    let dry_run = request.dry_run;
    let (results, _) = futures::stream::iter(&request.operations)
        .fold(
            (Vec::with_capacity(request.operations.len()), false),
            |(mut results, mut should_stop), operation| async move {
                let result = if should_stop {
                    make_skipped_result(operation, "Previous operation failed")
                } else if dry_run {
                    make_dry_run_result(operation)
                } else {
                    execute_batch_operation(operation).await
                };

                // Track if we should stop (required operation failed in atomic mode)
                // In dry-run, we never fail, so we don't stop unless we want to simulate failure?
                // For now assume dry-run always succeeds.
                let is_required = !operation.optional;
                let is_failure = !result.success;
                let stop_now = !dry_run && atomic && is_required && is_failure;

                if stop_now {
                    should_stop = true;
                }

                results.push(result);
                (results, should_stop)
            },
        )
        .await;

    // Phase 3: Compute final result
    let succeeded = results
        .iter()
        .filter(|r| matches!(r.status, BatchItemStatus::Succeeded))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r.status, BatchItemStatus::Failed))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r.status, BatchItemStatus::Skipped))
        .count();
    let success = if request.atomic {
        // Atomic mode: all required operations must succeed
        results
            .iter()
            .all(|r| r.success || operation_is_optional_by_id(&request.operations, r.id.as_ref()))
    } else {
        // Non-atomic mode: success if no required failures
        results
            .iter()
            .all(|r| r.success || matches!(r.status, BatchItemStatus::Skipped))
    };

    let checkpoint_id = checkpoint_guard.as_ref().map(|g| g.id().to_string());

    // Phase 4: Commit or rollback based on result
    let rolled_back = if !dry_run && request.atomic && !success {
        if let Some(guard) = checkpoint_guard {
            guard
                .rollback()
                .await
                .map_err(|e| IsolateError::OperationFailed(format!("Failed to rollback: {e}")))?;
            true
        } else {
            false
        }
    } else if request.atomic && success {
        if let Some(guard) = checkpoint_guard {
            guard
                .commit()
                .await
                .map_err(|e| IsolateError::OperationFailed(format!("Failed to commit checkpoint: {e}")))?;
        }
        false
    } else {
        false
    };

    // Update results with rollback status if rolled back
    let results = if rolled_back {
        results
            .into_iter()
            .map(|mut r| {
                if matches!(r.status, BatchItemStatus::Succeeded) {
                    r.status = BatchItemStatus::RolledBack;
                }
                r
            })
            .collect()
    } else {
        results
    };

    let response = BatchResponse {
        success,
        total: results.len(),
        succeeded,
        failed,
        skipped,
        results,
        atomic: request.atomic,
        checkpoint_id,
        rolled_back,
    };

    // Phase 5: Output response
    if format.is_json() {
        let envelope = SchemaEnvelope::new("batch-response", "single", &response);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .map_err(|e| { IsolateError::OperationFailed(format!("Failed to serialize response: {e}")) })?
        );
    } else {
        print_batch_human(&response);
    }

    Ok(response)
}

/// Execute a single batch operation.
async fn execute_batch_operation(operation: &BatchOperation) -> BatchItemResult {
    let start = std::time::Instant::now();
    let command_str = format!("{} {}", operation.command, operation.args.join(" "));

    execute_command(&operation.command, &operation.args)
        .await
        .map_or_else(
            |e| BatchItemResult {
                id: operation.id.clone(),
                command: command_str.clone(),
                success: false,
                status: BatchItemStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                duration_ms: to_duration_ms(start.elapsed()),
            },
            |output| BatchItemResult {
                id: operation.id.clone(),
                command: command_str.clone(),
                success: true,
                status: BatchItemStatus::Succeeded,
                output: Some(output),
                error: None,
                duration_ms: to_duration_ms(start.elapsed()),
            },
        )
}

/// Create a dry run operation result.
fn make_dry_run_result(operation: &BatchOperation) -> BatchItemResult {
    BatchItemResult {
        id: operation.id.clone(),
        command: format!("{} {}", operation.command, operation.args.join(" ")),
        success: true,
        status: BatchItemStatus::DryRun,
        output: Some("(dry run)".to_string()),
        error: None,
        duration_ms: Some(0),
    }
}

/// Create a skipped operation result.
fn make_skipped_result(operation: &BatchOperation, reason: &str) -> BatchItemResult {
    BatchItemResult {
        id: operation.id.clone(),
        command: format!("{} {}", operation.command, operation.args.join(" ")),
        success: false,
        status: BatchItemStatus::Skipped,
        output: None,
        error: Some(reason.to_string()),
        duration_ms: None,
    }
}

/// Execute a command asynchronously and capture output.
async fn execute_command(command: &str, args: &[String]) -> Result<String> {
    // Robustly handle 'isolate' prefix and argument shifting
    let (actual_command, actual_args) = if command == "isolate" {
        if args.is_empty() {
            return Err(IsolateError::OperationFailed("Empty command after 'isolate'".to_string()));
        }
        (args[0].as_str(), &args[1..])
    } else if let Some(stripped) = command.strip_prefix("isolate ") {
        (stripped, args)
    } else {
        (command, args)
    };

    // Determine the executable. If it's a known isolate subcommand, we use the current executable.
    // This is more robust than assuming "isolate" is in PATH.
    // SECURITY: current_exe is acceptable here - we are invoking our own isolate binary
    // for batch command execution, not for security-critical path validation.
    // The command/args are controlled by our code, not external input.
    let current_exe =
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("isolate"));

    let output = tokio::process::Command::new(current_exe)
        .arg(actual_command)
        .args(actual_args)
        .output()
        .await
        .map_err(|e| IsolateError::OperationFailed(format!("Failed to execute {actual_command}: {e}")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        Err(IsolateError::OperationFailed(error_msg))
    }
}

/// Convert Duration to milliseconds, clamping to u64 range.
fn to_duration_ms(duration: std::time::Duration) -> Option<u64> {
    let ms = duration.as_millis();
    ms.try_into().ok()
}

/// Check if an operation is optional by ID.
fn operation_is_optional_by_id(operations: &[BatchOperation], id: Option<&String>) -> bool {
    id.is_some_and(|op_id| {
        operations
            .iter()
            .find(|op| op.id.as_ref() == Some(op_id))
            .is_some_and(|op| op.optional)
    })
}

/// Print human-readable batch response.
fn print_batch_human(response: &BatchResponse) {
    println!(
        "Batch (atomic={}): {} total, {} succeeded, {} failed, {} skipped",
        response.atomic, response.total, response.succeeded, response.failed, response.skipped
    );

    if let Some(cp_id) = &response.checkpoint_id {
        println!("Checkpoint: {cp_id}");
    }

    if response.rolled_back {
        println!("Status: ROLLED BACK (all operations undone)");
    } else if response.success {
        println!("Status: SUCCESS (all operations committed)");
    } else {
        println!("Status: FAILED (some operations failed)");
    }

    println!();

    response.results.iter().for_each(|item_result| {
        let status_icon = match item_result.status {
            BatchItemStatus::Succeeded => "✓",
            BatchItemStatus::Failed => "✗",
            BatchItemStatus::Skipped => "○",
            BatchItemStatus::RolledBack => "↩",
            BatchItemStatus::DryRun => "▷",
        };

        let id_str = item_result
            .id
            .as_ref()
            .map(|id| format!("[{id}] "))
            .unwrap_or_default();

        println!("{}{}{}", status_icon, id_str, item_result.command);

        if let Some(e) = item_result.error.as_ref() {
            println!("    Error: {e}");
        }

        if let Some(ms) = item_result.duration_ms {
            println!("    Duration: {ms}ms");
        }
    });

    println!();
}