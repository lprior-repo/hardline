//! Infrastructure validation - I/O operations for filesystem checks
//!
//! This module provides **I/O validation functions** that form the "Imperative Shell"
//! of the validation architecture. These functions:
//! - Perform I/O operations (filesystem checks)
//! - Should be called from the infrastructure/services layer
//! - Return `Result<(), Error>` with context

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::Path;

use crate::error::{Error, Result};

/// Validates that a path exists on the filesystem.
///
/// # Errors
///
/// Returns an error if the path does not exist.
pub fn validate_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::validation_field_error(
            "path",
            format!("Path '{}' does not exist", path.display()),
            Some(path.display().to_string()),
        ));
    }
    Ok(())
}

/// Validates that a path is a directory.
///
/// # Errors
///
/// Returns an error if the path is not a directory.
pub fn validate_is_directory(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::validation_field_error(
            "path",
            format!("Path '{}' is not a directory", path.display()),
            Some(path.display().to_string()),
        ));
    }
    Ok(())
}

/// Validates that a path is a file.
///
/// # Errors
///
/// Returns an error if the path is not a file.
pub fn validate_is_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(Error::validation_field_error(
            "path",
            format!("Path '{}' is not a file", path.display()),
            Some(path.display().to_string()),
        ));
    }
    Ok(())
}

/// Validates that a path is an existing directory (suitable for a workspace).
///
/// # Errors
///
/// Returns an error if the path does not exist or is not a directory.
pub fn validate_workspace_path(path: &Path) -> Result<()> {
    validate_path_exists(path)?;
    validate_is_directory(path)
}

/// Validates that a path is readable.
///
/// # Errors
///
/// Returns an error if the path metadata cannot be read.
pub fn validate_is_readable(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::validation_field_error(
            "path",
            format!("Path '{}' is not readable: {}", path.display(), e),
            Some(path.display().to_string()),
        )),
    }
}

/// Validates that a path (or its parent directory) is writable.
///
/// # Errors
///
/// Returns an error if neither the path nor its parent directory is writable,
/// or if the path has no parent.
pub fn validate_is_writable(path: &Path) -> Result<()> {
    if path.is_dir() {
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::validation_field_error(
                "path",
                format!("Directory '{}' is not writable", path.display()),
                Some(path.display().to_string()),
            )),
        }
    } else {
        path.parent().map_or_else(
            || {
                Err(Error::validation_field_error(
                    "path",
                    format!(
                        "Cannot check writability for path without parent: '{}'",
                        path.display()
                    ),
                    Some(path.display().to_string()),
                ))
            },
            validate_is_writable,
        )
    }
}

/// Validates that a directory is empty.
///
/// # Errors
///
/// Returns an error if the directory contains entries or cannot be read.
pub fn validate_directory_empty(path: &Path) -> Result<()> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(Error::validation_field_error(
                    "path",
                    format!("Directory '{}' is not empty", path.display()),
                    Some(path.display().to_string()),
                ));
            }
            Ok(())
        }
        Err(e) => Err(Error::validation_field_error(
            "path",
            format!("Cannot read directory '{}': {}", path.display(), e),
            Some(path.display().to_string()),
        )),
    }
}

/// Validates that sufficient disk space is available (currently a placeholder).
///
/// # Errors
///
/// Returns an error if the path does not exist.
pub fn validate_sufficient_space(path: &Path, _required_bytes: u64) -> Result<()> {
    validate_path_exists(path)?;
    Ok(())
}

/// Validates that all given paths exist.
///
/// # Errors
///
/// Returns an error if any path does not exist.
pub fn validate_all_paths_exist(paths: &[&Path]) -> Result<()> {
    for &path in paths {
        validate_path_exists(path)?;
    }
    Ok(())
}

/// Validates that at least one of the given paths exists.
///
/// # Errors
///
/// Returns an error if none of the paths exist.
pub fn validate_any_path_exists(paths: &[&Path]) -> Result<()> {
    let exists = paths.iter().any(|&path| path.exists());

    if !exists {
        return Err(Error::validation_field_error(
            "paths",
            format!(
                "None of the provided paths exist: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            None,
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_validate_is_readable_for_tmp() {
        let result = validate_is_readable(Path::new("/tmp"));
        assert!(result.is_ok());
    }

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
    fn test_validate_any_path_exists_none_exist() {
        let paths = vec![Path::new("/nonexistent1"), Path::new("/nonexistent2")];
        let result = validate_any_path_exists(&paths);
        assert!(result.is_err());
    }
}
