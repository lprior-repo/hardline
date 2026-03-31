//! Validation helper functions
//!
//! Contains async helper functions for workspace validation checks.

use std::path::{Path, PathBuf};

use crate::workspace_integrity::issue::IntegrityIssue;
use crate::workspace_integrity::types::CorruptionType;
use crate::{Error, Result};

/// Resolve a workspace name to its full path
///
/// # Arguments
/// * `workspaces_root` - Root directory containing workspaces
/// * `workspace_name` - Name of the workspace to resolve
///
/// # Returns
/// The resolved path for the workspace
pub fn resolve_workspace_path(workspaces_root: &Path, workspace_name: &str) -> PathBuf {
    let input_path = Path::new(workspace_name);

    if input_path.is_absolute() {
        return input_path.to_path_buf();
    }

    let looks_like_path = workspace_name.contains(std::path::MAIN_SEPARATOR)
        || workspace_name.contains('/')
        || workspace_name.starts_with('.');

    if looks_like_path {
        return input_path.to_path_buf();
    }

    workspaces_root.join(workspace_name)
}

/// Validate the .jj directory structure
async fn validate_jj_directory(jj_dir: &Path) -> std::result::Result<(), IntegrityIssue> {
    let repo_path = jj_dir.join("repo");
    let repo_exists = tokio::fs::try_exists(&repo_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check repo: {e}"),
        )
        .with_path(&repo_path)
    })?;
    if !repo_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedJjDir,
            "JJ repository metadata missing ('repo' path)",
        )
        .with_path(jj_dir));
    }

    // Check if repo is a file (workspace pointing to shared repo) or directory
    let repo_metadata = tokio::fs::metadata(&repo_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check repo metadata: {e}"),
        )
        .with_path(&repo_path)
    })?;

    // If repo is a file, this is a workspace pointing to a shared repo.
    if repo_metadata.is_file() {
        validate_shared_repo_pointer(jj_dir, &repo_path).await?;
        return Ok(());
    }

    // If repo is a directory, validate the op_store
    // Check for empty critical directories
    let op_store = repo_path.join("op_store");
    let op_store_exists = tokio::fs::try_exists(&op_store).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check op_store directory: {e}"),
        )
        .with_path(&op_store)
    })?;
    if op_store_exists {
        match tokio::fs::read_dir(&op_store).await {
            Ok(mut entries) => {
                let has_entries = match entries.next_entry().await {
                    Ok(Some(_)) => true,
                    Ok(None) | Err(_) => false,
                };
                if !has_entries {
                    return Err(IntegrityIssue::new(
                        CorruptionType::CorruptedJjDir,
                        "JJ operation store is empty",
                    )
                    .with_path(&op_store));
                }
            }
            Err(e) => {
                return Err(IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read JJ op_store: {e}"),
                )
                .with_path(&op_store));
            }
        }
    }

    Ok(())
}

/// Validate a workspace pointing to a shared repo via pointer file
async fn validate_shared_repo_pointer(
    jj_dir: &Path,
    repo_pointer_path: &Path,
) -> std::result::Result<(), IntegrityIssue> {
    let shared_repo_path = match tokio::fs::read_to_string(repo_pointer_path).await {
        Ok(content) => {
            let path_str = content.trim();
            if path_str.is_empty() {
                return Err(IntegrityIssue::new(
                    CorruptionType::CorruptedJjDir,
                    "JJ repo file is empty - workspace has no backing repo",
                )
                .with_path(repo_pointer_path));
            }

            if std::path::Path::new(path_str).is_absolute() {
                std::path::PathBuf::from(path_str)
            } else {
                jj_dir.join(path_str)
            }
        }
        Err(e) => {
            return Err(IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot read JJ repo file: {e}"),
            )
            .with_path(repo_pointer_path));
        }
    };

    let shared_exists = tokio::fs::try_exists(&shared_repo_path)
        .await
        .map_err(|e| {
            IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                format!("Cannot check referenced shared repo: {e}"),
            )
            .with_path(&shared_repo_path)
        })?;

    if !shared_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedJjDir,
            format!(
                "JJ repo file points to non-existent path: {}",
                shared_repo_path.display()
            ),
        )
        .with_path(repo_pointer_path));
    }

    let shared_metadata = tokio::fs::metadata(&shared_repo_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot inspect referenced shared repo metadata: {e}"),
        )
        .with_path(&shared_repo_path)
    })?;

    if !shared_metadata.is_dir() {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedJjDir,
            format!(
                "JJ repo file points to non-directory path: {}",
                shared_repo_path.display()
            ),
        )
        .with_path(repo_pointer_path));
    }

    let shared_op_store = shared_repo_path.join("op_store");
    let shared_op_store_exists = tokio::fs::try_exists(&shared_op_store).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check shared repo op_store: {e}"),
        )
        .with_path(&shared_op_store)
    })?;

    if !shared_op_store_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedJjDir,
            format!(
                "Referenced shared repo missing op_store: {}",
                shared_op_store.display()
            ),
        )
        .with_path(repo_pointer_path));
    }

    Ok(())
}

/// Check for stale lock files in the workspace
pub async fn check_stale_locks(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
    let lock_file = workspace_path.join(".jj").join("working_copy").join("lock");

    let lock_exists = tokio::fs::try_exists(&lock_file)
        .await
        .map_err(|e| Error::io_error(e.to_string()))?;
    if lock_exists {
        // Check age of lock file
        let metadata = tokio::fs::metadata(&lock_file)
            .await
            .map_err(|e| Error::io_error(e.to_string()))?;
        let modified = metadata
            .modified()
            .map_err(|e| Error::io_error(e.to_string()))?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .map_err(|e| Error::invalid_state(format!("Failed to calculate lock age: {e}")))?;
        let age_secs = age.as_secs();

        // Lock older than 1 hour is suspicious
        if age_secs > 3600 {
            return Ok(Some(
                IntegrityIssue::new(
                    CorruptionType::StaleLocks,
                    format!("Stale lock file detected (age: {age_secs}s)"),
                )
                .with_path(&lock_file),
            ));
        }
    }

    Ok(None)
}

/// Check config file for TOML parsing errors
///
/// Validates that the workspace's .isolate/config.toml file (if present)
/// is valid TOML and can be parsed.
pub async fn check_config_file(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
    let config_file = workspace_path.join(".isolate").join("config.toml");

    // Check if config file exists
    let config_exists = tokio::fs::try_exists(&config_file)
        .await
        .map_err(|e| Error::io_error(e.to_string()))?;
    if !config_exists {
        // No config file is fine - not all workspaces have custom config
        return Ok(None);
    }

    // Read and validate TOML
    let file_content = match tokio::fs::read_to_string(&config_file).await {
        Ok(file_content) => file_content,
        Err(e) => {
            return Ok(Some(
                IntegrityIssue::new(
                    CorruptionType::CorruptedJjDir,
                    format!("Cannot read config file: {e}"),
                )
                .with_path(&config_file)
                .with_context(format!("Permission denied or file corrupted: {e}")),
            ));
        }
    };

    // Try to parse as TOML
    match toml::from_str::<toml::Value>(&file_content) {
        Ok(_) => Ok(None), // Valid TOML
        Err(e) => {
            let hint = format!(
                "TOML parse error: {e}. \
                 Suggestion: Check for syntax errors like unclosed brackets, \
                 missing quotes, or invalid values."
            );
            Ok(Some(
                IntegrityIssue::new(
                    CorruptionType::CorruptedJjDir,
                    format!("Config file contains invalid TOML: {e}"),
                )
                .with_path(&config_file)
                .with_context(hint),
            ))
        }
    }
}

/// Validate JJ directory and return an issue if invalid
pub async fn validate_jj_dir_for_issue(jj_dir: &Path) -> Option<IntegrityIssue> {
    validate_jj_directory(jj_dir).await.err()
}
