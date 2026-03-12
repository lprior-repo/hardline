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

pub fn validate_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::ValidationFieldError {
            message: format!("Path '{}' does not exist", path.display()),
            field: "path".to_string(),
            value: Some(path.display().to_string()),
        });
    }
    Ok(())
}

pub fn validate_is_directory(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::ValidationFieldError {
            message: format!("Path '{}' is not a directory", path.display()),
            field: "path".to_string(),
            value: Some(path.display().to_string()),
        });
    }
    Ok(())
}

pub fn validate_is_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(Error::ValidationFieldError {
            message: format!("Path '{}' is not a file", path.display()),
            field: "path".to_string(),
            value: Some(path.display().to_string()),
        });
    }
    Ok(())
}

pub fn validate_workspace_path(path: &Path) -> Result<()> {
    validate_path_exists(path)?;
    validate_is_directory(path)
}

pub fn validate_is_readable(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::ValidationFieldError {
            message: format!("Path '{}' is not readable: {}", path.display(), e),
            field: "path".to_string(),
            value: Some(path.display().to_string()),
        }),
    }
}

pub fn validate_is_writable(path: &Path) -> Result<()> {
    if path.is_dir() {
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ValidationFieldError {
                message: format!("Directory '{}' is not writable", path.display()),
                field: "path".to_string(),
                value: Some(path.display().to_string()),
            }),
        }
    } else {
        match path.parent() {
            Some(parent) => validate_is_writable(parent),
            None => Err(Error::ValidationFieldError {
                message: format!(
                    "Cannot check writability for path without parent: '{}'",
                    path.display()
                ),
                field: "path".to_string(),
                value: Some(path.display().to_string()),
            }),
        }
    }
}

pub fn validate_directory_empty(path: &Path) -> Result<()> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(Error::ValidationFieldError {
                    message: format!("Directory '{}' is not empty", path.display()),
                    field: "path".to_string(),
                    value: Some(path.display().to_string()),
                });
            }
            Ok(())
        }
        Err(e) => Err(Error::ValidationFieldError {
            message: format!("Cannot read directory '{}': {}", path.display(), e),
            field: "path".to_string(),
            value: Some(path.display().to_string()),
        }),
    }
}

pub fn validate_sufficient_space(path: &Path, _required_bytes: u64) -> Result<()> {
    validate_path_exists(path)?;
    Ok(())
}

pub fn validate_all_paths_exist(paths: &[&Path]) -> Result<()> {
    for &path in paths {
        validate_path_exists(path)?;
    }
    Ok(())
}

pub fn validate_any_path_exists(paths: &[&Path]) -> Result<()> {
    let exists = paths.iter().any(|&path| path.exists());

    if !exists {
        return Err(Error::ValidationFieldError {
            message: format!(
                "None of the provided paths exist: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            field: "paths".to_string(),
            value: None,
        });
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
