//! LockManager core implementation.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use super::types::Ttl;
use crate::Result;

/// Default lock TTL in seconds (5 minutes).
const DEFAULT_TTL_SECS: u64 = 300;

/// Maximum session name length (SQLite TEXT limit).
const MAX_SESSION_NAME_LEN: usize = 255;

/// Maximum allowed TTL in seconds (24 hours).
const MAX_TTL_SECS: u64 = 86400;

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
            ttl: Duration::seconds(DEFAULT_TTL_SECS as i64),
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
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

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
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

        Ok(())
    }

    /// Validate session name according to contract constraints.
    pub(super) fn validate_session_name(session: &str) -> crate::Result<()> {
        if session.is_empty() {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::EmptySessionName(
                    "Session name cannot be empty".to_string(),
                ),
            ));
        }
        if session.len() > MAX_SESSION_NAME_LEN {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::SessionNameTooLong(
                    "Session name cannot exceed 255 characters".to_string(),
                ),
            ));
        }
        if session.chars().any(|c| c.is_control()) {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::InvalidSessionName(
                    "Session name must not contain control characters".to_string(),
                ),
            ));
        }
        Ok(())
    }

    /// Validate agent ID according to contract constraints.
    pub(super) fn validate_agent_id(agent_id: &str) -> crate::Result<()> {
        if agent_id.is_empty() {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::EmptyAgentId("Agent ID cannot be empty".to_string()),
            ));
        }
        Ok(())
    }

    /// Validate TTL value according to contract constraints.
    pub(super) fn validate_ttl(ttl_seconds: u64) -> crate::Result<Ttl> {
        if ttl_seconds == u64::MAX {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::TtlOverflow("TTL overflow detected".to_string()),
            ));
        }
        if ttl_seconds > MAX_TTL_SECS {
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::TtlOutOfRange(
                    "TTL must be in range [0, 86400]".to_string(),
                ),
            ));
        }
        Ttl::new(ttl_seconds).ok_or_else(|| {
            crate::error::Error::from(super::errors::LockErrorKind::TtlOutOfRange(
                "TTL must be in range [0, 86400]".to_string(),
            ))
        })
    }

    /// Log a lock operation to the audit trail.
    pub(super) async fn log_operation(
        &self,
        session: &str,
        agent_id: &str,
        operation: super::types::LockOperation,
    ) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO session_lock_audit (session, agent_id, operation, timestamp)
             VALUES (?, ?, ?, ?)",
        )
        .bind(session)
        .bind(agent_id)
        .bind(operation.as_str())
        .bind(&now_str)
        .execute(&self.db)
        .await
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

        Ok(())
    }

    /// Verify that a session exists in the sessions table.
    ///
    /// This is called before acquiring a lock to prevent orphaned locks.
    /// Returns Ok(()) if the sessions table doesn't exist (graceful degradation).
    pub async fn verify_session_exists(&self, session: &str) -> crate::Result<()> {
        let query_result = sqlx::query("SELECT name FROM sessions WHERE name = ?")
            .bind(session)
            .fetch_optional(&self.db)
            .await;

        match query_result {
            Ok(None) => Err(crate::error::Error::from(
                super::errors::LockErrorKind::SessionNotFound {
                    session: session.to_string(),
                },
            )),
            Ok(Some(_)) => Ok(()),
            Err(e) => {
                // If sessions table doesn't exist, allow locks (graceful degradation)
                let msg = e.to_string();
                if msg.contains("no such table") {
                    return Ok(());
                }
                Err(crate::error::Error::from(
                    super::errors::LockErrorKind::DatabaseError(format!(
                        "Failed to query sessions: {e}"
                    )),
                ))
            }
        }
    }
}
