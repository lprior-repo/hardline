//! Action functions for the branch command handler (Tier 3).
//!
//! I/O operations that orchestrate branch create, delete, and rename.

use scp_core::{output::Output, Error, Result};

/// Whether a local branch exists, via the same git the real rename path
/// shells out to. A missing ref (or a git failure) means "does not exist".
fn branch_exists(cwd: &std::path::Path, name: &str) -> bool {
    let reference = format!("refs/heads/{name}");
    match std::process::Command::new("git")
        .args(["show-ref", "--verify", "--quiet", &reference])
        .current_dir(cwd)
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

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
    validate_branch_name(&options.name).map_err(Error::validation_error)?;

    if options.dry_run {
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "branch": options.name,
                "result": "created",
                "dry_run": true,
            }));
        } else {
            Output::info(&format!(
                "[dry-run] Would create branch: '{}'",
                options.name
            ));
        }
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

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "branch": options.name,
            "result": "created",
        }));
    } else {
        Output::success(&format!("Created branch '{}'", options.name));
    }

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
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "branch": options.name,
                "result": "deleted",
                "dry_run": true,
            }));
        } else {
            Output::info(&format!(
                "[dry-run] Would delete branch: '{}'",
                options.name
            ));
        }
        return Ok(());
    }

    let cwd = std::env::current_dir()?;
    let force_flag = if options.force { "-D" } else { "-d" };
    let output = std::process::Command::new("git")
        .args(["branch", force_flag, &options.name])
        .current_dir(&cwd)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "branch": options.name,
                "result": "deleted",
            }));
        } else {
            Output::success(&format!("Deleted branch '{}'", options.name));
        }
        Ok(())
    } else {
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

    validate_branch_name(&options.new_name).map_err(Error::validation_error)?;

    // Edge case: rename to same name is a no-op
    if options.old_name == options.new_name {
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "old_name": options.old_name,
                "new_name": options.new_name,
                "result": "no-op",
            }));
        } else {
            Output::info(&format!(
                "Branch '{}' already has that name (no-op)",
                options.old_name
            ));
        }
        return Ok(());
    }

    if options.dry_run {
        // A dry-run must predict the real rename, not bless it: the
        // real path fails 51 when the source branch does not exist or
        // the target already exists, so the dry-run fails the same way
        // instead of printing a success for a rename that cannot happen.
        let cwd = std::env::current_dir()?;
        if !branch_exists(&cwd, &options.old_name) {
            return Err(Error::vcs_conflict(
                "branch rename",
                format!("error: branch '{}' not found", options.old_name),
            ));
        }
        if branch_exists(&cwd, &options.new_name) {
            return Err(Error::vcs_conflict(
                "branch rename",
                format!("error: A branch named '{}' already exists", options.new_name),
            ));
        }
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "old_name": options.old_name,
                "new_name": options.new_name,
                "result": "renamed",
                "dry_run": true,
            }));
        } else {
            Output::info(&format!(
                "[dry-run] Would rename branch: '{}' -> '{}'",
                options.old_name, options.new_name
            ));
        }
        return Ok(());
    }

    let cwd = std::env::current_dir()?;
    let output = std::process::Command::new("git")
        .args(["branch", "-m", &options.old_name, &options.new_name])
        .current_dir(&cwd)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        if Output::json_output() {
            // As in delete: the stderr text is never printed on this
            // path (the success line discards it, the info line only
            // runs when it is empty), so there is nothing to fold in.
            Output::json(&serde_json::json!({
                "old_name": options.old_name,
                "new_name": options.new_name,
                "result": "renamed",
            }));
        } else if !stderr.is_empty() || stderr.contains("Renamed") {
            Output::success(&format!(
                "Renamed branch '{}' -> '{}'",
                options.old_name, options.new_name
            ));
        } else {
            Output::info(&stderr);
        }
        Ok(())
    } else {
        Err(Error::vcs_conflict("branch rename", stderr))
    }
}
