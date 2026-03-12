//! File watching for beads database changes
//!
//! Monitors `.beads/beads.db` in all workspace directories and emits
//! events when changes are detected. Events are debounced to prevent
//! excessive updates during bulk changes.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//!
//! use scp_core::{
//!     config::WatchConfig,
//!     watcher::{FileWatcher, WatchEvent},
//! };
//!
//! # async fn example() -> scp_core::Result<()> {
//! let config = WatchConfig {
//!     enabled: true,
//!     debounce_ms: 100,
//!     paths: vec![".beads/beads.db".to_string()],
//! };
//!
//! let workspaces = vec![PathBuf::from("/path/to/workspace")];
//! let mut rx = FileWatcher::watch_workspaces(&config, &workspaces)?;
//!
//! while let Some(event) = rx.recv().await {
//!     match event {
//!         WatchEvent::BeadsChanged { workspace_path } => {
//!             // Update UI
//!             println!("Beads changed in {:?}", workspace_path);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use notify::RecursiveMode;
#[cfg(test)]
use notify_debouncer_mini::DebouncedEventKind;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::{config::WatchConfig, error::Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Events emitted by the file watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// Beads database changed in a workspace
    BeadsChanged { workspace_path: PathBuf },
}

/// Beads status for a workspace
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BeadsStatus {
    /// No beads database found
    NoBeads,
    /// Beads database with issue counts
    Counts {
        open: u32,
        in_progress: u32,
        blocked: u32,
        closed: u32,
    },
}

/// File watcher for beads database changes
pub struct FileWatcher;

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

impl FileWatcher {
    /// Watch beads databases in multiple workspaces
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Watcher is disabled in config
    /// - Debounce duration is invalid
    /// - Unable to watch any of the workspace paths
    /// - Unable to create event channel
    pub fn watch_workspaces(
        config: &WatchConfig,
        workspaces: &[PathBuf],
    ) -> Result<mpsc::Receiver<WatchEvent>> {
        if !config.enabled {
            return Err(Error::ConfigInvalid("File watcher is disabled".to_string()));
        }

        if config.debounce_ms < 10 || config.debounce_ms > 5000 {
            return Err(Error::ConfigInvalid(format!(
                "debounce_ms must be between 10 and 5000, got {}",
                config.debounce_ms
            )));
        }

        let (tx, rx) = mpsc::channel(100);

        let mut debouncer = new_debouncer(
            Duration::from_millis(u64::from(config.debounce_ms)),
            move |res: notify_debouncer_mini::DebounceEventResult| {
                if let Ok(events) = res {
                    for event in events {
                        if let Some(workspace_path) = extract_workspace_path(&event) {
                            let _ = tx.blocking_send(WatchEvent::BeadsChanged { workspace_path });
                        }
                    }
                }
            },
        )
        .map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create file watcher: {e}"),
            ))
        })?;

        // Watch each workspace's beads database
        workspaces.iter().try_for_each(|workspace| {
            let beads_db = workspace.join(".beads/beads.db");
            let file_watch_result = debouncer
                .watcher()
                .watch(&beads_db, RecursiveMode::NonRecursive);

            if file_watch_result.is_err() {
                if let Some(parent) = beads_db.parent() {
                    let parent_watch_result = debouncer
                        .watcher()
                        .watch(parent, RecursiveMode::NonRecursive);

                    if parent_watch_result.is_err() {
                        tracing::debug!(
                            "Skipping watcher for {} because neither file nor parent is watchable yet",
                            beads_db.display()
                        );
                    }
                }
            }
            Ok::<(), Error>(())
        })?;

        tokio::spawn(async move {
            let _debouncer = debouncer;
            tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
        });

        Ok(rx)
    }
}

/// Query beads status for a workspace
///
/// # Errors
///
/// Returns error if:
/// - Unable to open database
/// - Database query fails
/// - Database schema is invalid
pub async fn query_beads_status(pool: &SqlitePool, workspace_path: &Path) -> Result<BeadsStatus> {
    let beads_db = workspace_path.join(".beads/beads.db");

    match tokio::fs::try_exists(&beads_db).await {
        Ok(true) => {
            let open = query_count(pool, "open").await?;
            let in_progress = query_count(pool, "in_progress").await?;
            let blocked = query_count(pool, "blocked").await?;
            let closed = query_count(pool, "closed").await?;

            Ok(BeadsStatus::Counts {
                open,
                in_progress,
                blocked,
                closed,
            })
        }
        _ => Ok(BeadsStatus::NoBeads),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

fn extract_workspace_path(event: &DebouncedEvent) -> Option<PathBuf> {
    event
        .path
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
}

async fn query_count(pool: &SqlitePool, status: &str) -> Result<u32> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE status = ?")
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query {status} count: {e}")))?;

    u32::try_from(count)
        .map_err(|_| Error::Database(format!("Issue count exceeds u32::MAX: {count}")))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_watcher_disabled() {
        let config = WatchConfig {
            enabled: false,
            debounce_ms: 100,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, &[]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, Error::ConfigInvalid(_)));
        }
    }

    #[test]
    fn test_watcher_invalid_debounce_too_low() {
        let config = WatchConfig {
            enabled: true,
            debounce_ms: 5,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, &[]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::ConfigInvalid(_)));
        }
    }

    #[test]
    fn test_watcher_invalid_debounce_too_high() {
        let config = WatchConfig {
            enabled: true,
            debounce_ms: 10000,
            paths: vec![".beads/beads.db".to_string()],
        };

        let result = FileWatcher::watch_workspaces(&config, &[]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::ConfigInvalid(_)));
        }
    }

    #[tokio::test]
    async fn test_query_beads_status_no_beads() -> Result<()> {
        let Ok(temp_dir) = TempDir::new() else {
            return Ok(());
        };

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::Database(format!("Failed to create in-memory pool: {e}")))?;

        let result = query_beads_status(&pool, temp_dir.path()).await;

        assert!(result.is_ok());
        if let Ok(status) = result {
            assert_eq!(status, BeadsStatus::NoBeads);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_query_beads_status_with_database() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create temp dir: {e}"),
            ))
        })?;
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir(&beads_dir).map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create beads dir: {e}"),
            ))
        })?;

        let db_path = beads_dir.join("beads.db");
        let path_str = db_path.to_str().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid UTF-8 in path".to_string(),
            ))
        })?;
        let db_url = format!("sqlite:///{path_str}?mode=rwc");
        let pool = SqlitePool::connect(&db_url)
            .await
            .map_err(|e| Error::Database(format!("Failed to open DB: {e}")))?;

        sqlx::query(
            "CREATE TABLE issues (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create table: {e}")))?;

        sqlx::query("INSERT INTO issues (id, status) VALUES ('1', 'open')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('2', 'in_progress')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('3', 'in_progress')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('4', 'blocked')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('5', 'closed')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('6', 'closed')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;
        sqlx::query("INSERT INTO issues (id, status) VALUES ('7', 'closed')")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to insert test data: {e}")))?;

        let result = query_beads_status(&pool, temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(status) = result {
            if let BeadsStatus::Counts {
                open,
                in_progress,
                blocked,
                closed,
            } = status
            {
                assert_eq!(open, 1);
                assert_eq!(in_progress, 2);
                assert_eq!(blocked, 1);
                assert_eq!(closed, 3);
            } else {
                return Err(Error::InvalidState(
                    "Expected Counts, got NoBeads".to_string(),
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn test_extract_workspace_path() {
        let event = DebouncedEvent {
            path: PathBuf::from("/workspace/.beads/beads.db"),
            kind: DebouncedEventKind::Any,
        };

        let result = extract_workspace_path(&event);
        assert!(result.is_some());
        if let Some(path) = result {
            assert_eq!(path, PathBuf::from("/workspace"));
        }
    }

    #[test]
    fn test_watch_event_equality() {
        let event1 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/workspace"),
        };
        let event2 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/workspace"),
        };
        let event3 = WatchEvent::BeadsChanged {
            workspace_path: PathBuf::from("/other"),
        };

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    #[test]
    fn test_beads_status_equality() {
        let status1 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status2 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status3 = BeadsStatus::NoBeads;

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }
}
