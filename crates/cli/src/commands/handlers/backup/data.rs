//! Data types for backup command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of backup commands.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Input Types
// ============================================================================

/// Backup command variants (CLI subcommand representation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupCommand {
    /// Create backups of all databases
    Create,
    /// List all available backups
    List,
    /// Restore a database from backup
    Restore {
        /// Database name to restore (e.g. "state.db", "beads.db")
        database: String,
        /// Specific backup timestamp (format: YYYYMMDD-HHMMSS); None = latest
        timestamp: Option<String>,
    },
    /// Apply retention policy
    Retention,
    /// Show backup status and retention information
    Status,
}

// ============================================================================
// Backup Metadata
// ============================================================================

/// Metadata stored alongside each backup file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// When the backup was created
    pub created_at: DateTime<Utc>,
    /// Original database file name
    pub database_name: String,
    /// Size of backup file in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum for integrity verification
    pub checksum: String,
}

impl BackupMetadata {
    /// Create new backup metadata.
    #[must_use]
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

// ============================================================================
// Backup Configuration
// ============================================================================

/// Backup system configuration.
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Number of backups to retain per database
    pub retention_count: usize,
    /// Root directory for backup storage
    pub backup_dir: PathBuf,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            retention_count: 10,
            backup_dir: PathBuf::from(".scp/backups"),
        }
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// Information about a single backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Path to backup file
    pub path: PathBuf,
    /// Timestamp extracted from filename
    pub timestamp: DateTime<Utc>,
    /// Associated metadata (if available)
    pub metadata: Option<BackupMetadata>,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Result of a backup create operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCreateOutput {
    /// Number of backups created
    pub backup_count: usize,
    /// Paths to created backups
    pub backup_paths: Vec<String>,
}

/// Result of a backup list operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupListOutput {
    /// Backups grouped by database name
    pub databases: Vec<DatabaseBackups>,
}

/// Backups for a single database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackups {
    /// Database name
    pub database: String,
    /// Number of backups
    pub backup_count: usize,
    /// Individual backup entries
    pub backups: Vec<BackupInfoOutput>,
}

/// Single backup entry for list output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfoOutput {
    /// Path to backup file
    pub path: String,
    /// Timestamp of backup
    pub timestamp: DateTime<Utc>,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Result of a backup restore operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreOutput {
    /// Database name that was restored
    pub database: String,
    /// Source backup path
    pub restored_from: String,
    /// Target database path
    pub restored_to: String,
}

/// Result of a retention policy application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRetentionOutput {
    /// Number of backups removed
    pub removed_count: usize,
    /// Paths to removed backups
    pub removed_backups: Vec<String>,
}

/// Retention status for a single database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStatusOutput {
    /// Database name
    pub database_name: String,
    /// Current number of backups
    pub backup_count: usize,
    /// Maximum backups to retain
    pub retention_limit: usize,
    /// Total disk space used
    pub total_size_bytes: u64,
    /// Human-readable total size
    pub total_size_human: String,
    /// Disk space that would be freed by applying retention
    pub would_free_bytes: u64,
    /// Human-readable would-free size
    pub would_free_human: String,
    /// Whether backup count is within the retention limit
    pub within_limit: bool,
}

/// Result of backup status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatusOutput {
    /// Maximum backups per database
    pub max_backups_per_database: usize,
    /// Per-database status
    pub databases: Vec<RetentionStatusOutput>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- BackupCommand ----

    #[test]
    fn backup_command_create_equality() {
        assert_eq!(BackupCommand::Create, BackupCommand::Create);
    }

    #[test]
    fn backup_command_list_equality() {
        assert_eq!(BackupCommand::List, BackupCommand::List);
    }

    #[test]
    fn backup_command_restore_equality() {
        let a = BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        };
        let b = BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn backup_command_restore_with_timestamp() {
        let cmd = BackupCommand::Restore {
            database: "beads.db".to_string(),
            timestamp: Some("20250101-120000".to_string()),
        };
        if let BackupCommand::Restore { database, timestamp } = cmd {
            assert_eq!(database, "beads.db");
            assert_eq!(timestamp.as_deref(), Some("20250101-120000"));
        }
    }

    #[test]
    fn backup_command_restore_none_timestamp() {
        let cmd = BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        };
        if let BackupCommand::Restore { timestamp, .. } = cmd {
            assert!(timestamp.is_none());
        }
    }

    #[test]
    fn backup_command_retention_equality() {
        assert_eq!(BackupCommand::Retention, BackupCommand::Retention);
    }

    #[test]
    fn backup_command_status_equality() {
        assert_eq!(BackupCommand::Status, BackupCommand::Status);
    }

    #[test]
    fn backup_command_different_variants_inequality() {
        assert_ne!(BackupCommand::Create, BackupCommand::List);
    }

    // ---- BackupMetadata ----

    #[test]
    fn backup_metadata_new_construction() {
        let meta = BackupMetadata::new(
            "state.db".to_string(),
            1024,
            "abc123checksum".to_string(),
        );
        assert_eq!(meta.database_name, "state.db");
        assert_eq!(meta.size_bytes, 1024);
        assert_eq!(meta.checksum, "abc123checksum");
    }

    #[test]
    fn backup_metadata_new_sets_created_at() {
        let before = chrono::Utc::now();
        let meta = BackupMetadata::new("db".to_string(), 0, "".to_string());
        let after = chrono::Utc::now();
        assert!(meta.created_at >= before && meta.created_at <= after);
    }

    #[test]
    fn backup_metadata_serialization_roundtrip() {
        let meta = BackupMetadata::new(
            "beads.db".to_string(),
            2048,
            "sha256".to_string(),
        );
        let json = serde_json::to_string(&meta).expect("serialize");
        let deserialized: BackupMetadata =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.database_name, "beads.db");
        assert_eq!(deserialized.size_bytes, 2048);
        assert_eq!(deserialized.checksum, "sha256");
    }

    // ---- BackupConfig ----

    #[test]
    fn backup_config_default_values() {
        let config = BackupConfig::default();
        assert_eq!(config.retention_count, 10);
        assert_eq!(config.backup_dir, PathBuf::from(".scp/backups"));
    }

    #[test]
    fn backup_config_custom_values() {
        let config = BackupConfig {
            retention_count: 5,
            backup_dir: PathBuf::from("/custom/backups"),
        };
        assert_eq!(config.retention_count, 5);
        assert_eq!(config.backup_dir, PathBuf::from("/custom/backups"));
    }

    // ---- BackupInfo ----

    #[test]
    fn backup_info_construction() {
        let info = BackupInfo {
            path: PathBuf::from("/backups/state.db/backup-20250101-120000.db"),
            timestamp: chrono::Utc::now(),
            metadata: None,
            size_bytes: 4096,
        };
        assert_eq!(info.size_bytes, 4096);
        assert!(info.metadata.is_none());
    }

    #[test]
    fn backup_info_with_metadata() {
        let meta = BackupMetadata::new("db".to_string(), 100, "hash".to_string());
        let info = BackupInfo {
            path: PathBuf::from("/a/b"),
            timestamp: chrono::Utc::now(),
            metadata: Some(meta),
            size_bytes: 100,
        };
        assert!(info.metadata.is_some());
        assert_eq!(info.metadata.as_ref().unwrap().checksum, "hash");
    }

    #[test]
    fn backup_info_serialization_roundtrip() {
        let info = BackupInfo {
            path: PathBuf::from("/b"),
            timestamp: chrono::Utc::now(),
            metadata: None,
            size_bytes: 50,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: BackupInfo =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.size_bytes, 50);
    }

    // ---- BackupCreateOutput ----

    #[test]
    fn backup_create_output_construction() {
        let output = BackupCreateOutput {
            backup_count: 2,
            backup_paths: vec!["/a/1.db".to_string(), "/a/2.db".to_string()],
        };
        assert_eq!(output.backup_count, 2);
        assert_eq!(output.backup_paths.len(), 2);
    }

    #[test]
    fn backup_create_output_empty() {
        let output = BackupCreateOutput {
            backup_count: 0,
            backup_paths: vec![],
        };
        assert_eq!(output.backup_count, 0);
        assert!(output.backup_paths.is_empty());
    }

    #[test]
    fn backup_create_output_serialization_roundtrip() {
        let output = BackupCreateOutput {
            backup_count: 1,
            backup_paths: vec!["/p".to_string()],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BackupCreateOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.backup_count, 1);
    }

    // ---- BackupListOutput ----

    #[test]
    fn backup_list_output_construction() {
        let output = BackupListOutput {
            databases: vec![DatabaseBackups {
                database: "state.db".to_string(),
                backup_count: 3,
                backups: vec![],
            }],
        };
        assert_eq!(output.databases.len(), 1);
        assert_eq!(output.databases[0].database, "state.db");
    }

    #[test]
    fn backup_list_output_serialization_roundtrip() {
        let output = BackupListOutput { databases: vec![] };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BackupListOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(deserialized.databases.is_empty());
    }

    // ---- DatabaseBackups ----

    #[test]
    fn database_backups_construction() {
        let db = DatabaseBackups {
            database: "beads.db".to_string(),
            backup_count: 5,
            backups: vec![BackupInfoOutput {
                path: "/a".to_string(),
                timestamp: chrono::Utc::now(),
                size_bytes: 100,
            }],
        };
        assert_eq!(db.backup_count, 5);
        assert_eq!(db.backups.len(), 1);
    }

    #[test]
    fn database_backups_serialization_roundtrip() {
        let db = DatabaseBackups {
            database: "x".to_string(),
            backup_count: 0,
            backups: vec![],
        };
        let json = serde_json::to_string(&db).expect("serialize");
        let deserialized: DatabaseBackups =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.database, "x");
    }

    // ---- BackupInfoOutput ----

    #[test]
    fn backup_info_output_construction() {
        let info = BackupInfoOutput {
            path: "/backups/file.db".to_string(),
            timestamp: chrono::Utc::now(),
            size_bytes: 999,
        };
        assert_eq!(info.size_bytes, 999);
    }

    #[test]
    fn backup_info_output_serialization_roundtrip() {
        let info = BackupInfoOutput {
            path: "/p".to_string(),
            timestamp: chrono::Utc::now(),
            size_bytes: 1,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: BackupInfoOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.size_bytes, 1);
    }

    // ---- BackupRestoreOutput ----

    #[test]
    fn backup_restore_output_construction() {
        let output = BackupRestoreOutput {
            database: "state.db".to_string(),
            restored_from: "/backups/state.db/backup-20250101.db".to_string(),
            restored_to: ".scp/state.db".to_string(),
        };
        assert_eq!(output.database, "state.db");
        assert_eq!(output.restored_from, "/backups/state.db/backup-20250101.db");
        assert_eq!(output.restored_to, ".scp/state.db");
    }

    #[test]
    fn backup_restore_output_serialization_roundtrip() {
        let output = BackupRestoreOutput {
            database: "d".to_string(),
            restored_from: "f".to_string(),
            restored_to: "t".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BackupRestoreOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.database, "d");
    }

    // ---- BackupRetentionOutput ----

    #[test]
    fn backup_retention_output_construction() {
        let output = BackupRetentionOutput {
            removed_count: 3,
            removed_backups: vec!["/a/old1.db".to_string(), "/a/old2.db".to_string(), "/a/old3.db".to_string()],
        };
        assert_eq!(output.removed_count, 3);
        assert_eq!(output.removed_backups.len(), 3);
    }

    #[test]
    fn backup_retention_output_empty() {
        let output = BackupRetentionOutput {
            removed_count: 0,
            removed_backups: vec![],
        };
        assert_eq!(output.removed_count, 0);
        assert!(output.removed_backups.is_empty());
    }

    #[test]
    fn backup_retention_output_serialization_roundtrip() {
        let output = BackupRetentionOutput {
            removed_count: 0,
            removed_backups: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BackupRetentionOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.removed_count, 0);
    }

    // ---- RetentionStatusOutput ----

    #[test]
    fn retention_status_output_construction_within_limit() {
        let status = RetentionStatusOutput {
            database_name: "state.db".to_string(),
            backup_count: 5,
            retention_limit: 10,
            total_size_bytes: 5000,
            total_size_human: "4.88 KB".to_string(),
            would_free_bytes: 0,
            would_free_human: "0 B".to_string(),
            within_limit: true,
        };
        assert!(status.within_limit);
        assert_eq!(status.backup_count, 5);
    }

    #[test]
    fn retention_status_output_construction_over_limit() {
        let status = RetentionStatusOutput {
            database_name: "beads.db".to_string(),
            backup_count: 15,
            retention_limit: 10,
            total_size_bytes: 15_000,
            total_size_human: "14.65 KB".to_string(),
            would_free_bytes: 5000,
            would_free_human: "4.88 KB".to_string(),
            within_limit: false,
        };
        assert!(!status.within_limit);
        assert_eq!(status.would_free_bytes, 5000);
    }

    #[test]
    fn retention_status_output_serialization_roundtrip() {
        let status = RetentionStatusOutput {
            database_name: "x".to_string(),
            backup_count: 1,
            retention_limit: 10,
            total_size_bytes: 100,
            total_size_human: "100 B".to_string(),
            would_free_bytes: 0,
            would_free_human: "0 B".to_string(),
            within_limit: true,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: RetentionStatusOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.database_name, "x");
        assert!(deserialized.within_limit);
    }

    // ---- BackupStatusOutput ----

    #[test]
    fn backup_status_output_construction() {
        let status = BackupStatusOutput {
            max_backups_per_database: 10,
            databases: vec![RetentionStatusOutput {
                database_name: "state.db".to_string(),
                backup_count: 3,
                retention_limit: 10,
                total_size_bytes: 3000,
                total_size_human: "2.93 KB".to_string(),
                would_free_bytes: 0,
                would_free_human: "0 B".to_string(),
                within_limit: true,
            }],
        };
        assert_eq!(status.max_backups_per_database, 10);
        assert_eq!(status.databases.len(), 1);
    }

    #[test]
    fn backup_status_output_empty_databases() {
        let status = BackupStatusOutput {
            max_backups_per_database: 5,
            databases: vec![],
        };
        assert_eq!(status.max_backups_per_database, 5);
        assert!(status.databases.is_empty());
    }

    #[test]
    fn backup_status_output_serialization_roundtrip() {
        let status = BackupStatusOutput {
            max_backups_per_database: 3,
            databases: vec![],
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: BackupStatusOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.max_backups_per_database, 3);
    }
}
