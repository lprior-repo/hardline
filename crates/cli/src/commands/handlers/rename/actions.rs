//! Action functions for the rename command handler (Tier 3).
//!
//! I/O operations that orchestrate session renaming.

use scp_core::{output::Output, validation::domain::validate_input_name, Error, Result};

use super::data::{validate_name_length, validate_session_name, RenameOptions, RenameOutput};

/// Execute the rename command with the given options.
///
/// # Errors
///
/// Returns errors for validation failures, session not found,
/// or filesystem/database operation failures.
pub fn run_rename(options: &RenameOptions) -> Result<RenameOutput> {
    // Edge case: rename to same name is a no-op
    if options.old_name == options.new_name {
        Output::info(&format!(
            "Session '{}' already has that name (no-op)",
            options.old_name
        ));
        return Ok(RenameOutput {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            dry_run: options.dry_run,
            error: None,
        });
    }

    validate_rename_names(options)?;

    if options.dry_run {
        Output::info(&format!(
            "[dry-run] Would rename: '{}' -> '{}'",
            options.old_name, options.new_name
        ));
        return Ok(RenameOutput {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            dry_run: true,
            error: None,
        });
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

    Output::success(&format!(
        "Renamed session '{}' -> '{}'",
        options.old_name, options.new_name
    ));

    Ok(RenameOutput {
        success: true,
        old_name: options.old_name.clone(),
        new_name: options.new_name.clone(),
        dry_run: false,
        error: None,
    })
}

/// Rename a `hd work` session: move its git worktree, then rename its
/// session-database row (name and workspace path) so `query` keeps finding
/// it under the new name. A worktree-move failure aborts before any row
/// change, so the rename never half-applies.
fn rename_work_session(
    cwd: &std::path::Path,
    work_path: &std::path::Path,
    options: &RenameOptions,
) -> Result<()> {
    let new_path = cwd
        .join(".scp")
        .join("workspaces")
        .join(&options.new_name);
    if new_path.exists() {
        return Err(Error::session_exists(options.new_name.clone()));
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
