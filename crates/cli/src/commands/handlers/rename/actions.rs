//! Action functions for the rename command handler (Tier 3).
//!
//! I/O operations that orchestrate session renaming.

use scp_core::{output::Output, validation::domain::validate_input_name, Error, Result};

use super::data::{validate_name_length, validate_session_name, RenameOptions, RenameOutput};

/// Emit a successful rename result: a single JSON object in JSON mode
/// (the global `-f json` switch, via [`Output::json_output`]), the exact
/// previous human prose otherwise. `success_mark` selects the `✓`
/// success form versus the plain info form of the human message.
fn emit_rename(output: &RenameOutput, human: &str, success_mark: bool) {
    if Output::json_output() {
        Output::json(output);
    } else if success_mark {
        Output::success(human);
    } else {
        Output::info(human);
    }
}

/// Execute the rename command with the given options.
///
/// # Errors
///
/// Returns errors for validation failures, session not found,
/// or filesystem/database operation failures.
pub fn run_rename(options: &RenameOptions) -> Result<RenameOutput> {
    // Edge case: rename to same name is a no-op
    if options.old_name == options.new_name {
        let output = RenameOutput {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            dry_run: options.dry_run,
            error: None,
        };
        emit_rename(
            &output,
            &format!(
                "Session '{}' already has that name (no-op)",
                options.old_name
            ),
            false,
        );
        return Ok(output);
    }

    validate_rename_names(options)?;

    if options.dry_run {
        let output = RenameOutput {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            dry_run: true,
            error: None,
        };
        emit_rename(
            &output,
            &format!(
                "[dry-run] Would rename: '{}' -> '{}'",
                options.old_name, options.new_name
            ),
            false,
        );
        return Ok(output);
    }

    perform_rename(options)
}

/// Validate both old and new session names.
fn validate_rename_names(options: &RenameOptions) -> Result<()> {
    validate_input_name(&options.old_name).map_err(|e| {
        Error::invalid_identifier(format!(
            "old session name '{}' is invalid: {e}",
            options.old_name
        ))
    })?;

    validate_input_name(&options.new_name).map_err(|e| {
        Error::invalid_identifier(format!(
            "new session name '{}' is invalid: {e}",
            options.new_name
        ))
    })?;

    validate_name_length(&options.new_name).map_err(Error::validation_error)?;
    validate_session_name(&options.new_name).map_err(Error::validation_error)?;

    Ok(())
}

/// Perform the actual filesystem rename.
///
/// A session that does not exist is an error, not a success: the old code
/// printed "Renamed" even when the old path was absent. A `hd work`
/// session lives at `.scp/workspaces/<name>`; renaming one moves the real
/// git worktree with `git worktree move` (a plain directory rename would
/// orphan the worktree metadata) and renames its session-database row.
/// A plain directory under the current directory keeps the old behaviour
/// of a direct filesystem rename.
fn perform_rename(options: &RenameOptions) -> Result<RenameOutput> {
    let cwd = std::env::current_dir()?;
    let work_path = cwd
        .join(".scp")
        .join("workspaces")
        .join(&options.old_name);
    let plain_path = cwd.join(&options.old_name);

    if work_path.exists() {
        rename_work_session(&cwd, &work_path, options)?;
    } else if plain_path.exists() {
        std::fs::rename(&plain_path, cwd.join(&options.new_name)).map_err(|e| {
            Error::io_error(format!(
                "Failed to rename '{}' to '{}': {e}",
                plain_path.display(),
                options.new_name
            ))
        })?;
    } else {
        return Err(Error::not_found(format!(
            "Session '{}' not found",
            options.old_name
        )));
    }

    let output = RenameOutput {
        success: true,
        old_name: options.old_name.clone(),
        new_name: options.new_name.clone(),
        dry_run: false,
        error: None,
    };
    emit_rename(
        &output,
        &format!(
            "Renamed session '{}' -> '{}'",
            options.old_name, options.new_name
        ),
        true,
    );

    Ok(output)
}

/// Which branch, if any, a session rename should rename.
///
/// A `hd work` session's worktree checks out a branch named after the
/// session, so that branch follows the session name. Any other checked-out
/// branch (a shared branch such as `main`, or a branch the session did not
/// create) is never renamed: renaming `main` because a worktree happened
/// to sit on it would be a destructive surprise, and a detached HEAD has
/// no branch to rename.
fn branch_to_rename<'a>(checked_out_branch: &'a str, old_name: &str) -> Option<&'a str> {
    if checked_out_branch.is_empty() || checked_out_branch != old_name {
        None
    } else {
        Some(checked_out_branch)
    }
}

/// The branch currently checked out in a worktree, or an empty string for
/// a detached HEAD or a git failure (callers treat both as "no branch").
fn checked_out_branch(work_path: &std::path::Path) -> String {
    match std::process::Command::new("git")
        .args([
            "-C",
            &work_path.to_string_lossy(),
            "branch",
            "--show-current",
        ])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Whether a local branch of this name exists.
fn branch_exists(cwd: &std::path::Path, branch: &str) -> bool {
    match std::process::Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(cwd)
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Rename a `hd work` session: move its git worktree, rename the session's
/// own branch when it is named after the session, then rename its
/// session-database row (name and workspace path) so `query` keeps finding
/// it under the new name. A worktree-move failure aborts before any row
/// change, and a branch-rename failure moves the worktree back, so the
/// rename never half-applies and never claims success it did not achieve.
/// `git branch -m` runs from the main worktree: it renames a branch even
/// when that branch is checked out in a linked worktree.
fn rename_work_session(
    cwd: &std::path::Path,
    work_path: &std::path::Path,
    options: &RenameOptions,
) -> Result<()> {
    let new_path = cwd.join(".scp").join("workspaces").join(&options.new_name);
    if new_path.exists() {
        return Err(Error::session_exists(options.new_name.clone()));
    }
    let checked_out = checked_out_branch(work_path);
    let branch = branch_to_rename(&checked_out, &options.old_name);
    if branch.is_some() && branch_exists(cwd, &options.new_name) {
        return Err(Error::vcs_conflict(
            "branch rename",
            format!("branch '{}' already exists", options.new_name),
        ));
    }
    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "move",
            &work_path.to_string_lossy(),
            &new_path.to_string_lossy(),
        ])
        .current_dir(cwd)
        .output()
        .map_err(|e| Error::io_error(format!("Failed to run git worktree move: {e}")))?;
    if !output.status.success() {
        return Err(Error::io_error(format!(
            "Failed to move worktree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    if let Some(branch) = branch {
        let output = std::process::Command::new("git")
            .args(["branch", "-m", branch, &options.new_name])
            .current_dir(cwd)
            .output()
            .map_err(|e| Error::io_error(format!("Failed to run git branch -m: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // Roll the worktree move back so a failed branch rename does
            // not leave dir and branch disagreeing about the session name.
            let _ = std::process::Command::new("git")
                .args([
                    "worktree",
                    "move",
                    &new_path.to_string_lossy(),
                    &work_path.to_string_lossy(),
                ])
                .current_dir(cwd)
                .output();
            return Err(Error::io_error(format!(
                "Failed to rename branch '{branch}' to '{}': {stderr}",
                options.new_name
            )));
        }
    }
    rename_session_row(cwd, options)
}

/// Rename the session-database row for a work session, if one exists.
/// A missing row (or missing database) is not an error: the worktree move
/// above is the rename, and older sessions may have no row yet.
fn rename_session_row(cwd: &std::path::Path, options: &RenameOptions) -> Result<()> {
    let db_path = match crate::commands::handlers::config_ports::resolve_state_db_path(cwd) {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };
    if !db_path.exists() {
        return Ok(());
    }
    let new_workspace = format!(".scp/workspaces/{}", options.new_name);
    rename_session_row_with_db(&db_path, options, &new_workspace)
}

fn rename_session_row_with_db(
    db_path: &std::path::Path,
    options: &RenameOptions,
    new_workspace: &str,
) -> Result<()> {
    use scp_core::infrastructure::database::{DatabaseConfig, DatabaseService};

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
    runtime.block_on(async {
        let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
        let service = scp_core::infrastructure::database::SqliteDatabaseService::new(config).await?;
        service
            .execute_params(
                "UPDATE sessions SET name = ?, workspace = ? WHERE name = ?",
                &[
                    Some(options.new_name.clone()),
                    Some(new_workspace.to_string()),
                    Some(options.old_name.clone()),
                ],
            )
            .await?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::rename::data::{
        is_reserved_name, validate_name_length, validate_session_name, RenameOptions,
    };

    #[test]
    fn run_rename_same_name_is_noop() {
        let options = RenameOptions {
            old_name: "test".to_string(),
            new_name: "test".to_string(),
            dry_run: false,
        };
        let result = run_rename(&options).expect("should succeed");
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn run_rename_dry_run() {
        let options = RenameOptions {
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            dry_run: true,
        };
        let result = run_rename(&options).expect("should succeed");
        assert!(result.success);
        assert!(result.dry_run);
    }

    #[test]
    fn branch_to_rename_follows_session_named_branch() {
        assert_eq!(branch_to_rename("old1", "old1"), Some("old1"));
    }

    #[test]
    fn branch_to_rename_skips_detached_head() {
        assert_eq!(branch_to_rename("", "old1"), None);
    }

    #[test]
    fn branch_to_rename_never_renames_a_different_branch() {
        // A worktree sitting on main (or any branch the session did not
        // create) keeps its branch: renaming it would be destructive.
        assert_eq!(branch_to_rename("main", "old1"), None);
        assert_eq!(branch_to_rename("feature-x", "old1"), None);
    }

    #[test]
    fn run_rename_invalid_new_name() {
        let options = RenameOptions {
            old_name: "old".to_string(),
            new_name: "123invalid".to_string(),
            dry_run: false,
        };
        assert!(run_rename(&options).is_err());
    }

    #[test]
    fn validate_name_length_boundary() {
        assert!(validate_name_length("a").is_ok());
        assert!(validate_name_length(&"a".repeat(64)).is_ok());
        assert!(validate_name_length(&"a".repeat(65)).is_err());
    }

    #[test]
    fn validate_session_name_backslash_rejected() {
        assert!(validate_session_name("test\\name").is_err());
    }

    #[test]
    fn is_reserved_names() {
        assert!(is_reserved_name("main"));
        assert!(is_reserved_name("master"));
        assert!(is_reserved_name("default"));
        assert!(is_reserved_name("trunk"));
        assert!(!is_reserved_name("feature"));
    }
}
