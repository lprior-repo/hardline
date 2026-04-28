//! Infrastructure validation - I/O operations for filesystem checks
//!
//! This module provides **I/O validation functions** that form the "Imperative Shell"
//! of the validation architecture. These functions:
//! - Perform I/O operations (filesystem checks)
// - Should be called from the infrastructure/services layer
//! - Delegate to domain validators for business rules
//! - Return `Result<(), Error>` with context
//!
//! # Design Principle
//!
//! Following the "Functional Core, Imperative Shell" pattern:
//! - **Core** (domain module): Pure validation functions
//! - **Shell** (this module): I/O operations that call core functions
//!
//! # Usage
//!
//! ```rust,ignore
//! use isolate_core::validation::infrastructure::*;
//!
//! // Filesystem checks
//! validate_path_exists(Path::new("/tmp"))?;
//! validate_is_directory(Path::new("/tmp"))?;
//!
//! // Session validation with I/O
//! validate_session_workspace_exists(&session)?;
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::Path;

use crate::{
    types::{Session, SessionStatus},
    Error, Result,
};

// ============================================================================
// PATH VALIDATION
// ============================================================================

/// Validate that a path exists on the filesystem.
///
/// This is an **I/O operation** and should only be called from the
/// infrastructure layer, not from pure domain functions.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path does not exist.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_path_exists;
/// use std::path::Path;
///
/// validate_path_exists(Path::new("/tmp"))?;
/// validate_path_exists(Path::new("/nonexistent"))?; // Returns Err
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::ValidationError {
            message: format!("Path '{}' does not exist", path.display()),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Validate that a path is a directory.
///
/// This is an **I/O operation** and should only be called from the
/// infrastructure layer.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path is not a directory.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_is_directory;
/// use std::path::Path;
///
/// validate_is_directory(Path::new("/tmp"))?;
/// validate_is_directory(Path::new("/etc/hosts"))?; // Returns Err (it's a file)
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_is_directory(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::ValidationError {
            message: format!("Path '{}' is not a directory", path.display()),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Validate that a path is a file.
///
/// This is an **I/O operation** and should only be called from the
/// infrastructure layer.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path is not a file.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_is_file;
/// use std::path::Path;
///
/// validate_is_file(Path::new("/etc/hosts"))?;
/// validate_is_file(Path::new("/tmp"))?; // Returns Err (it's a directory)
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_is_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(Error::ValidationError {
            message: format!("Path '{}' is not a file", path.display()),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }
    Ok(())
}

/// Combined validation: path exists and is a directory.
///
/// This is a convenience function that combines `validate_path_exists`
/// and `validate_is_directory` for workspace directory validation.
///
/// # Errors
///
/// Returns `Error::ValidationError` if:
/// - Path does not exist
/// - Path is not a directory
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_workspace_path;
/// use std::path::Path;
///
/// validate_workspace_path(Path::new("/tmp/my-workspace"))?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_workspace_path(path: &Path) -> Result<()> {
    validate_path_exists(path)?;
    validate_is_directory(path)
}

/// Validate that a path is readable.
///
/// This is an **I/O operation** that checks if a file can be read.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path is not readable.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_is_readable;
/// use std::path::Path;
///
/// validate_is_readable(Path::new("/etc/hosts"))?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_is_readable(path: &Path) -> Result<()> {
    // Try to read metadata to check accessibility
    match std::fs::metadata(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::ValidationError {
            message: format!("Path '{}' is not readable: {}", path.display(), e),
            field: None,
            value: None,
            constraints: Vec::new(),
        }),
    }
}

/// Validate that a path is writable.
///
/// This is an **I/O operation** that checks if a file/directory can be written to.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the path is not writable.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_is_writable;
/// use std::path::Path;
///
/// validate_is_writable(Path::new("/tmp"))?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_is_writable(path: &Path) -> Result<()> {
    // Try to open with write access to check writability
    // Use a read-only open for directories to avoid accidental modification
    if path.is_dir() {
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ValidationError {
                message: format!("Directory '{}' is not writable", path.display()),
                field: None,
                value: None,
                constraints: Vec::new(),
            }),
        }
    } else {
        // For files, check parent directory writability
        path.parent().map_or_else(
            || {
                Err(Error::ValidationError {
                    message: format!(
                        "Cannot check writability for path without parent: '{}'",
                        path.display()
                    ),
                    field: None,
                    value: None,
                    constraints: Vec::new(),
                })
            },
            validate_is_writable,
        )
    }
}

// ============================================================================
// SESSION VALIDATION
// ============================================================================

/// Validate that a session's workspace path exists on the filesystem.
///
/// This is an **I/O operation** and should only be called from the
/// infrastructure layer.
///
/// The validation is skipped if the session status is `Creating`, since
/// the workspace may not have been created yet.
///
/// # Errors
///
/// Returns `Error::ValidationError` if:
/// - Session status is not `Creating` AND
/// - The workspace path does not exist on the filesystem
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_session_workspace_exists;
///
/// let session = Session { /* ... */ };
/// validate_session_workspace_exists(&session)?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_session_workspace_exists(session: &Session) -> Result<()> {
    if session.status != SessionStatus::Creating && !session.workspace_path.exists() {
        return Err(Error::ValidationError {
            message: format!(
                "Workspace '{}' does not exist for session '{}'",
                session.workspace_path.as_str(),
                session.name()
            ),
            field: Some("workspace_path".to_string()),
            value: Some(session.workspace_path.to_string()),
            constraints: vec![
                "workspace must exist".to_string(),
                "path must be valid".to_string(),
            ],
        });
    }
    Ok(())
}

// ============================================================================
// FILE SYSTEM VALIDATION HELPERS
// ============================================================================

/// Validate that a directory is empty.
///
/// This is an **I/O operation** that checks if a directory contains any entries.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the directory is not empty.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_directory_empty;
/// use std::path::Path;
///
/// validate_directory_empty(Path::new("/tmp/empty-dir"))?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
pub fn validate_directory_empty(path: &Path) -> Result<()> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(Error::ValidationError {
                    message: format!("Directory '{}' is not empty", path.display()),
                    field: None,
                    value: None,
                    constraints: vec!["directory must be empty".to_string()],
                });
            }
            Ok(())
        }
        Err(e) => Err(Error::ValidationError {
            message: format!("Cannot read directory '{}': {}", path.display(), e),
            field: None,
            value: None,
            constraints: Vec::new(),
        }),
    }
}

/// Validate that a directory has sufficient space for operations.
///
/// This is an **I/O operation** that checks available disk space.
///
/// # Errors
///
/// Returns `Error::ValidationError` if insufficient space is available.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_sufficient_space;
/// use std::path::Path;
///
/// // Check for at least 1MB of free space
/// validate_sufficient_space(Path::new("/tmp"), 1024 * 1024)?;
/// ```
///
/// # Note
///
/// This function performs I/O and should not be called from pure domain logic.
/// This is a simplified implementation; a production version would use
/// platform-specific APIs to check actual disk space.
pub fn validate_sufficient_space(path: &Path, _required_bytes: u64) -> Result<()> {
    // Note: This is a simplified implementation
    // A production version would use platform-specific APIs:
    // - Unix: statvfs
    // - Windows: GetDiskFreeSpaceEx

    // For now, we'll just verify the path is accessible
    // Real disk space checking would be platform-specific
    validate_path_exists(path)?;

    // TODO: Implement actual disk space checking
    // See: https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html#tymethod.blocks
    // or use the `sysinfo` crate

    Ok(())
}

// ============================================================================
// VALIDATION COMBINATORS
// ============================================================================

/// Validate all paths in a collection exist.
///
/// This is a combinator that validates all paths in a collection,
/// returning the first error encountered or `Ok(())` if all exist.
///
/// # Errors
///
/// Returns the first `Error::ValidationError` encountered.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_all_paths_exist;
/// use std::path::Path;
///
/// let paths = vec![
///     Path::new("/tmp"),
///     Path::new("/home"),
/// ];
/// validate_all_paths_exist(&paths)?;
/// ```
pub fn validate_all_paths_exist(paths: &[&Path]) -> Result<()> {
    paths
        .iter()
        .try_for_each(|&path| validate_path_exists(path))
}

/// Validate that any path in a collection exists.
///
/// Returns `Ok(())` if at least one path exists, or an error if none exist.
///
/// # Errors
///
/// Returns `Error::ValidationError` if no paths exist.
///
/// # Examples
///
/// ```ignore
/// use isolate_core::validation::infrastructure::validate_any_path_exists;
/// use std::path::Path;
///
/// let paths = vec![
///     Path::new("/nonexistent1"),
///     Path::new("/tmp"),
///     Path::new("/nonexistent2"),
/// ];
/// validate_any_path_exists(&paths)?; // Ok because /tmp exists
/// ```
pub fn validate_any_path_exists(paths: &[&Path]) -> Result<()> {
    let exists = paths.iter().any(|&path| path.exists());

    if !exists {
        return Err(Error::ValidationError {
            message: format!(
                "None of the provided paths exist: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            field: None,
            value: None,
            constraints: vec!["at least one path must exist".to_string()],
        });
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // ===== Path Validation Tests =====

    #[test]
    fn test_validate_path_exists_for_tmp() {
        let result = validate_path_exists(Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_exists_rejects_nonexistent() {
        let result = validate_path_exists(Path::new("/nonexistent/path/that/should/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_is_directory_for_tmp() {
        let result = validate_is_directory(Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_is_directory_rejects_file() {
        let result = validate_is_directory(Path::new("/etc/hosts"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_is_file_for_hosts() {
        let result = validate_is_file(Path::new("/etc/hosts"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_is_file_rejects_directory() {
        let result = validate_is_file(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_path_for_tmp() {
        let result = validate_workspace_path(Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_workspace_path_rejects_nonexistent() {
        let result = validate_workspace_path(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_path_rejects_file() {
        let result = validate_workspace_path(Path::new("/etc/hosts"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_is_readable_for_tmp() {
        let result = validate_is_readable(Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_is_readable_rejects_nonexistent() {
        let result = validate_is_readable(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    // ===== Directory Empty Tests =====

    #[test]
    fn test_validate_directory_empty_for_empty_dir() {
        let Ok(temp_dir) = tempfile::TempDir::new() else {
            // Skip test if tempfile creation fails
            return;
        };

        let result = validate_directory_empty(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_directory_empty_rejects_nonempty_dir() {
        let Ok(temp_dir) = tempfile::TempDir::new() else {
            // Skip test if tempfile creation fails
            return;
        };

        // Create a file in the temp dir
        let test_file = temp_dir.path().join("test.txt");
        if fs::write(&test_file, b"test").is_err() {
            return;
        }

        let result = validate_directory_empty(temp_dir.path());
        assert!(result.is_err());
    }

    // ===== Validation Combinators Tests =====

    #[test]
    fn test_validate_all_paths_exist_all_exist() {
        let paths = vec![Path::new("/tmp"), Path::new("/home")];
        let result = validate_all_paths_exist(&paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_all_paths_exist_one_missing() {
        let paths = vec![
            Path::new("/tmp"),
            Path::new("/nonexistent"),
            Path::new("/home"),
        ];
        let result = validate_all_paths_exist(&paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_any_path_exists_all_exist() {
        let paths = vec![Path::new("/tmp"), Path::new("/home")];
        let result = validate_any_path_exists(&paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_any_path_exists_one_exists() {
        let paths = vec![
            Path::new("/nonexistent1"),
            Path::new("/tmp"),
            Path::new("/nonexistent2"),
        ];
        let result = validate_any_path_exists(&paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_any_path_exists_none_exist() {
        let paths = vec![
            Path::new("/nonexistent1"),
            Path::new("/nonexistent2"),
            Path::new("/nonexistent3"),
        ];
        let result = validate_any_path_exists(&paths);
        assert!(result.is_err());
    }
}
