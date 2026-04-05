//! Action functions for the branch command handler (Tier 3).
//!
//! I/O operations that orchestrate branch create, delete, and rename.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    validate_branch_name, BranchCreateOptions, BranchCreateOutput, BranchDeleteOptions,
    BranchRenameOptions,
};

/// Execute the branch create command.
///
/// # Errors
///
/// Returns errors for validation failures or VCS operation failures.
pub fn run_branch_create(options: &BranchCreateOptions) -> Result<BranchCreateOutput> {
    validate_branch_name(&options.name).map_err(|e| Error::validation_error(e))?;

    if options.dry_run {
        Output::info(&format!("[dry-run] Would create branch: '{}'", options.name));
        return Ok(BranchCreateOutput {
            success: true,
            branch_name: options.name.clone(),
            dry_run: true,
            error: None,
        });
    }

    let cwd = std::env::current_dir()?;
    let backend = scp_core::vcs::create_backend(&cwd)?;
    backend.create_branch(&options.name)?;

    Output::success(&format!("Created branch '{}'", options.name));

    Ok(BranchCreateOutput {
        success: true,
        branch_name: options.name.clone(),
        dry_run: false,
        error: None,
    })
}

/// Execute the branch delete command.
///
/// # Errors
///
/// Returns errors for validation failures, protected branch, or VCS operation failures.
pub fn run_branch_delete(options: &BranchDeleteOptions) -> Result<()> {
    use super::data::is_protected_branch;

    if is_protected_branch(&options.name) {
        return Err(Error::invalid_state(format!(
            "Cannot delete protected branch '{}'",
            options.name
        )));
    }

    if options.dry_run {
        Output::info(&format!("[dry-run] Would delete branch: '{}'", options.name));
        return Ok(());
    }

    let cwd = std::env::current_dir()?;
    let force_flag = if options.force { "-D" } else { "-d" };
    let output = std::process::Command::new("git")
        .args(["branch", force_flag, &options.name])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            Output::success(&format!("Deleted branch '{}'", options.name));
            Ok(())
        } else {
            // Git returned non-zero but with info in stderr
            Output::info(&stderr);
            Ok(())
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::vcs_conflict("branch delete", stderr))
    }
}

/// Execute the branch rename command.
///
/// # Errors
///
/// Returns errors for validation failures, protected branch, or VCS operation failures.
pub fn run_branch_rename(options: &BranchRenameOptions) -> Result<()> {
    use super::data::is_protected_branch;

    if is_protected_branch(&options.old_name) {
        return Err(Error::invalid_state(format!(
            "Cannot rename protected branch '{}'",
            options.old_name
        )));
    }

    validate_branch_name(&options.new_name).map_err(|e| Error::validation_error(e))?;

    // Edge case: rename to same name is a no-op
    if options.old_name == options.new_name {
        Output::info(&format!(
            "Branch '{}' already has that name (no-op)",
            options.old_name
        ));
        return Ok(());
    }

    if options.dry_run {
        Output::info(&format!(
            "[dry-run] Would rename branch: '{}' -> '{}'",
            options.old_name, options.new_name
        ));
        return Ok(());
    }

    let cwd = std::env::current_dir()?;
    let output = std::process::Command::new("git")
        .args(["branch", "-m", &options.old_name, &options.new_name])
        .current_dir(&cwd)
        .output()?;

    if output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() || stderr.contains("Renamed") {
            Output::success(&format!(
                "Renamed branch '{}' -> '{}'",
                options.old_name, options.new_name
            ));
        } else {
            Output::info(&stderr);
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::vcs_conflict("branch rename", stderr))
    }
}

