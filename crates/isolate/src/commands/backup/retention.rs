//! Backup retention policy management

#![forbid(unsafe_code)]

use std::path::Path;

use anyhow::{Context, Result};
use tokio::fs;

use super::BackupConfig;
use crate::commands::backup::list;

/// Apply retention policy to backups for a specific database
///
/// Keeps the most recent N backups as configured in `BackupConfig`.
/// Removes older backups beyond the retention count.
///
/// # Errors
///
/// Returns error if:
/// - Backup directory cannot be read
/// - Old backups cannot be removed
#[allow(dead_code)]
// Core retention policy enforcement function
pub async fn apply_retention_policy(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<Vec<String>> {
    let backups: Vec<list::BackupInfo> =
        list::list_database_backups(root, database_name, config).await?;

    if backups.len() <= config.retention_count {
        return Ok(Vec::new());
    }

    // Remove backups beyond retention count (oldest ones)
    let backups_to_remove = &backups[config.retention_count..];

    let mut removed_paths = Vec::new();

    for backup in backups_to_remove {
        // Remove backup file
        fs::remove_file(&backup.path)
            .await
            .with_context(|| format!("Failed to remove backup: {}", backup.path.display()))?;

        // Remove metadata file if exists
        let metadata_path = backup.path.with_extension("json");
        if metadata_path.exists() {
            fs::remove_file(&metadata_path).await.with_context(|| {
                format!(
                    "Failed to remove backup metadata: {}",
                    metadata_path.display()
                )
            })?;
        }

        removed_paths.push(backup.path.display().to_string());
    }

    Ok(removed_paths)
}

/// Apply retention policy to all databases
///
/// # Errors
///
/// Returns error if any retention operation fails
#[allow(dead_code)]
// High-level retention orchestration function
pub async fn apply_retention_policy_all(root: &Path, config: &BackupConfig) -> Result<Vec<String>> {
    // Only state.db and beads.db now - queue.db merged into state.db (bd-30s)
    let databases = vec!["state.db", "beads.db"];

    let mut all_removed = Vec::new();

    for db_name in databases {
        match apply_retention_policy(root, db_name, config).await {
            Ok(removed) => all_removed.extend(removed),
            Err(e) => {
                tracing::warn!("Failed to apply retention policy to {}: {}", db_name, e);
                // Continue with other databases
            }
        }
    }

    Ok(all_removed)
}

/// Calculate total disk space used by backups
///
/// # Errors
///
/// Returns error if backup metadata cannot be accessed
#[allow(dead_code)]
// Utility function for backup size analysis
pub async fn calculate_backup_size(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<u64> {
    let backups: Vec<list::BackupInfo> =
        list::list_database_backups(root, database_name, config).await?;

    let total_size = backups.iter().map(|b| b.size_bytes).sum();

    Ok(total_size)
}

/// Calculate total disk space that would be freed by applying retention policy
///
/// # Errors
///
/// Returns error if backup metadata cannot be accessed
#[allow(dead_code)]
// Utility function for retention planning
pub async fn calculate_freed_space(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<u64> {
    let backups: Vec<list::BackupInfo> =
        list::list_database_backups(root, database_name, config).await?;

    if backups.len() <= config.retention_count {
        return Ok(0);
    }

    let backups_to_remove = &backups[config.retention_count..];
    let freed_space = backups_to_remove.iter().map(|b| b.size_bytes).sum();

    Ok(freed_space)
}

/// Get retention policy status for all databases
///
/// Returns a summary of current backup counts and space usage
///
/// # Errors
///
/// Returns error if backup information cannot be retrieved
#[allow(dead_code)]
// Status reporting function for retention policy
pub async fn get_retention_status(
    root: &Path,
    config: &BackupConfig,
) -> Result<Vec<RetentionStatus>> {
    // Only state.db and beads.db now - queue.db merged into state.db (bd-30s)
    let databases = vec!["state.db", "beads.db"];

    let mut statuses = Vec::new();

    for db_name in databases {
        let backups = super::list::list_database_backups(root, db_name, config).await?;
        let total_size = calculate_backup_size(root, db_name, config).await?;
        let freed_space = calculate_freed_space(root, db_name, config).await?;

        statuses.push(RetentionStatus {
            database_name: db_name.to_string(),
            backup_count: backups.len(),
            retention_limit: config.retention_count,
            total_size_bytes: total_size,
            would_free_bytes: freed_space,
            within_limit: backups.len() <= config.retention_count,
        });
    }

    Ok(statuses)
}

/// Retention policy status for a database
#[derive(Debug, Clone)]
pub struct RetentionStatus {
    /// Database name
    pub database_name: String,
    /// Current number of backups
    pub backup_count: usize,
    /// Maximum backups to retain
    pub retention_limit: usize,
    /// Total disk space used by backups
    pub total_size_bytes: u64,
    /// Disk space that would be freed by applying retention
    pub would_free_bytes: u64,
    /// Whether backup count is within retention limit
    pub within_limit: bool,
}

impl RetentionStatus {
    /// Format size as human-readable string
    #[allow(dead_code)]
    // Utility method for human-readable size formatting
    #[allow(clippy::cast_precision_loss)]
    // Safe because we're displaying sizes with 2 decimal places; precision loss beyond f64 mantissa
    // is negligible for human-readable output
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{bytes} B")
        }
    }
}

#[cfg(test)]
mod tests {
    // Test code uses expect() for test clarity.
    // Production code must use Result<T, Error> patterns.
    #![allow(clippy::expect_used)]

    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_apply_retention_policy_removes_old_backups() -> Result<()> {
        let temp_dir =
            TempDir::new().map_err(|e| anyhow::anyhow!("Failed to create temp directory: {e}"))?;
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            retention_count: 2,
        };

        let backup_dir = root.join("backups").join("state.db");
        fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create backup directory for test: {e}"))?;

        // Create 3 backups
        for i in 1..=3 {
            let backup_path = backup_dir.join(format!("backup-2025010{i}-120000.db"));
            fs::write(&backup_path, b"data")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write test backup file: {e}"))?;
        }

        // Apply retention (keep 2, remove 1)
        let removed = apply_retention_policy(root, "state.db", &config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to apply retention policy: {e}"))?;

        assert_eq!(removed.len(), 1);

        // Verify 2 backups remain
        let backups: Vec<list::BackupInfo> = list::list_database_backups(root, "state.db", &config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list backups: {e}"))?;

        assert_eq!(backups.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_calculate_backup_size() -> Result<()> {
        let temp_dir =
            TempDir::new().map_err(|e| anyhow::anyhow!("Failed to create temp directory: {e}"))?;
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        let backup_dir = root.join("backups").join("state.db");
        fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create backup directory for test: {e}"))?;

        // Create backups with known sizes
        let backup1 = backup_dir.join("backup-20250101-100000.db");
        let backup2 = backup_dir.join("backup-20250101-120000.db");

        fs::write(&backup1, vec![0u8; 1000])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write test backup file: {e}"))?;
        fs::write(&backup2, vec![0u8; 2000])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write test backup file: {e}"))?;

        let total_size = calculate_backup_size(root, "state.db", &config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to calculate backup size: {e}"))?;

        assert_eq!(total_size, 3000);
        Ok(())
    }

    #[tokio::test]
    async fn test_retention_status_within_limit() -> Result<()> {
        let temp_dir =
            TempDir::new().map_err(|e| anyhow::anyhow!("Failed to create temp directory: {e}"))?;
        let root = temp_dir.path();

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            retention_count: 5,
        };

        let backup_dir = root.join("backups").join("state.db");
        fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create backup directory for test: {e}"))?;

        // Create 3 backups (under limit of 5)
        for i in 1..=3 {
            let backup_path = backup_dir.join(format!("backup-2025010{i}-120000.db"));
            fs::write(&backup_path, vec![0u8; 1000])
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write test backup file: {e}"))?;
        }

        let statuses = get_retention_status(root, &config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get retention status for test: {e}"))?;

        let test_status = statuses
            .iter()
            .find(|s| s.database_name == "state.db")
            .ok_or_else(|| anyhow::anyhow!("state.db status not found"))?;

        assert_eq!(test_status.backup_count, 3);
        assert!(test_status.within_limit);
        assert_eq!(test_status.would_free_bytes, 0);
        Ok(())
    }
}