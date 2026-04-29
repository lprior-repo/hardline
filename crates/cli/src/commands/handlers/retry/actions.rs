//! Action functions for the retry command handler (Tier 3).
//!
//! I/O operations that orchestrate retrying the last failed VCS operation.

use std::path::Path;

use scp_core::{output::Output, Error, Result};

use super::data::{LastOperation, RetryOptions, RetryOutput, LAST_OPERATION_PATH};

// ============================================================================
// Public API
// ============================================================================

/// Retry the last failed VCS operation.
///
/// Reads `.hd/last_operation.json`, checks if the last operation failed,
/// and reports information about it. Full re-execution is a future enhancement.
///
/// # Errors
///
/// Returns errors for file I/O failures or malformed operation log.
pub fn run_retry(opts: RetryOptions) -> Result<RetryOutput> {
    let op = read_last_operation()?;

    if op.succeeded {
        return Ok(RetryOutput {
            success: false,
            attempts: 0,
            message: format!(
                "Last operation '{}' already succeeded at {}. Nothing to retry.",
                op.operation, op.timestamp
            ),
        });
    }

    if opts.verbose {
        Output::info(&format!("Last operation: {}", op.operation));
        Output::info(&format!("Arguments: {}", op.args.join(" ")));
        Output::info(&format!(
            "Error: {}",
            op.error.as_deref().unwrap_or("unknown")
        ));
    }

    Ok(RetryOutput {
        success: false,
        attempts: 0,
        message: format!(
            "Last failed operation: '{}' at {}. Error: {}. \
             Re-execution not yet implemented — use the command manually.",
            op.operation,
            op.timestamp,
            op.error.as_deref().unwrap_or("unknown error")
        ),
    })
}

/// Record a VCS operation to the last-operation log.
///
/// Writes to `.hd/last_operation.json`. If the `.hd` directory does not exist,
/// it is created. If the file already exists it is overwritten.
///
/// # Errors
///
/// Returns errors for directory creation or file write failures.
pub fn record_operation(op: LastOperation) -> Result<()> {
    let path = Path::new(LAST_OPERATION_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io_error(format!("Failed to create {}: {e}", parent.display())))?;
    }

    let json = serde_json::to_string_pretty(&op)
        .map_err(|e| Error::internal(format!("Failed to serialize operation: {e}")))?;

    std::fs::write(path, json)
        .map_err(|e| Error::io_error(format!("Failed to write {}: {e}", path.display())))?;

    Ok(())
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Read and deserialize the last operation from disk.
///
/// # Errors
///
/// Returns `Error::not_found` if the log file does not exist.
/// Returns `Error::internal` if the file cannot be parsed as JSON.
fn read_last_operation() -> Result<LastOperation> {
    let path = Path::new(LAST_OPERATION_PATH);
    if !path.exists() {
        return Err(Error::not_found(format!(
            "No operation log found at {}",
            path.display()
        )));
    }

    let contents = std::fs::read_to_string(path).map_err(|e| {
        Error::io_error(format!("Failed to read {}: {e}", path.display()))
    })?;

    serde_json::from_str(&contents).map_err(|e| {
        Error::internal(format!(
            "Malformed operation log at {}: {e}",
            path.display()
        ))
    })
}
