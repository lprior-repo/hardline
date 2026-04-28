//! Backup restoration functionality

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;

use super::backup_internal::{
    compute_checksum, get_database_backup_dir, parse_backup_filename, BackupConfig, BackupMetadata,
};

/// Restore a database from a backup
///
/// # Errors
///
/// Returns error if:
/// - Backup file does not exist
/// - Backup metadata file does not exist
/// - Metadata cannot be parsed
/// - Checksum verification fails
/// - Target database file cannot be written
#[allow(dead_code)]
// Core restore functionality with checksum verification
pub async fn restore_backup(
    backup_path: &Path,
    target_path: &Path,
    verify_checksum: bool,
) -> Result<()> {
    // Validate backup file exists
    anyhow::ensure!(
        backup_path.exists(),
        "Backup file does not exist: {}",
        backup_path.display()
    );

    // Load and verify metadata
    let metadata_path = backup_path.with_extension("json");
    anyhow::ensure!(
        metadata_path.exists(),
        "Backup metadata file does not exist: {}",
        metadata_path.display()
    );

    let metadata_json = fs::read_to_string(&metadata_path)
        .await
        .context("Failed to read backup metadata")?;

    let metadata: BackupMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse backup metadata")?;

    // Verify checksum if requested
    if verify_checksum {
        let current_checksum = compute_checksum(backup_path)
            .await
            .context("Failed to compute backup checksum for verification")?;

        anyhow::ensure!(
            current_checksum == metadata.checksum,
            "Checksum verification failed: expected {}, got {}",
            metadata.checksum,
            current_checksum
        );
    }

    // Create target directory if needed
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .await
            .context("Failed to create target directory")?;
    }

    // Copy backup to target location
    fs::copy(backup_path, target_path)
        .await
        .context("Failed to copy backup to target")?;

    Ok(())
}

/// Find the most recent backup for a database
///
/// # Errors
///
/// Returns error if:
/// - Backup directory does not exist
/// - No backups are found
#[allow(dead_code)]
// Helper function to locate most recent backup
pub async fn find_latest_backup(
    _root: &Path,
    database_name: &str,
    config: &BackupConfig,
) -> Result<PathBuf> {
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);

    anyhow::ensure!(
        backup_dir.exists(),
        "No backups found for database: {database_name}"
    );

    // Read backup directory and find most recent backup
    let mut entries = fs::read_dir(&backup_dir)
        .await
        .context("Failed to read backup directory")?;

    let mut latest_backup: Option<(PathBuf, chrono::DateTime<chrono::Utc>)> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only process .db files (not metadata .json files)
        if path.extension().and_then(|s| s.to_str()) != Some("db") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid backup filename"))?;

        // Parse timestamp from filename
        if let Ok(timestamp) = parse_backup_filename(filename) {
            match &latest_backup {
                None => {
                    latest_backup = Some((path, timestamp));
                }
                Some((_, latest_ts)) => {
                    if timestamp > *latest_ts {
                        latest_backup = Some((path, timestamp));
                    }
                }
            }
        }
    }

    latest_backup
        .map(|(path, _)| path)
        .ok_or_else(|| anyhow::anyhow!("No valid backups found for database: {database_name}"))
}

/// Restore database from latest backup
///
/// # Errors
///
/// Returns error if:
/// - No backups exist
/// - Restore operation fails
#[allow(dead_code)]
// Convenience function to restore from most recent backup
pub async fn restore_from_latest(
    root: &Path,
    database_name: &str,
    config: &BackupConfig,
    verify_checksum: bool,
) -> Result<PathBuf> {
    let backup_path = find_latest_backup(root, database_name, config).await?;

    // Determine target path based on database name
    let target_path = match database_name {
        "beads.db" => root.join(".beads").join(database_name),
        _ => root.join(".isolate").join(database_name),
    };

    restore_backup(&backup_path, &target_path, verify_checksum).await?;

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::unwrap_used)]

    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_restore_backup_creates_target() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let root = temp_dir.path();

        // Create backup file
        let backup_dir = root.join("backups").join("test.db");
        match fs::create_dir_all(&backup_dir).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to create backup dir: {e}"),
        }

        let backup_path = backup_dir.join("backup-20250101-120000.db");
        match fs::write(&backup_path, b"backup data").await {
            Ok(_) => {}
            Err(e) => panic!("Failed to write backup file: {e}"),
        }

        // Create metadata
        let metadata = BackupMetadata::new("test.db".to_string(), 11, "checksum123".to_string());
        let metadata_path = backup_path.with_extension("json");
        let metadata_json = match serde_json::to_string_pretty(&metadata) {
            Ok(json) => json,
            Err(e) => panic!("Failed to serialize metadata: {e}"),
        };
        match fs::write(&metadata_path, metadata_json).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to write metadata: {e}"),
        }

        // Restore
        let target_path = root.join("restored.db");
        match restore_backup(&backup_path, &target_path, false).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to restore backup: {e}"),
        }

        assert!(target_path.exists());
        assert_eq!(
            match fs::read_to_string(&target_path).await {
                Ok(s) => s,
                Err(e) => panic!("Failed to read target file: {e}"),
            },
            "backup data"
        );
    }

    #[tokio::test]
    async fn test_restore_backup_checksum_verification() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let root = temp_dir.path();

        // Create backup file
        let backup_dir = root.join("backups").join("test.db");
        match fs::create_dir_all(&backup_dir).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to create backup dir: {e}"),
        }

        let backup_path = backup_dir.join("backup-20250101-120000.db");
        match fs::write(&backup_path, b"backup data").await {
            Ok(_) => {}
            Err(e) => panic!("Failed to write backup file: {e}"),
        }

        // Create metadata with WRONG checksum
        let metadata = BackupMetadata::new("test.db".to_string(), 11, "wrongchecksum".to_string());
        let metadata_path = backup_path.with_extension("json");
        let metadata_json = match serde_json::to_string_pretty(&metadata) {
            Ok(json) => json,
            Err(e) => panic!("Failed to serialize metadata: {e}"),
        };
        match fs::write(&metadata_path, metadata_json).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to write metadata: {e}"),
        }

        let target_path = root.join("restored.db");

        // Should fail with checksum verification
        assert!(restore_backup(&backup_path, &target_path, true)
            .await
            .is_err());

        // Should succeed without checksum verification
        assert!(restore_backup(&backup_path, &target_path, false)
            .await
            .is_ok());
    }
}