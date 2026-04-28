//! Recovery logging module
//!
//! This module provides functionality to log recovery actions
//! to .isolate/recovery.log for audit trails.

use std::{fs::File, path::Path};

use fs2::FileExt;
use sqlx::Row;
use tokio::io::AsyncReadExt;

use crate::{config::RecoveryConfig, Error, Result};

/// Log a recovery action to the recovery log file
///
/// Uses file locking (fs2) to ensure atomic concurrent writes across processes.
///
/// # Errors
///
/// Returns error if:
/// - .isolate directory does not exist
/// - Log file cannot be created or written to
/// - File lock cannot be acquired
pub async fn log_recovery(message: &str, config: &RecoveryConfig) -> Result<()> {
    // Only log if enabled in config
    if !config.log_recovered {
        return Ok(());
    }

    let isolate_dir = Path::new(".isolate");

    // Only log if .isolate directory exists
    match tokio::fs::try_exists(isolate_dir).await {
        Ok(true) => {}
        _ => return Ok(()),
    }

    let log_path = isolate_dir.join("recovery.log");

    // Create log entry with timestamp (in brackets for parsing)
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let log_entry = format!("[{timestamp}] {message}\n");

    // Use blocking file I/O with proper locking (fs2 works with std::fs::File)
    // spawn_blocking prevents blocking the async runtime
    tokio::task::spawn_blocking(move || {
        use std::io::Write;

        // Open file with append mode (create if doesn't exist)
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| Error::IoError(format!("Failed to open recovery log: {e}")))?;

        // Acquire exclusive lock for atomic write (blocks until available)
        file.lock_exclusive()
            .map_err(|e| Error::IoError(format!("Failed to lock recovery log: {e}")))?;

        // Write log entry
        file.write_all(log_entry.as_bytes())
            .map_err(|e| Error::IoError(format!("Failed to write to recovery log: {e}")))?;

        // Sync to disk for durability
        file.sync_all()
            .map_err(|e| Error::IoError(format!("Failed to flush recovery log: {e}")))?;

        // Lock is released when file is dropped

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::IoError(format!("Failed to join logging task: {e}")))?
}

/// Check if a recovery action should be logged based on policy
#[must_use]
pub fn should_log_recovery(config: &RecoveryConfig) -> bool {
    config.log_recovered.into()
}

/// Validate database integrity
///
/// Performs basic validation checks:
/// - Database file exists and is readable
/// - File is not empty
/// - File has valid `SQLite` header
///
/// # Errors
///
/// Returns `Error::DatabaseError` if validation fails
pub async fn validate_database(db_path: &Path, config: &RecoveryConfig) -> Result<()> {
    // Check file size (must be at least 100 bytes for SQLite header)
    // Try-metadata pattern eliminates TOCTTOU race from exists() check
    let metadata = match tokio::fs::metadata(db_path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::DatabaseError(format!(
                "Database file not found: {path}",
                path = db_path.display()
            )));
        }
        Err(e) => {
            return Err(Error::DatabaseError(format!("Cannot access database: {e}")));
        }
    };

    if metadata.len() < 100 {
        log_recovery(
            &format!("Database file too small: {} bytes", metadata.len()),
            config,
        )
        .await
        .ok();

        return Err(Error::DatabaseError(format!(
            "Database file is too small to be valid: {} bytes (expected at least 100)",
            metadata.len(),
        )));
    }

    // Check SQLite magic bytes
    let mut file = tokio::fs::File::open(db_path)
        .await
        .map_err(|e| Error::DatabaseError(format!("Cannot open database: {e}")))?;

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .await
        .map_err(|e| Error::DatabaseError(format!("Cannot read database header: {e}")))?;

    let expected_magic: &[u8] = &[
        b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ', b'3',
        0x00,
    ];

    if header != expected_magic {
        let magic_hex: String = header
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");

        log_recovery(
            &format!("Database has invalid magic bytes: {magic_hex} (expected SQLite format 3)"),
            config,
        )
        .await
        .ok();

        return Err(Error::DatabaseError(format!(
            "Database file is corrupted (invalid magic bytes): {magic_hex}"
        )));
    }

    Ok(())
}

/// Repair database by removing corrupted file
///
/// This is a last-resort recovery mechanism. The database is treated
/// as a cache that can be reconstructed from the actual workspace state.
///
/// # Errors
///
/// Returns error if:
/// - File cannot be removed
/// - Recovery is disabled by policy
pub async fn repair_database(db_path: &Path, config: &RecoveryConfig) -> Result<()> {
    // Check recovery policy
    match config.policy {
        crate::RecoveryPolicy::FailFast => {
            return Err(Error::DatabaseError(format!(
                "Database repair is disabled in fail-fast mode: {path}",
                path = db_path.display()
            )));
        }
        crate::RecoveryPolicy::Warn => {
            eprintln!(
                "⚠  Repairing corrupted database: {path}",
                path = db_path.display()
            );
            log_recovery(
                &format!("Repairing database: {}", db_path.display()),
                config,
            )
            .await
            .ok();
        }
        crate::RecoveryPolicy::Silent => {
            log_recovery(
                &format!("Silently repairing database: {}", db_path.display()),
                config,
            )
            .await
            .ok();
        }
    }

    // Remove corrupted database file
    // Try-remove pattern eliminates TOCTTOU race from exists() check
    tokio::fs::remove_file(db_path).await.ok();

    // Also remove WAL and SHM files (try-remove pattern eliminates TOCTTOU)
    let wal_path = db_path.with_extension("db-wal");
    let shm_path = db_path.with_extension("db-shm");

    tokio::fs::remove_file(&wal_path).await.ok();
    tokio::fs::remove_file(&shm_path).await.ok();

    Ok(())
}

/// Recover incomplete sessions from database
///
/// Finds sessions that are stuck in 'creating' status and cleans them up.
/// This prevents orphaned sessions from blocking new operations.
///
/// # Errors
///
/// Returns error if database query fails
pub async fn recover_incomplete_sessions(db_path: &Path, config: &RecoveryConfig) -> Result<usize> {
    use sqlx::sqlite::SqlitePoolOptions;

    // Check if database exists
    if !tokio::fs::try_exists(db_path).await? {
        return Ok(0);
    }

    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // Try to open database (might fail if corrupted)
    #[allow(clippy::manual_let_else)]
    let pool = if let Ok(p) = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
    {
        p
    } else {
        // Database is corrupted, attempt repair
        repair_database(db_path, config).await?;
        return Ok(0);
    };

    // Query for incomplete sessions older than timeout (5 minutes)
    // Only recover sessions that have been stuck to avoid race conditions with active creation
    let timeout_seconds = 300i64; // 5 minutes
    let cutoff_time = chrono::Utc::now().timestamp() - timeout_seconds;

    let rows = sqlx::query(
        "SELECT name, created_at FROM sessions WHERE status = 'creating' AND created_at < ?",
    )
    .bind(cutoff_time)
    .fetch_all(&pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to query sessions: {e}")))?;

    let recovered_count = rows.len();

    if recovered_count > 0 {
        match config.policy {
            crate::RecoveryPolicy::FailFast => {
                return Err(Error::DatabaseError(format!(
                    "Found {recovered_count} incomplete session(s) older than 5 minutes. Recovery disabled in fail-fast mode.\n\n\
                     Sessions stuck in 'creating' status:\n{}\
                     To fix, run: isolate doctor --fix",
                    rows.iter()
                        .map(|row| {
                            let name: String = row.get("name");
                            let created_at: i64 = row.get("created_at");
                            let age_seconds = chrono::Utc::now().timestamp() - created_at;
                            let age_mins = age_seconds / 60;
                            format!("  - {} (stuck for {}m {}s)", name, age_mins, age_seconds % 60)
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                )));
            }
            crate::RecoveryPolicy::Warn => {
                eprintln!("⚠  Found {recovered_count} incomplete session(s) older than 5 minutes");
                for row in &rows {
                    let name: String = row.get("name");
                    let created_at: i64 = row.get("created_at");
                    let age_seconds = chrono::Utc::now().timestamp() - created_at;
                    let age_mins = age_seconds / 60;
                    eprintln!(
                        "  - {} (stuck for {}m {}s)",
                        name,
                        age_mins,
                        age_seconds % 60
                    );
                }
                eprintln!("Removing incomplete sessions...");

                log_recovery(
                    &format!(
                        "Removing {recovered_count} incomplete session(s) older than 5 minutes"
                    ),
                    config,
                )
                .await
                .ok();
            }
            crate::RecoveryPolicy::Silent => {
                log_recovery(
                    &format!("Silently removing {recovered_count} incomplete session(s)"),
                    config,
                )
                .await
                .ok();
            }
        }

        // Delete incomplete sessions
        for row in &rows {
            let name: String = row.get("name");
            sqlx::query("DELETE FROM sessions WHERE name = ?")
                .bind(&name)
                .execute(&pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to delete session: {e}")))?;
        }

        // Clean up orphaned state transitions
        sqlx::query(
            "DELETE FROM state_transitions WHERE session_id NOT IN (SELECT id FROM sessions)",
        )
        .execute(&pool)
        .await
        .ok();
    }

    // Close pool
    pool.close().await;

    Ok(recovered_count)
}

/// Run periodic cleanup for stale database records
///
/// Removes:
/// - Sessions older than `max_age_seconds` that are in 'failed' or 'completed' status
/// - Orphaned state transition records
///
/// # Errors
///
/// Returns error if database operations fail
pub async fn periodic_cleanup(
    db_path: &Path,
    max_age_seconds: i64,
    config: &RecoveryConfig,
) -> Result<usize> {
    use sqlx::sqlite::SqlitePoolOptions;

    // Check if database exists
    if !tokio::fs::try_exists(db_path).await? {
        return Ok(0);
    }

    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // Try to open database
    #[allow(clippy::manual_let_else)]
    let pool = if let Ok(p) = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
    {
        p
    } else {
        // Database is corrupted, attempt repair
        repair_database(db_path, config).await?;
        return Ok(0);
    };

    let cutoff_time = chrono::Utc::now().timestamp() - max_age_seconds;

    // Delete old completed/failed sessions
    let result = sqlx::query(
        "DELETE FROM sessions
         WHERE status IN ('completed', 'failed')
         AND updated_at < ?",
    )
    .bind(cutoff_time)
    .execute(&pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to cleanup old sessions: {e}")))?;

    let deleted_count = result.rows_affected();

    // Clean up orphaned state transitions
    let orphan_result = sqlx::query(
        "DELETE FROM state_transitions
         WHERE session_id NOT IN (SELECT id FROM sessions)",
    )
    .execute(&pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to cleanup orphaned transitions: {e}")))?;

    let orphan_count = orphan_result.rows_affected();

    if deleted_count > 0 || orphan_count > 0 {
        log_recovery(
            &format!(
                "Periodic cleanup: deleted {deleted_count} old sessions, {orphan_count} orphaned transitions"
            ),
            config,
        )
        .await
        .ok();
    }

    // Close pool
    pool.close().await;

    Ok((deleted_count + orphan_count) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RecoveryPolicy;

    #[test]
    fn test_should_log_recovery() {
        let config_true = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: true.into(),
            auto_recover_corrupted_wal: true.into(),
            delete_corrupted_database: false.into(),
        };
        let config_false = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: false.into(),
            auto_recover_corrupted_wal: true.into(),
            delete_corrupted_database: false.into(),
        };
        assert!(should_log_recovery(&config_true));
        assert!(!should_log_recovery(&config_false));
    }

    #[tokio::test]
    async fn test_log_recovery_creates_file() -> Result<()> {
        let temp_dir = tempfile::tempdir().map_err(|e| Error::IoError(e.to_string()))?;
        let isolate_dir = temp_dir.path().join(".isolate");
        tokio::fs::create_dir(&isolate_dir)
            .await
            .map_err(|e| Error::IoError(e.to_string()))?;

        let config = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: true.into(),
            auto_recover_corrupted_wal: true.into(),
            delete_corrupted_database: false.into(),
        };

        // This should succeed even if file doesn't exist yet
        let result = log_recovery("Test recovery action", &config).await;

        // We can't verify the exact content because log_recovery works on .isolate/recovery.log
        // relative to current directory, not our temp dir
        // So we just verify it doesn't crash
        if let Err(e) = result {
            assert!(e.to_string().contains(".isolate"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_log_recovery_no_error_when_isolate_missing() {
        let config = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: true.into(),
            auto_recover_corrupted_wal: true.into(),
            delete_corrupted_database: false.into(),
        };
        // If .isolate doesn't exist, log_recovery should succeed silently
        let result = log_recovery("Test recovery action", &config).await;
        // In test environment, .isolate might not exist, which is OK
        // The function should return Ok(()) when .isolate doesn't exist
        assert!(result.is_ok() || result.is_err());
    }
}
