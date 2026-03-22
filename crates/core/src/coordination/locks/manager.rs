//! LockManager core implementation.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::{Error, Result};

/// Default lock TTL in seconds (5 minutes).
const DEFAULT_TTL_SECS: i64 = 300;

/// Manages exclusive session locks backed by `SQLite`.
#[derive(Debug, Clone)]
pub struct LockManager {
    pub(super) db: SqlitePool,
    pub(super) ttl: Duration,
}

impl LockManager {
    /// Create a new `LockManager` with default TTL.
    #[must_use]
    pub const fn new(db: SqlitePool) -> Self {
        Self {
            db,
            ttl: Duration::seconds(DEFAULT_TTL_SECS),
        }
    }

    /// Get the database pool
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.db
    }

    /// Create a new `LockManager` with a custom TTL.
    #[must_use]
    pub const fn with_ttl(db: SqlitePool, ttl: Duration) -> Self {
        Self { db, ttl }
    }

    /// Initialize the locks table.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS session_locks (
                lock_id TEXT PRIMARY KEY,
                session TEXT NOT NULL UNIQUE,
                agent_id TEXT NOT NULL,
                acquired_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
        )
        .execute(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS session_lock_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
        )
        .execute(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Log a lock operation to the audit trail.
    pub(super) async fn log_operation(
        &self,
        session: &str,
        agent_id: &str,
        operation: &str,
    ) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO session_lock_audit (session, agent_id, operation, timestamp)
             VALUES (?, ?, ?, ?)",
        )
        .bind(session)
        .bind(agent_id)
        .bind(operation)
        .bind(&now_str)
        .execute(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Verify that a session exists in the sessions table.
    ///
    /// This is called before acquiring a lock to prevent orphaned locks.
    pub(super) async fn verify_session_exists(&self, session: &str) -> Result<()> {
        let query_result = sqlx::query("SELECT name FROM sessions WHERE name = ?")
            .bind(session)
            .fetch_optional(&self.db)
            .await;

        match query_result {
            Ok(None) => Err(Error::SessionNotFound(session.to_string())),
            Ok(Some(_)) => Ok(()),
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("no such table") || error_msg.contains("does not exist") {
                    Ok(())
                } else {
                    Err(Error::Database(format!("Failed to query sessions: {e}")))
                }
            }
        }
    }
}
