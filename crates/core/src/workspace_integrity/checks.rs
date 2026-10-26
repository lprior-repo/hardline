//! Validation helper functions
//!
//! Contains async helper functions for workspace validation checks.

use std::path::{Path, PathBuf};

use crate::{
    workspace_integrity::{issue::IntegrityIssue, types::CorruptionType},
    Error, Result,
};

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

/// Validate the .git directory structure
///
/// A linked git worktree's `.git` is a file containing `gitdir: <path>`,
/// not a directory. In that case the pointed-at gitdir is validated
/// instead: HEAD lives in the gitdir itself, while objects/refs live in
/// the common dir when the gitdir contains a `commondir` file (its
/// contents, relative to the gitdir, point at the common dir), and in
/// the gitdir itself otherwise. A real `.git` directory is validated
/// exactly as before.
async fn validate_git_directory(git_dir: &Path) -> std::result::Result<(), IntegrityIssue> {
    // Linked worktree: `.git` is a pointer file, not a directory.
    if let Ok(metadata) = tokio::fs::metadata(git_dir).await {
        if metadata.is_file() {
            return validate_linked_worktree_gitdir(git_dir).await;
        }
    }

    validate_git_dir_contents(git_dir, git_dir).await
}

/// Resolve a linked-worktree `.git` pointer file and validate its gitdir.
async fn validate_linked_worktree_gitdir(
    git_file: &Path,
) -> std::result::Result<(), IntegrityIssue> {
    let contents = tokio::fs::read_to_string(git_file).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot read .git pointer file: {e}"),
        )
        .with_path(git_file)
    })?;
    let pointer = match contents.trim().strip_prefix("gitdir:") {
        Some(rest) => rest.trim(),
        None => {
            return Err(IntegrityIssue::new(
                CorruptionType::CorruptedGitDir,
                "Git worktree pointer file is malformed (expected 'gitdir: <path>')",
            )
            .with_path(git_file));
        }
    };
    if pointer.is_empty() {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedGitDir,
            "Git worktree pointer file is malformed (empty 'gitdir:' path)",
        )
        .with_path(git_file));
    }
    let pointer_path = Path::new(pointer);
    let resolved_gitdir = if pointer_path.is_absolute() {
        pointer_path.to_path_buf()
    } else {
        match git_file.parent() {
            Some(worktree_root) => worktree_root.join(pointer_path),
            None => {
                return Err(IntegrityIssue::new(
                    CorruptionType::CorruptedGitDir,
                    "Git worktree pointer file has no parent directory to resolve its relative path against",
                )
                .with_path(git_file));
            }
        }
    };

    // objects/refs live in the common dir when a commondir file is present.
    let commondir_path = resolved_gitdir.join("commondir");
    let commondir_exists = matches!(
        tokio::fs::try_exists(&commondir_path).await,
        Ok(true)
    );
    let objects_refs_base = if commondir_exists {
        let common_contents =
            tokio::fs::read_to_string(&commondir_path)
                .await
                .map_err(|e| {
                    IntegrityIssue::new(
                        CorruptionType::PermissionDenied,
                        format!("Cannot read commondir file: {e}"),
                    )
                    .with_path(&commondir_path)
                })?;
        let common_pointer = common_contents.trim();
        if common_pointer.is_empty() {
            resolved_gitdir.clone()
        } else {
            let common_path = Path::new(common_pointer);
            if common_path.is_absolute() {
                common_path.to_path_buf()
            } else {
                resolved_gitdir.join(common_path)
            }
        }
    } else {
        resolved_gitdir.clone()
    };

    validate_git_dir_contents(&resolved_gitdir, &objects_refs_base).await
}

/// Check HEAD in `git_dir` and objects/refs in `objects_refs_base`.
async fn validate_git_dir_contents(
    git_dir: &Path,
    objects_refs_base: &Path,
) -> std::result::Result<(), IntegrityIssue> {
    // Check for HEAD file (required for a valid Git repository)
    let head_path = git_dir.join("HEAD");
    let head_exists = tokio::fs::try_exists(&head_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check HEAD file: {e}"),
        )
        .with_path(&head_path)
    })?;
    if !head_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedGitDir,
            "Git repository metadata missing ('HEAD' file)",
        )
        .with_path(git_dir));
    }

    // Check for objects directory (required for a valid Git repository)
    let objects_path = objects_refs_base.join("objects");
    let objects_exists = tokio::fs::try_exists(&objects_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check objects directory: {e}"),
        )
        .with_path(&objects_path)
    })?;
    if !objects_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedGitDir,
            "Git repository missing objects directory",
        )
        .with_path(git_dir));
    }

    // Check for refs directory (required for a valid Git repository)
    let refs_path = objects_refs_base.join("refs");
    let refs_exists = tokio::fs::try_exists(&refs_path).await.map_err(|e| {
        IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            format!("Cannot check refs directory: {e}"),
        )
        .with_path(&refs_path)
    })?;
    if !refs_exists {
        return Err(IntegrityIssue::new(
            CorruptionType::CorruptedGitDir,
            "Git repository missing refs directory",
        )
        .with_path(git_dir));
    }

    Ok(())
}

/// Check for stale lock files in the workspace
pub async fn check_stale_locks(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
    // Check for Git index lock file
    let lock_file = workspace_path.join(".git").join("index.lock");

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
/// Validates that the workspace's .hardline/config.toml file (if present)
/// is valid TOML and can be parsed.
pub async fn check_config_file(workspace_path: &Path) -> Result<Option<IntegrityIssue>> {
    let config_file = workspace_path.join(".hardline").join("config.toml");

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
                    CorruptionType::CorruptedGitDir,
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
                    CorruptionType::CorruptedGitDir,
                    format!("Config file contains invalid TOML: {e}"),
                )
                .with_path(&config_file)
                .with_context(hint),
            ))
        }
    }
}

/// Validate Git directory and return an issue if invalid
pub async fn validate_git_dir_for_issue(git_dir: &Path) -> Option<IntegrityIssue> {
    validate_git_directory(git_dir).await.err()
}
