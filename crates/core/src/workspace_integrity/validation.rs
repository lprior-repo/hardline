//! Workspace integrity validation
//!
//! Provides the IntegrityValidator for detecting workspace corruption.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use futures::future::try_join_all;

use crate::workspace_integrity::checks::{
    check_config_file, check_stale_locks, resolve_workspace_path, validate_jj_dir_for_issue,
};
use crate::workspace_integrity::types::CorruptionType;
use crate::workspace_integrity::issue::IntegrityIssue;
use crate::workspace_integrity::validation_result::ValidationResult;
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY VALIDATOR
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace integrity validator
///
/// Provides methods to validate workspace integrity and detect corruption.
#[derive(Debug, Clone)]
pub struct IntegrityValidator {
    /// Root directory containing workspaces
    workspaces_root: PathBuf,
    /// Check timeout in milliseconds
    timeout_ms: u64,
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

        let mut issues = Vec::new();

        // Check 1: Directory exists
        let path_exists = tokio::fs::try_exists(&workspace_path).await?;
        if !path_exists {
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

            // Can't continue validation if directory is missing
            let duration = start
                .elapsed()
                .map_err(|e| Error::Internal(format!("Failed to measure duration: {e}")))
                .and_then(|d| {
                    u64::try_from(d.as_millis()).map_err(|_| {
                        Error::Internal("Duration overflow - operation took too long".to_string())
                    })
                })?;
            return Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            );
        }

        // Check 2: Directory is readable
        if let Err(e) = tokio::fs::read_dir(&workspace_path).await {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read workspace directory: {e}"),
                )
                .with_path(&workspace_path)
                .with_context(e.to_string()),
            );
        }

        // Check 3: .jj directory exists
        let jj_dir = workspace_path.join(".jj");
        let jj_dir_exists = tokio::fs::try_exists(&jj_dir).await?;
        if jj_dir_exists {
            // Check 4: .jj directory is valid
            if let Some(issue) = validate_jj_dir_for_issue(&jj_dir).await {
                issues.push(issue);
            }
        } else {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingJjDir,
                    format!(
                        ".jj directory missing from workspace: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&jj_dir),
            );
        }

        // Check 5: Config file integrity (TOML validation)
        if let Ok(Some(issue)) = check_config_file(&workspace_path).await {
            issues.push(issue);
        }

        // Check 6: Lock files
        if let Ok(Some(issue)) = check_stale_locks(&workspace_path).await {
            issues.push(issue);
        }

        let duration = start
            .elapsed()
            .map_err(|e| Error::Internal(format!("Failed to measure duration: {e}")))
            .and_then(|d| {
                u64::try_from(d.as_millis()).map_err(|_| {
                    Error::Internal("Duration overflow - operation took too long".to_string())
                })
            })?;

        if issues.is_empty() {
            Ok(
                ValidationResult::valid(workspace_name, &workspace_path)
                    .with_duration(duration),
            )
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
}
