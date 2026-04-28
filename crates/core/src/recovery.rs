//! Recovery module for database integrity and session cleanup.
//!
//! Provides functionality for:
//! - Logging recovery actions
//! - Validating database integrity
//! - Repairing corrupted databases
//! - Recovering incomplete sessions
//! - Periodic cleanup of stale records

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
    config::types::ValidatedBool,
    error::{Error, Result},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryPolicy {
    #[default]
    Warn,
    Repair,
    Panic,
}

impl std::fmt::Display for RecoveryPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Warn => write!(f, "warn"),
            Self::Repair => write!(f, "repair"),
            Self::Panic => write!(f, "panic"),
        }
    }
}

impl std::str::FromStr for RecoveryPolicy {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "warn" => Ok(Self::Warn),
            "repair" => Ok(Self::Repair),
            "panic" => Ok(Self::Panic),
            _ => Err(Error::config_invalid(format!(
                "Invalid recovery policy: {s}"
            ))),
        }
    }
}

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
    if f.read_exact(&mut header).await.is_err() {
        // File too small to be a valid SQLite database
        let _ = file.unlock();
        return Ok(false);
    }

    file.unlock().map_err(|e| Error::io_error(e.to_string()))?;

    Ok(&header[..15] == b"SQLite format 3")
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // RecoveryPolicy tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_recovery_policy_default_is_warn() {
        assert_eq!(RecoveryPolicy::default(), RecoveryPolicy::Warn);
    }

    #[test]
    fn test_recovery_policy_all_variants() {
        let all = [
            RecoveryPolicy::Warn,
            RecoveryPolicy::Repair,
            RecoveryPolicy::Panic,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_recovery_policy_copy() {
        let policy = RecoveryPolicy::Repair;
        let copied = policy;
        assert_eq!(policy, copied);
    }

    #[test]
    fn test_recovery_policy_clone() {
        let policy = RecoveryPolicy::Panic;
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    #[test]
    fn test_recovery_policy_display() {
        assert_eq!(format!("{}", RecoveryPolicy::Warn), "warn");
        assert_eq!(format!("{}", RecoveryPolicy::Repair), "repair");
        assert_eq!(format!("{}", RecoveryPolicy::Panic), "panic");
    }

    #[test]
    fn test_recovery_policy_from_str_valid() {
        assert_eq!(
            "warn".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Warn
        );
        assert_eq!(
            "repair".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Repair
        );
        assert_eq!(
            "panic".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Panic
        );
    }

    #[test]
    fn test_recovery_policy_from_str_case_insensitive() {
        assert_eq!(
            "WARN".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Warn
        );
        assert_eq!(
            "Repair".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Repair
        );
        assert_eq!(
            "PANIC".parse::<RecoveryPolicy>().unwrap(),
            RecoveryPolicy::Panic
        );
    }

    #[test]
    fn test_recovery_policy_from_str_invalid() {
        let result = "invalid".parse::<RecoveryPolicy>();
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_policy_from_str_empty() {
        let result = "".parse::<RecoveryPolicy>();
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_policy_debug() {
        assert_eq!(format!("{:?}", RecoveryPolicy::Warn), "Warn");
        assert_eq!(format!("{:?}", RecoveryPolicy::Repair), "Repair");
        assert_eq!(format!("{:?}", RecoveryPolicy::Panic), "Panic");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RecoveryConfig tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.policy, RecoveryPolicy::Warn);
        assert!(*config.log_recovered);
        assert!(*config.auto_recover_corrupted_wal);
        assert!(!*config.delete_corrupted_database);
    }

    #[test]
    fn test_recovery_config_clone() {
        let config = RecoveryConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_recovery_config_equality() {
        let a = RecoveryConfig::default();
        let b = RecoveryConfig::default();
        assert_eq!(a, b);

        let c = RecoveryConfig {
            policy: RecoveryPolicy::Panic,
            ..Default::default()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn test_recovery_config_custom_policy() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Repair,
            ..Default::default()
        };
        assert_eq!(config.policy, RecoveryPolicy::Repair);
        // Other fields keep defaults
        assert!(*config.log_recovered);
        assert!(*config.auto_recover_corrupted_wal);
        assert!(!*config.delete_corrupted_database);
    }

    #[test]
    fn test_recovery_config_all_flags_enabled() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Panic,
            log_recovered: ValidatedBool::new(true),
            auto_recover_corrupted_wal: ValidatedBool::new(true),
            delete_corrupted_database: ValidatedBool::new(true),
        };
        assert!(*config.log_recovered);
        assert!(*config.auto_recover_corrupted_wal);
        assert!(*config.delete_corrupted_database);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // should_log_recovery
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_should_log_recovery_true() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: ValidatedBool::new(true),
            auto_recover_corrupted_wal: ValidatedBool::new(true),
            delete_corrupted_database: ValidatedBool::new(false),
        };
        assert!(should_log_recovery(&config));
    }

    #[test]
    fn test_should_log_recovery_false() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: ValidatedBool::new(false),
            auto_recover_corrupted_wal: ValidatedBool::new(true),
            delete_corrupted_database: ValidatedBool::new(false),
        };
        assert!(!should_log_recovery(&config));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // log_recovery
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_log_recovery_returns_ok() {
        let config = RecoveryConfig::default();
        let result = log_recovery("test message", &config).await;
        assert!(result.is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // check_database_integrity
    // ═══════════════════════════════════════════════════════════════════════

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

    #[tokio::test]
    async fn test_check_database_integrity_nonexistent_path() {
        let path = std::path::Path::new("/tmp/nonexistent_db_file_test_12345.db");
        // Non-existent path should return Ok(true) — nothing to be corrupt
        assert!(check_database_integrity(path).await.unwrap());
    }

    #[tokio::test]
    async fn test_check_database_integrity_truncated_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        // Write only a few bytes — too short for SQLite header
        std::fs::write(path, b"SQL").unwrap();
        assert!(!check_database_integrity(path).await.unwrap());
    }

    #[tokio::test]
    async fn test_check_database_integrity_wrong_magic() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        // Wrong magic bytes
        std::fs::write(path, b"PostgreSQL format 3\0").unwrap();
        assert!(!check_database_integrity(path).await.unwrap());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_recovery_policy_serde_roundtrip_all_variants() {
        for policy in [
            RecoveryPolicy::Warn,
            RecoveryPolicy::Repair,
            RecoveryPolicy::Panic,
        ] {
            let json = serde_json::to_string(&policy).expect("serialize ok");
            let deserialized: RecoveryPolicy = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(policy, deserialized);
        }
    }

    #[test]
    fn test_recovery_policy_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&RecoveryPolicy::Warn).expect("ok"),
            "\"warn\""
        );
        assert_eq!(
            serde_json::to_string(&RecoveryPolicy::Repair).expect("ok"),
            "\"repair\""
        );
        assert_eq!(
            serde_json::to_string(&RecoveryPolicy::Panic).expect("ok"),
            "\"panic\""
        );
    }

    #[test]
    fn test_recovery_config_serde_roundtrip() {
        let config = RecoveryConfig::default();
        let json = serde_json::to_string(&config).expect("serialize ok");
        let deserialized: RecoveryConfig = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_recovery_config_serde_custom() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Panic,
            log_recovered: ValidatedBool::new(false),
            auto_recover_corrupted_wal: ValidatedBool::new(false),
            delete_corrupted_database: ValidatedBool::new(true),
        };
        let json = serde_json::to_string(&config).expect("serialize ok");
        let deserialized: RecoveryConfig = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(config, deserialized);
    }
}
