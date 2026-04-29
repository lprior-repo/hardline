//! Action functions for the rename command handler (Tier 3).
//!
//! I/O operations that orchestrate session renaming.

use scp_core::{
    output::Output,
    validation::domain::validate_input_name,
    Error, Result,
};

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
        Error::invalid_identifier(format!("old session name '{}' is invalid: {e}", options.old_name))
    })?;

    validate_input_name(&options.new_name).map_err(|e| {
        Error::invalid_identifier(format!("new session name '{}' is invalid: {e}", options.new_name))
    })?;

    validate_name_length(&options.new_name).map_err(Error::validation_error)?;
    validate_session_name(&options.new_name).map_err(Error::validation_error)?;

    Ok(())
}

/// Perform the actual filesystem rename.
fn perform_rename(options: &RenameOptions) -> Result<RenameOutput> {
    let cwd = std::env::current_dir()?;
    let old_path = cwd.join(&options.old_name);
    let new_path = cwd.join(&options.new_name);

    if old_path.exists() {
        std::fs::rename(&old_path, &new_path).map_err(|e| {
            Error::io_error(format!(
                "Failed to rename '{}' to '{}': {e}",
                old_path.display(),
                new_path.display()
            ))
        })?;
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
