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
use sqlx::Row;
use tokio::io::AsyncReadExt;

use crate::error::{Error, Result};
use crate::Error::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPolicy {
    FailFast,
    Warn,
    Silent,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self::Warn
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub policy: RecoveryPolicy,
    pub log_recovered: bool,
    pub auto_recover_corrupted_wal: bool,
    pub delete_corrupted_database: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            policy: RecoveryPolicy::default(),
            log_recovered: true,
            auto_recover_corrupted_wal: true,
            delete_corrupted_database: false,
        }
    }
}

pub async fn log_recovery(message: &str, config: &RecoveryConfig) -> Result<()> {
    if !config.log_recovered {
        return Ok(());
    }

    let isolate_dir = Path::new(".isolate");

    match tokio::fs::try_exists(isolate_dir).await {
        Ok(true) => {}
        _ => return Ok(()),
    }

    let log_path = isolate_dir.join("recovery.log");

    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let log_entry = format!("[{timestamp}] {message}\n");

    let _ = tokio::task::spawn_blocking(move || {
        use std::io::Write;

        let mut file = std::fs::File::options()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| {
                Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to open recovery log: {e}"),
                ))
            })?;

        file.lock_exclusive().map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to lock recovery log: {e}"),
            ))
        })?;

        file.write_all(log_entry.as_bytes()).map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write to recovery log: {e}"),
            ))
        })?;

        file.sync_all().map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to flush recovery log: {e}"),
            ))
        })?;

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::Internal(format!("Failed to join logging task: {e}")))?;
    Ok(())
}

#[must_use]
pub fn should_log_recovery(config: &RecoveryConfig) -> bool {
    config.log_recovered
}

pub async fn validate_database(db_path: &Path, config: &RecoveryConfig) -> Result<()> {
    let metadata = match tokio::fs::metadata(db_path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Database(format!(
                "Database file not found: {}",
                db_path.display()
            )));
        }
        Err(e) => {
            return Err(Database(format!("Cannot access database: {e}")));
        }
    };

    if metadata.len() < 100 {
        log_recovery(
            &format!("Database file too small: {} bytes", metadata.len()),
            config,
        )
        .await
        .ok();
        return Err(Database(format!(
            "Database file is too small to be valid: {} bytes (expected at least 100)",
            metadata.len(),
        )));
    }

    let mut file = tokio::fs::File::open(db_path)
        .await
        .map_err(|e| Database(format!("Cannot open database: {e}")))?;

    let mut header = [0u8; 16];
    file.read_exact(&mut header)
        .await
        .map_err(|e| Database(format!("Cannot read database header: {e}")))?;

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

        return Err(Database(format!(
            "Database file is corrupted (invalid magic bytes): {magic_hex}"
        )));
    }

    Ok(())
}

pub async fn repair_database(db_path: &Path, config: &RecoveryConfig) -> Result<()> {
    match config.policy {
        RecoveryPolicy::FailFast => {
            return Err(Database(format!(
                "Database repair is disabled in fail-fast mode: {}",
                db_path.display()
            )));
        }
        RecoveryPolicy::Warn => {
            eprintln!("⚠  Repairing corrupted database: {}", db_path.display());
            log_recovery(
                &format!("Repairing database: {}", db_path.display()),
                config,
            )
            .await
            .ok();
        }
        RecoveryPolicy::Silent => {
            log_recovery(
                &format!("Silently repairing database: {}", db_path.display()),
                config,
            )
            .await
            .ok();
        }
    }

    tokio::fs::remove_file(db_path).await.ok();

    let wal_path = db_path.with_extension("db-wal");
    let shm_path = db_path.with_extension("db-shm");

    tokio::fs::remove_file(&wal_path).await.ok();
    tokio::fs::remove_file(&shm_path).await.ok();

    Ok(())
}

pub async fn recover_incomplete_sessions(db_path: &Path, config: &RecoveryConfig) -> Result<usize> {
    use sqlx::sqlite::SqlitePoolOptions;

    if !tokio::fs::try_exists(db_path).await? {
        return Ok(0);
    }

    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            repair_database(db_path, config).await?;
            return Ok(0);
        }
    };

    let timeout_seconds = 300i64;
    let cutoff_time = chrono::Utc::now().timestamp() - timeout_seconds;

    let rows = sqlx::query(
        "SELECT name, created_at FROM sessions WHERE status = 'creating' AND created_at < ?",
    )
    .bind(cutoff_time)
    .fetch_all(&pool)
    .await
    .map_err(|e| Database(format!("Failed to query sessions: {e}")))?;

    let recovered_count = rows.len();

    if recovered_count > 0 {
        match config.policy {
            RecoveryPolicy::FailFast => {
                return Err(Database(format!(
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
            RecoveryPolicy::Warn => {
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
            RecoveryPolicy::Silent => {
                log_recovery(
                    &format!("Silently removing {recovered_count} incomplete session(s)"),
                    config,
                )
                .await
                .ok();
            }
        }

        for row in &rows {
            let name: String = row.get("name");
            sqlx::query("DELETE FROM sessions WHERE name = ?")
                .bind(&name)
                .execute(&pool)
                .await
                .map_err(|e| Database(format!("Failed to delete session: {e}")))?;
        }

        sqlx::query(
            "DELETE FROM state_transitions WHERE session_id NOT IN (SELECT id FROM sessions)",
        )
        .execute(&pool)
        .await
        .ok();
    }

    pool.close().await;

    Ok(recovered_count)
}

pub async fn periodic_cleanup(
    db_path: &Path,
    max_age_seconds: i64,
    config: &RecoveryConfig,
) -> Result<usize> {
    use sqlx::sqlite::SqlitePoolOptions;

    if !tokio::fs::try_exists(db_path).await? {
        return Ok(0);
    }

    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            repair_database(db_path, config).await?;
            return Ok(0);
        }
    };

    let cutoff_time = chrono::Utc::now().timestamp() - max_age_seconds;

    let result = sqlx::query(
        "DELETE FROM sessions
         WHERE status IN ('completed', 'failed')
         AND updated_at < ?",
    )
    .bind(cutoff_time)
    .execute(&pool)
    .await
    .map_err(|e| Database(format!("Failed to cleanup old sessions: {e}")))?;

    let deleted_count = result.rows_affected();

    let orphan_result = sqlx::query(
        "DELETE FROM state_transitions
         WHERE session_id NOT IN (SELECT id FROM sessions)",
    )
    .execute(&pool)
    .await
    .map_err(|e| Database(format!("Failed to cleanup orphaned transitions: {e}")))?;

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

    pool.close().await;

    Ok((deleted_count + orphan_count) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_log_recovery() {
        let config_true = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: true,
            auto_recover_corrupted_wal: true,
            delete_corrupted_database: false,
        };
        let config_false = RecoveryConfig {
            policy: RecoveryPolicy::Warn,
            log_recovered: false,
            auto_recover_corrupted_wal: true,
            delete_corrupted_database: false,
        };
        assert!(should_log_recovery(&config_true));
        assert!(!should_log_recovery(&config_false));
    }

    #[test]
    fn test_recovery_policy_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.policy, RecoveryPolicy::Warn);
        assert!(config.log_recovered);
    }
}
