//! Pure calculation functions for backup command handler (Tier 2).
//!
//! No I/O, no side effects. All functions are pure.

use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use super::data::{BackupCommand, BackupConfig, BackupInfo, RetentionStatusOutput};

// ============================================================================
// Validation
// ============================================================================

/// Known database names that can be backed up or restored.
const KNOWN_DATABASES: &[&str] = &["state.db", "beads.db"];

/// Validate a backup command before execution.
///
/// Ensures required fields are present and database names are recognized.
///
/// # Errors
///
/// Returns error if:
/// - Restore command has an empty database name
/// - Database name is not recognized
pub fn validate_backup_command(cmd: &BackupCommand) -> scp_core::Result<()> {
    match cmd {
        BackupCommand::Create
        | BackupCommand::List
        | BackupCommand::Retention
        | BackupCommand::Status => Ok(()),
        BackupCommand::Restore {
            database,
            timestamp: _,
        } => validate_database_name(database),
    }
}

/// Validate that a database name is recognized.
///
/// # Errors
///
/// Returns error if the database name is not in the known list.
fn validate_database_name(database: &str) -> scp_core::Result<()> {
    if KNOWN_DATABASES.contains(&database) {
        Ok(())
    } else {
        Err(scp_core::Error::internal(format!(
            "Unknown database: {database}. Valid options: {}",
            KNOWN_DATABASES.join(", ")
        )))
    }
}

/// Validate a backup timestamp format.
///
/// Expects format `YYYYMMDD-HHMMSS`.
///
/// # Errors
///
/// Returns error if the timestamp does not match the expected format.
pub fn validate_timestamp(timestamp: &str) -> scp_core::Result<()> {
    chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S")
        .map_err(|e| {
            scp_core::Error::internal(format!(
                "Invalid timestamp '{timestamp}': {e}. Expected format: YYYYMMDD-HHMMSS"
            ))
        })
        .map(|_| ())
}

// ============================================================================
// Filename Parsing
// ============================================================================

/// Parse timestamp from a backup filename.
///
/// Expects filename format: `backup-YYYYMMDD-HHMMSS.db`.
///
/// # Errors
///
/// Returns error if filename format is invalid.
pub fn parse_backup_filename(filename: &str) -> scp_core::Result<DateTime<Utc>> {
    let ts = filename
        .strip_prefix("backup-")
        .and_then(|s| s.strip_suffix(".db"))
        .ok_or_else(|| {
            scp_core::Error::internal(format!("Invalid backup filename format: {filename}"))
        })?;

    chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d-%H%M%S")
        .map(|dt| dt.and_utc())
        .map_err(|e| {
            scp_core::Error::internal(format!("Invalid timestamp in backup filename '{ts}': {e}"))
        })
}

/// Generate a backup filename with the given timestamp.
#[must_use]
pub fn generate_backup_filename(timestamp: &DateTime<Utc>) -> String {
    format!("backup-{}.db", timestamp.format("%Y%m%d-%H%M%S"))
}

// ============================================================================
// Path Calculations
// ============================================================================

/// Get the backup directory for a specific database.
#[must_use]
pub fn get_database_backup_dir(backup_root: &Path, database_name: &str) -> PathBuf {
    backup_root.join(database_name)
}

/// Determine the target database path from a database name relative to root.
///
/// # Errors
///
/// Returns error if the database name is not recognized.
pub fn resolve_database_target(root: &Path, database_name: &str) -> scp_core::Result<PathBuf> {
    validate_database_name(database_name)?;
    match database_name {
        "state.db" => Ok(root.join(".scp").join("state.db")),
        "beads.db" => Ok(root.join(".scp").join("beads.db")),
        _ => Err(scp_core::Error::internal(format!(
            "Unknown database: {database_name}"
        ))),
    }
}

/// Find the database source paths to back up.
///
/// Returns a list of (database_name, source_path) pairs.
#[must_use]
pub fn get_backupable_databases(root: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("state.db", root.join(".scp").join("state.db")),
        ("beads.db", root.join(".scp").join("beads.db")),
    ]
}

// ============================================================================
// Retention Calculations
// ============================================================================

/// Determine which backups should be removed based on retention policy.
///
/// Returns the subset of backups that exceed the retention count.
/// Since `list_database_backups` returns backups sorted newest-first,
/// the entries beyond `retention_count` are the oldest.
#[must_use]
pub fn backups_to_remove(backups: &[BackupInfo], retention_count: usize) -> &[BackupInfo] {
    if backups.len() <= retention_count {
        &[]
    } else {
        &backups[retention_count..]
    }
}

/// Format a byte count as a human-readable string.
#[must_use]
#[allow(clippy::cast_precision_loss)]
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

/// Build a `RetentionStatusOutput` from raw data (pure).
#[must_use]
pub fn build_retention_status(
    database_name: &str,
    backup_count: usize,
    total_size_bytes: u64,
    would_free_bytes: u64,
    retention_limit: usize,
) -> RetentionStatusOutput {
    let within_limit = backup_count <= retention_limit;
    RetentionStatusOutput {
        database_name: database_name.to_string(),
        backup_count,
        retention_limit,
        total_size_bytes,
        total_size_human: format_size(total_size_bytes),
        would_free_bytes,
        would_free_human: format_size(would_free_bytes),
        within_limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_parse_backup_filename_roundtrip() {
        let timestamp = Utc::now();
        let filename = generate_backup_filename(&timestamp);
        let parsed = parse_backup_filename(&filename).expect("should parse successfully");
        let diff = (timestamp - parsed).num_seconds().abs();
        assert!(diff <= 1, "Timestamps differ by {diff} seconds");
    }

    #[test]
    fn test_parse_backup_filename_invalid() {
        assert!(parse_backup_filename("invalid.txt").is_err());
        assert!(parse_backup_filename("backup-invalid.db").is_err());
        assert!(parse_backup_filename("backup-20250101-120000.txt").is_err());
    }

    #[test]
    fn test_validate_database_name() {
        assert!(validate_database_name("state.db").is_ok());
        assert!(validate_database_name("beads.db").is_ok());
        assert!(validate_database_name("unknown.db").is_err());
        assert!(validate_database_name("").is_err());
    }

    #[test]
    fn test_validate_timestamp() {
        assert!(validate_timestamp("20250101-120000").is_ok());
        assert!(validate_timestamp("invalid").is_err());
        assert!(validate_timestamp("2025-01-01 12:00:00").is_err());
    }

    #[test]
    fn test_validate_backup_command() {
        assert!(validate_backup_command(&BackupCommand::Create).is_ok());
        assert!(validate_backup_command(&BackupCommand::List).is_ok());
        assert!(validate_backup_command(&BackupCommand::Retention).is_ok());
        assert!(validate_backup_command(&BackupCommand::Status).is_ok());
        assert!(validate_backup_command(&BackupCommand::Restore {
            database: "state.db".to_string(),
            timestamp: None,
        })
        .is_ok());
        assert!(validate_backup_command(&BackupCommand::Restore {
            database: "unknown.db".to_string(),
            timestamp: None,
        })
        .is_err());
    }

    #[test]
    fn test_backups_to_remove() {
        let backups = vec![
            BackupInfo {
                path: PathBuf::from("/a/backup-20250101-030000.db"),
                timestamp: chrono::NaiveDateTime::parse_from_str("20250101-030000", "%Y%m%d-%H%M%S")
                    .expect("valid")
                    .and_utc(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/backup-20250101-020000.db"),
                timestamp: chrono::NaiveDateTime::parse_from_str("20250101-020000", "%Y%m%d-%H%M%S")
                    .expect("valid")
                    .and_utc(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/backup-20250101-010000.db"),
                timestamp: chrono::NaiveDateTime::parse_from_str("20250101-010000", "%Y%m%d-%H%M%S")
                    .expect("valid")
                    .and_utc(),
                metadata: None,
                size_bytes: 100,
            },
        ];
        // Retention count 2: remove 1 oldest
        let to_remove = backups_to_remove(&backups, 2);
        assert_eq!(to_remove.len(), 1);
        // Retention count 5: remove none
        let to_remove = backups_to_remove(&backups, 5);
        assert!(to_remove.is_empty());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.00 KB");
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_get_database_backup_dir() {
        let dir = get_database_backup_dir(Path::new("/backups"), "state.db");
        assert_eq!(dir, PathBuf::from("/backups/state.db"));
    }

    // ---- Additional edge case tests ----

    #[test]
    fn test_resolve_database_target_state_db() {
        let result = resolve_database_target(Path::new("/project"), "state.db");
        assert!(result.is_ok());
        assert_eq!(
            result.expect("ok"),
            PathBuf::from("/project/.scp/state.db")
        );
    }

    #[test]
    fn test_resolve_database_target_beads_db() {
        let result = resolve_database_target(Path::new("/project"), "beads.db");
        assert!(result.is_ok());
        assert_eq!(
            result.expect("ok"),
            PathBuf::from("/project/.scp/beads.db")
        );
    }

    #[test]
    fn test_resolve_database_target_unknown_db() {
        let result = resolve_database_target(Path::new("/project"), "unknown.db");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_backupable_databases_returns_both() {
        let root = Path::new("/root");
        let databases = get_backupable_databases(root);
        assert_eq!(databases.len(), 2);
        assert_eq!(databases[0].0, "state.db");
        assert_eq!(databases[1].0, "beads.db");
    }

    #[test]
    fn test_get_backupable_databases_paths() {
        let root = Path::new("/root");
        let databases = get_backupable_databases(root);
        assert_eq!(
            databases[0].1,
            PathBuf::from("/root/.scp/state.db")
        );
        assert_eq!(
            databases[1].1,
            PathBuf::from("/root/.scp/beads.db")
        );
    }

    #[test]
    fn test_format_size_zero_bytes() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn test_format_size_one_byte() {
        assert_eq!(format_size(1), "1 B");
    }

    #[test]
    fn test_format_size_exactly_one_kb() {
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn test_format_size_exactly_one_mb() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_format_size_exactly_one_gb() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_size_fractional_kb() {
        assert_eq!(format_size(1500), "1.46 KB");
    }

    #[test]
    fn test_backups_to_remove_exact_retention_count() {
        let backups = vec![
            BackupInfo {
                path: PathBuf::from("/a/1"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/2"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
        ];
        // Exact match: nothing to remove
        let to_remove = backups_to_remove(&backups, 2);
        assert!(to_remove.is_empty());
    }

    #[test]
    fn test_backups_to_remove_empty_list() {
        let backups: Vec<BackupInfo> = vec![];
        let to_remove = backups_to_remove(&backups, 5);
        assert!(to_remove.is_empty());
    }

    #[test]
    fn test_backups_to_remove_zero_retention() {
        let backups = vec![BackupInfo {
            path: PathBuf::from("/a/1"),
            timestamp: Utc::now(),
            metadata: None,
            size_bytes: 100,
        }];
        let to_remove = backups_to_remove(&backups, 0);
        assert_eq!(to_remove.len(), 1);
    }

    #[test]
    fn test_build_retention_status_within_limit() {
        let status = build_retention_status("state.db", 5, 5000, 0, 10);
        assert!(status.within_limit);
        assert_eq!(status.database_name, "state.db");
        assert_eq!(status.backup_count, 5);
        assert_eq!(status.retention_limit, 10);
        assert_eq!(status.total_size_bytes, 5000);
        assert_eq!(status.would_free_bytes, 0);
        assert!(!status.total_size_human.is_empty());
        assert!(!status.would_free_human.is_empty());
    }

    #[test]
    fn test_build_retention_status_over_limit() {
        let status = build_retention_status("beads.db", 12, 24_000, 4000, 10);
        assert!(!status.within_limit);
        assert_eq!(status.backup_count, 12);
        assert_eq!(status.would_free_bytes, 4000);
    }

    #[test]
    fn test_build_retention_status_exact_limit() {
        let status = build_retention_status("x.db", 10, 0, 0, 10);
        assert!(status.within_limit);
    }

    #[test]
    fn test_validate_timestamp_valid_various_formats() {
        assert!(validate_timestamp("20250101-000000").is_ok());
        assert!(validate_timestamp("20251231-235959").is_ok());
    }

    #[test]
    fn test_validate_backup_command_restore_with_timestamp() {
        let cmd = BackupCommand::Restore {
            database: "beads.db".to_string(),
            timestamp: Some("20250101-120000".to_string()),
        };
        assert!(validate_backup_command(&cmd).is_ok());
    }

    #[test]
    fn test_validate_backup_command_restore_empty_database() {
        let cmd = BackupCommand::Restore {
            database: "".to_string(),
            timestamp: None,
        };
        assert!(validate_backup_command(&cmd).is_err());
    }

    // ---- format_size edge cases ----

    #[test]
    fn test_format_size_max_u64() {
        // u64::MAX in bytes (~18.4 EB). The function only supports up to GB,
        // so it formats as a very large GB value. Must not panic.
        let result = format_size(u64::MAX);
        assert!(result.contains("GB"), "expected GB unit for max u64, got: {result}");
        assert!(result.ends_with(" GB"));
    }

    #[test]
    fn test_format_size_just_below_kb() {
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_just_below_mb() {
        assert_eq!(format_size(1_048_575), "1024.00 KB");
    }

    #[test]
    fn test_format_size_just_below_gb() {
        assert_eq!(format_size(1_073_741_823), "1024.00 MB");
    }

    // ---- validate_timestamp boundary values ----

    #[test]
    fn test_validate_timestamp_boundary_min_valid() {
        // Earliest reasonable timestamp: 0001-01-01 00:00:00
        // chrono NaiveDateTime range starts at year 1.
        assert!(validate_timestamp("00010101-000000").is_ok());
    }

    #[test]
    fn test_validate_timestamp_boundary_max_valid() {
        // Latest reasonable timestamp supported by chrono NaiveDateTime.
        // chrono NaiveDateTime max is year 9999-12-31 23:59:59.
        assert!(validate_timestamp("99991231-235959").is_ok());
    }

    #[test]
    fn test_validate_timestamp_leap_second() {
        // chrono NaiveDateTime supports leap seconds (:60), so this parses
        // successfully rather than failing.
        assert!(validate_timestamp("20250101-235960").is_ok());
    }

    #[test]
    fn test_validate_timestamp_invalid_month() {
        assert!(validate_timestamp("20251301-120000").is_err());
    }

    #[test]
    fn test_validate_timestamp_invalid_day() {
        assert!(validate_timestamp("20250132-120000").is_err());
    }

    #[test]
    fn test_validate_timestamp_invalid_hour() {
        assert!(validate_timestamp("20250101-240000").is_err());
    }

    #[test]
    fn test_validate_timestamp_invalid_minute() {
        assert!(validate_timestamp("20250101-126000").is_err());
    }

    #[test]
    fn test_validate_timestamp_invalid_second() {
        assert!(validate_timestamp("20250101-120061").is_err());
    }

    #[test]
    fn test_validate_timestamp_february_29_non_leap() {
        // 2025 is not a leap year, so Feb 29 is invalid.
        assert!(validate_timestamp("20250229-120000").is_err());
    }

    #[test]
    fn test_validate_timestamp_february_29_leap_year() {
        // 2024 is a leap year, so Feb 29 is valid.
        assert!(validate_timestamp("20240229-120000").is_ok());
    }

    #[test]
    fn test_validate_timestamp_empty_string() {
        assert!(validate_timestamp("").is_err());
    }

    #[test]
    fn test_validate_timestamp_correct_format_wrong_separator() {
        // Underscore instead of hyphen separator.
        assert!(validate_timestamp("20250101_120000").is_err());
    }

    // ---- backups_to_remove additional edge cases ----

    #[test]
    fn test_backups_to_remove_one_over_limit() {
        let backups = vec![
            BackupInfo {
                path: PathBuf::from("/a/1"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/2"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/3"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
        ];
        let to_remove = backups_to_remove(&backups, 2);
        assert_eq!(to_remove.len(), 1);
        assert_eq!(to_remove[0].path, PathBuf::from("/a/3"));
    }

    #[test]
    fn test_backups_to_remove_all_exceed_limit() {
        let backups = vec![
            BackupInfo {
                path: PathBuf::from("/a/1"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
            BackupInfo {
                path: PathBuf::from("/a/2"),
                timestamp: Utc::now(),
                metadata: None,
                size_bytes: 100,
            },
        ];
        let to_remove = backups_to_remove(&backups, 1);
        assert_eq!(to_remove.len(), 1);
    }

    #[test]
    fn test_backups_to_remove_single_backup_at_limit_one() {
        let backups = vec![BackupInfo {
            path: PathBuf::from("/a/only"),
            timestamp: Utc::now(),
            metadata: None,
            size_bytes: 50,
        }];
        let to_remove = backups_to_remove(&backups, 1);
        assert!(to_remove.is_empty());
    }
}
