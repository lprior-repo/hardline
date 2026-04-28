//! Automated database backup system for isolate
//!
//! Provides backup and restore functionality for `SQLite` databases (`state.db`, `beads.db`)
//! with configurable retention policies and automated periodic backups.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Backup metadata stored alongside backup files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Timestamp when backup was created
    pub created_at: DateTime<Utc>,
    /// Original database file name
    pub database_name: String,
    /// Size of backup file in bytes
    pub size_bytes: u64,
    /// Checksum for integrity verification (SHA-256)
    pub checksum: String,
}

impl BackupMetadata {
    /// Create new backup metadata
    ///
    /// # Errors
    ///
    /// Returns error if checksum computation fails
    #[allow(dead_code)]
    // Part of public API for backup metadata
    pub fn new(database_name: String, size_bytes: u64, checksum: String) -> Self {
        let created_at = Utc::now();
        Self {
            created_at,
            database_name,
            size_bytes,
            checksum,
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Number of backups to retain per database
    pub retention_count: usize,
    /// Backup directory path
    pub backup_dir: PathBuf,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            retention_count: 10,
            backup_dir: PathBuf::from(".isolate/backups"),
        }
    }
}

impl BackupConfig {
    /// Create backup config with custom retention
    #[allow(dead_code)]
    // Builder pattern method for backup configuration
    pub const fn with_retention(mut self, count: usize) -> Self {
        self.retention_count = count;
        self
    }

    /// Create backup config with custom backup directory
    #[allow(dead_code)]
    // Builder pattern method for backup configuration
    pub fn with_backup_dir(mut self, dir: PathBuf) -> Self {
        self.backup_dir = dir;
        self
    }
}

/// Compute SHA-256 checksum of a file
///
/// # Errors
///
/// Returns error if file cannot be read
#[allow(dead_code)]
// Used by backup system for integrity verification
pub async fn compute_checksum(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use tokio::{fs::File, io::AsyncReadExt};

    let mut file = File::open(path)
        .await
        .context("Failed to open file for checksum")?;

    let mut hasher = Sha256::new();

    // Read file in chunks to avoid loading entire file into memory
    let mut chunk_buffer = vec![0u8; 8192];
    loop {
        let bytes_read = file
            .read(&mut chunk_buffer)
            .await
            .context("Failed to read file for checksum")?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&chunk_buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}

/// Get backup directory for a specific database
///
/// # Errors
///
/// Never fails (pure function)
#[allow(dead_code)]
// Helper function for backup file organization
pub fn get_database_backup_dir(backup_root: &Path, database_name: &str) -> PathBuf {
    backup_root.join(database_name)
}

/// Generate backup filename with timestamp
///
/// # Errors
///
/// Never fails (pure function)
#[allow(dead_code)]
// Helper function for backup file naming
pub fn generate_backup_filename(timestamp: &DateTime<Utc>) -> String {
    format!("backup-{}.db", timestamp.format("%Y%m%d-%H%M%S"))
}

/// Parse timestamp from backup filename
///
/// # Errors
///
/// Returns error if filename format is invalid
#[allow(dead_code)]
// Helper function for backup file parsing
pub fn parse_backup_filename(filename: &str) -> Result<DateTime<Utc>> {
    let ts = filename
        .strip_prefix("backup-")
        .and_then(|s| s.strip_suffix(".db"))
        .ok_or_else(|| anyhow::anyhow!("Invalid backup filename format: {filename}"))?;

    chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d-%H%M%S")
        .map(|dt| dt.and_utc())
        .map_err(|_| anyhow::anyhow!("Invalid timestamp in backup filename: {ts}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_generate_and_parse_backup_filename() {
        let timestamp = Utc::now();
        let filename = generate_backup_filename(&timestamp);
        let parsed = match parse_backup_filename(&filename) {
            Ok(parsed) => parsed,
            Err(e) => panic!("Backup filename should parse successfully: {e}"),
        };
        // Allow 1-second difference for formatting precision
        let diff = (timestamp - parsed).num_seconds().abs();
        assert!(diff <= 1, "Timestamps differ by {diff} seconds");
    }

    #[test]
    fn test_parse_backup_filename_invalid() {
        assert!(parse_backup_filename("invalid.txt").is_err());
        assert!(parse_backup_filename("backup-invalid.db").is_err());
    }
}