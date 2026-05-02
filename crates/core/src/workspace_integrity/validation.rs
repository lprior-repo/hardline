//! Workspace integrity validation
//!
//! Provides the IntegrityValidator for detecting workspace corruption.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use futures::future::try_join_all;

use crate::{
    workspace_integrity::{
        checks::{
            check_config_file, check_stale_locks, resolve_workspace_path,
            validate_git_dir_for_issue,
        },
        issue::IntegrityIssue,
        types::CorruptionType,
        validation_result::ValidationResult,
    },
    Error, Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY VALIDATOR
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace integrity validator
///
/// Provides methods to validate workspace integrity and detect corruption.
#[derive(Debug, Clone)]
pub struct IntegrityValidator {
    /// Root directory containing workspaces
    pub workspaces_root: PathBuf,
    /// Check timeout in milliseconds
    pub timeout_ms: u64,
}

impl IntegrityValidator {
    /// Default timeout for validation checks (5 seconds)
    pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

    /// Create a new integrity validator
    #[must_use]
    pub fn new(workspaces_root: impl Into<PathBuf>) -> Self {
        Self {
            workspaces_root: workspaces_root.into(),
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// Set custom timeout
    #[must_use]
    pub const fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Validate a single workspace
    pub async fn validate(&self, workspace_name: &str) -> Result<ValidationResult> {
        let start = SystemTime::now();
        let workspace_path = resolve_workspace_path(&self.workspaces_root, workspace_name);

        // Check 1: Directory exists
        let path_exists = tokio::fs::try_exists(&workspace_path)
            .await
            .map_err(|e| Error::io_error(e.to_string()))?;
        if !path_exists {
            let mut issues = Vec::new();
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingDirectory,
                    format!(
                        "Workspace directory does not exist: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&workspace_path),
            );
            let duration = measure_duration(&start)?;
            return Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            );
        }

        let mut issues = Vec::new();
        Self::check_readability(&workspace_path, &mut issues).await;
        Self::check_git_dir(&workspace_path, &mut issues).await;
        Self::check_config_file(&workspace_path, &mut issues).await;
        Self::check_locks(&workspace_path, &mut issues).await;

        let duration = measure_duration(&start)?;

        if issues.is_empty() {
            Ok(ValidationResult::valid(workspace_name, &workspace_path).with_duration(duration))
        } else {
            Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            )
        }
    }

    /// Validate multiple workspaces in parallel
    ///
    /// RESULTS ARE ORDERED: Returns results in the same order as input workspaces.
    /// Uses concurrent validation for performance but preserves ordering for predictability.
    pub async fn validate_all(&self, workspaces: &[String]) -> Result<Vec<ValidationResult>> {
        // Clone self for each validation (IntegrityValidator is cheap to clone - just PathBuf +
        // u64)
        let validator = Arc::new(self);

        // Create validation futures that preserve input order
        let futures = workspaces
            .iter()
            .map(|name| {
                let validator = validator.clone();
                let name = name.clone();
                async move { (*validator).validate(&name).await }
            })
            .collect::<Vec<_>>();

        // Run concurrently but preserve order (try_join_all maintains input order)
        try_join_all(futures).await
    }

    /// Check 2: Directory is readable
    async fn check_readability(workspace_path: &PathBuf, issues: &mut Vec<IntegrityIssue>) {
        if let Err(e) = tokio::fs::read_dir(workspace_path).await {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read workspace directory: {e}"),
                )
                .with_path(workspace_path)
                .with_context(e.to_string()),
            );
        }
    }

    /// Check 3-4: .git directory exists and is valid
    async fn check_git_dir(workspace_path: &Path, issues: &mut Vec<IntegrityIssue>) {
        let git_dir = workspace_path.join(".git");
        let git_dir_exists = tokio::fs::try_exists(&git_dir)
            .await
            .map_err(|e| Error::io_error(e.to_string()));
        match git_dir_exists {
            Ok(true) => {
                if let Some(issue) = validate_git_dir_for_issue(&git_dir).await {
                    issues.push(issue);
                }
            }
            Ok(false) => {
                issues.push(
                    IntegrityIssue::new(
                        CorruptionType::MissingGitDir,
                        format!(
                            ".git directory missing from workspace: {}",
                            workspace_path.display()
                        ),
                    )
                    .with_path(&git_dir),
                );
            }
            Err(_) => {}
        }
    }

    /// Check 5: Config file integrity (TOML validation)
    async fn check_config_file(workspace_path: &Path, issues: &mut Vec<IntegrityIssue>) {
        if let Ok(Some(issue)) = check_config_file(workspace_path).await {
            issues.push(issue);
        }
    }

    /// Check 6: Lock files
    async fn check_locks(workspace_path: &Path, issues: &mut Vec<IntegrityIssue>) {
        if let Ok(Some(issue)) = check_stale_locks(workspace_path).await {
            issues.push(issue);
        }
    }
}

/// Measure elapsed time since `start`, returning milliseconds.
fn measure_duration(start: &SystemTime) -> Result<u64> {
    start
        .elapsed()
        .map_err(|e| Error::invalid_state(format!("Failed to measure duration: {e}")))
        .and_then(|d| {
            u64::try_from(d.as_millis()).map_err(|_| {
                Error::invalid_state("Duration overflow - operation took too long".to_string())
            })
        })
}
