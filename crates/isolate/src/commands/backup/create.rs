//! Backup creation functionality

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::fs;

use super::backup_internal::{
    compute_checksum, generate_backup_filename, get_database_backup_dir, BackupConfig,
    BackupMetadata,
};

/// Create a backup of a database file
///
/// # Errors
///
/// Returns error if:
/// - Source database file does not exist
/// - Backup directory cannot be created
/// - File copy fails
/// - Metadata cannot be written
#[allow(dead_code)]
// Core backup functionality used by backup commands
pub async fn create_backup(database_path: &Path, config: &BackupConfig) -> Result<PathBuf> {
    // Validate source database exists
    anyhow::ensure!(
        database_path.exists(),
        "Source database does not exist: {}",
        database_path.display()
    );

    // Get database name from path
    let database_name = database_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;

    // Create backup directory structure
    let backup_dir = get_database_backup_dir(&config.backup_dir, database_name);
    fs::create_dir_all(&backup_dir)
        .await
        .context("Failed to create backup directory")?;

    // Generate backup filename with timestamp
    let timestamp = Utc::now();
    let backup_filename = generate_backup_filename(&timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // Copy database file to backup location
    fs::copy(database_path, &backup_path)
        .await
        .context("Failed to copy database file")?;

    // Get backup file size
    let metadata = fs::metadata(&backup_path)
        .await
        .context("Failed to get backup file metadata")?;
    let size_bytes = metadata.len();

    // Compute checksum for integrity verification
    let checksum = compute_checksum(&backup_path)
        .await
        .context("Failed to compute backup checksum")?;

    // Create and save metadata
    let backup_metadata = BackupMetadata::new(database_name.to_string(), size_bytes, checksum);
    let metadata_path = backup_path.with_extension("json");
    let metadata_json = serde_json::to_string_pretty(&backup_metadata)
        .context("Failed to serialize backup metadata")?;

    fs::write(&metadata_path, metadata_json)
        .await
        .context("Failed to write backup metadata")?;

    Ok(backup_path)
}

/// Create backups for all isolate databases
///
/// # Errors
///
/// Returns error if any backup operation fails
#[allow(dead_code)]
// High-level backup orchestration function
pub async fn backup_all_databases(root: &Path, config: &BackupConfig) -> Result<Vec<PathBuf>> {
    // Only state.db now - queue.db merged into state.db (bd-30s)
    let databases = vec!["state.db"];

    let isolate_dir = root.join(".isolate");
    let beads_dir = root.join(".beads");

    let mut backup_paths = Vec::new();

    // Backup .isolate databases
    for db_name in &databases {
        let db_path = isolate_dir.join(db_name);
        if db_path.exists() {
            match create_backup(&db_path, config).await {
                Ok(path) => backup_paths.push(path),
                Err(e) => {
                    tracing::warn!("Failed to backup {}: {}", db_name, e);
                    // Continue with other databases
                }
            }
        }
    }

    // Backup beads.db
    let beads_db_path = beads_dir.join("beads.db");
    if beads_db_path.exists() {
        match create_backup(&beads_db_path, config).await {
            Ok(path) => backup_paths.push(path),
            Err(e) => {
                tracing::warn!("Failed to backup beads.db: {}", e);
            }
        }
    }

    Ok(backup_paths)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_create_backup_creates_files() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let root = temp_dir.path();

        // Create test database file
        let db_path = root.join("test.db");
        match fs::write(&db_path, b"test data").await {
            Ok(_) => {}
            Err(e) => panic!("Failed to write test data: {e}"),
        }

        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        let backup_path = match create_backup(&db_path, &config).await {
            Ok(path) => path,
            Err(e) => panic!("Failed to create backup: {e}"),
        };

        assert!(backup_path.exists());
        assert!(backup_path.with_extension("json").exists());
    }

    #[tokio::test]
    async fn test_create_backup_nonexistent_database() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let root = temp_dir.path();

        let db_path = root.join("nonexistent.db");
        let config = BackupConfig {
            backup_dir: root.join("backups"),
            ..Default::default()
        };

        assert!(create_backup(&db_path, &config).await.is_err());
    }
}