//! Action layer for stack range-diff - I/O operations via git CLI.
//!
//! Executes `git range-diff` and returns structured results.

use std::path::Path;
use std::process::Command;

use super::calc::{build_git_args, build_result, validate_refs};
use super::data::{RangeDiffError, RangeDiffOptions, RangeDiffResult};

/// Run the stack range-diff operation.
///
/// Executes `git range-diff` with the given options and returns
/// a structured result.
///
/// # Errors
///
/// Returns `RangeDiffError` for any failure:
/// - `InvalidRef` if required refs are empty
/// - `CommandFailed` if git range-diff exits non-zero
/// - `IoError` if the git process cannot be started
pub fn run_range_diff(
    workdir: &Path,
    options: &RangeDiffOptions,
) -> Result<RangeDiffResult, RangeDiffError> {
    // Validate refs before executing
    validate_refs(options)?;

    let args = build_git_args(options);

    let output = Command::new("git")
        .args(&args)
        .current_dir(workdir)
        .output()
        .map_err(|e| RangeDiffError::IoError(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // git range-diff returns non-zero for some diff findings,
        // but real errors contain "fatal:" or "error:"
        if stderr.contains("fatal:") || stderr.contains("error:") {
            return Err(RangeDiffError::CommandFailed { stderr });
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(build_result(&stdout))
}

/// Run range-diff comparing two branch ranges.
///
/// Convenience wrapper that builds options from branch names.
///
/// # Errors
///
/// Same as `run_range_diff`.
pub fn run_range_diff_branches(
    workdir: &Path,
    old_base: &str,
    old_tip: &str,
    new_base: &str,
    new_tip: &str,
) -> Result<RangeDiffResult, RangeDiffError> {
    let options = RangeDiffOptions {
        base_a: old_base.to_string(),
        tip_a: old_tip.to_string(),
        base_b: new_base.to_string(),
        tip_b: new_tip.to_string(),
        ..Default::default()
    };

    run_range_diff(workdir, &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_range_diff_validates_empty_refs() {
        let tmp = std::env::temp_dir();
        let opts = RangeDiffOptions {
            base_a: String::new(),
            tip_a: "feat".to_string(),
            base_b: "main".to_string(),
            tip_b: "feat".to_string(),
            ..Default::default()
        };
        let result = run_range_diff(&tmp, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("base_a"));
    }

    #[test]
    fn run_range_diff_branches_validates() {
        let tmp = std::env::temp_dir();
        // Empty old_base should fail validation
        let result = run_range_diff_branches(&tmp, "", "tip", "main", "tip");
        assert!(result.is_err());
    }

    #[test]
    fn error_display_command_failed() {
        let err = RangeDiffError::CommandFailed {
            stderr: "fatal: bad revision".to_string(),
        };
        assert!(err.to_string().contains("fatal: bad revision"));
    }

    #[test]
    fn error_display_io() {
        let err = RangeDiffError::IoError("permission denied".to_string());
        assert!(err.to_string().contains("permission denied"));
    }
}
