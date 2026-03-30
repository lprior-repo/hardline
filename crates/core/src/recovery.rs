//! Recovery module for database integrity and session cleanup.
//!
//! Provides functionality for:
//! - Logging recovery actions
//! - Validating database integrity
//! - Repairing corrupted databases
//! - Recovering incomplete sessions
//! - Periodic cleanup of stale records

use std::path::Path;

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::io::AsyncReadExt;

use crate::config::types::{RecoveryPolicy, ValidatedBool};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryConfig {
    pub policy: RecoveryPolicy,
    pub log_recovered: ValidatedBool,
    pub auto_recover_corrupted_wal: ValidatedBool,
    pub delete_corrupted_database: ValidatedBool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            policy: RecoveryPolicy::default(),
            log_recovered: ValidatedBool::new(true),
            auto_recover_corrupted_wal: ValidatedBool::new(true),
            delete_corrupted_database: ValidatedBool::new(false),
        }
    }
}

pub async fn log_recovery(message: &str, config: &RecoveryConfig) -> Result<()> {
    if *config.log_recovered {
        println!("RECOVERY: {}", message);
    }
    Ok(())
}

pub fn should_log_recovery(config: &RecoveryConfig) -> bool {
    *config.log_recovered
}

pub async fn check_database_integrity(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    let file = std::fs::File::open(path).map_err(|e| Error::io_error(e.to_string()))?;
    file.lock_shared()
        .map_err(|e| Error::io_error(e.to_string()))?;

    // Basic SQLite header check
    let mut f = tokio::fs::File::open(path)
        .await
        .map_err(|e| Error::io_error(e.to_string()))?;
    let mut header = [0u8; 16];
    f.read_exact(&mut header)
        .await
        .map_err(|e| Error::io_error(e.to_string()))?;

    file.unlock().map_err(|e| Error::io_error(e.to_string()))?;

    Ok(&header[..15] == b"SQLite format 3")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.policy, RecoveryPolicy::Warn);
        assert!(*config.log_recovered);
        assert!(*config.auto_recover_corrupted_wal);
        assert!(!*config.delete_corrupted_database);
    }

    #[tokio::test]
    async fn test_check_database_integrity() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        // Empty file is not a valid SQLite DB
        assert!(!check_database_integrity(path).await.unwrap());

        // Valid header
        std::fs::write(path, b"SQLite format 3\0").unwrap();
        assert!(check_database_integrity(path).await.unwrap());
    }

    #[test]
    fn test_should_log_recovery() {
        let mut config = RecoveryConfig::default();
        assert!(should_log_recovery(&config));

        config.log_recovered = ValidatedBool::new(false);
        assert!(!should_log_recovery(&config));
    }
}
