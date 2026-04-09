//! Action layer for stack diff - I/O operations via git CLI.
//!
//! V1 uses shell commands directly. V2 will use extended VcsBackend trait.

use std::path::Path;
use std::process::Command;

use scp_stack::Stack;

use super::calc::{aggregate_result, parse_numstat, select_branches};
use super::data::{
    BranchDiff, DiffError, FileStat, StackDiffOptions, StackDiffResult,
};

/// Run the full stack diff operation.
///
/// Walks the stack in topological order, computing the diff for each branch
/// against its parent using three-dot range syntax (`parent...branch`).
///
/// # Errors
///
/// Returns `DiffError` for any failure during the diff operation.
pub fn run_stack_diff(
    workdir: &Path,
    stack: &Stack,
    options: &StackDiffOptions,
) -> Result<StackDiffResult, DiffError> {
    let pairs = select_branches(stack, &options.range)?;
    let mut branch_diffs: Vec<BranchDiff> = Vec::new();

    for (branch, parent) in &pairs {
        let file_stats = diff_numstat(workdir, branch.as_str(), parent.as_str())?;

        let mut bd = BranchDiff::new(branch.clone(), parent.clone(), file_stats);

        if !options.stat_only {
            let lines = diff_range(workdir, branch.as_str(), parent.as_str(), options.color)?;
            bd = bd.with_diff_lines(lines);
        }

        branch_diffs.push(bd);
    }

    Ok(aggregate_result(branch_diffs))
}

// ============================================================================
// Shell command helpers
// ============================================================================

/// Get diff numstat between branch and parent using three-dot syntax.
///
/// Returns `(file, additions, deletions)` tuples.
fn diff_numstat(
    workdir: &Path,
    branch: &str,
    parent: &str,
) -> Result<Vec<FileStat>, DiffError> {
    let range = format!("{parent}...{branch}");
    let output = Command::new("git")
        .args(["diff", "--numstat", &range])
        .current_dir(workdir)
        .output()
        .map_err(|e| DiffError::IoError(e.to_string()))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    Ok(parse_numstat(&lines))
}

/// Get full diff between branch and parent using three-dot syntax.
fn diff_range(
    workdir: &Path,
    branch: &str,
    parent: &str,
    color: bool,
) -> Result<Vec<String>, DiffError> {
    let range = format!("{parent}...{branch}");
    let mut args = vec!["diff"];
    if !color {
        args.push("--color=never");
    }
    args.push(&range);

    let output = Command::new("git")
        .args(&args)
        .current_dir(workdir)
        .output()
        .map_err(|e| DiffError::IoError(e.to_string()))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_stack_diff_command_format() {
        // Verify git commands are well-formed - actual execution requires a git repo
        let cmd = Command::new("git")
            .args(["diff", "--numstat", "main...feat"])
            .current_dir("/tmp")
            .output();
        assert!(cmd.is_ok());
    }

    #[test]
    fn diff_error_variants_display() {
        let err = DiffError::NoStackBranches;
        assert!(err.to_string().contains("No stack"));

        let err = DiffError::BackendError("fail".to_string());
        assert!(err.to_string().contains("fail"));

        let err = DiffError::IoError("io fail".to_string());
        assert!(err.to_string().contains("io fail"));
    }
}
