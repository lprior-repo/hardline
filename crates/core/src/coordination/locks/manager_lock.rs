//! Lock acquisition operations.

use chrono::{DateTime, Duration, Utc};

use super::{
    helpers::is_constraint_conflict_error,
    manager::LockManager,
    types::{LockOperation, LockResponse},
};
use crate::Result;

/// Parameters required to insert a new lock row.
struct LockInsertParams<'a> {
    session: &'a str,
    agent_id: &'a str,
    lock_id: &'a str,
    now_str: &'a str,
    expires_str: &'a str,
}

impl LockManager {
    /// Acquire an exclusive lock on a session with custom TTL.
    ///
    /// Returns `SessionLocked` error if another agent holds a valid lock.
    /// Returns `SessionNotFound` error if the session doesn't exist.
    pub async fn lock_with_ttl(
        &self,
        session: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> Result<LockResponse> {
        Self::validate_session_name(session)?;
        Self::validate_agent_id(agent_id)?;
        let _ttl = Self::validate_ttl(ttl_seconds)?;

        let now = Utc::now();
        let now_str = now.to_rfc3339();

        if let Some(response) = self.check_existing_lock(session, agent_id, &now_str, now).await? {
            return Ok(response);
        }

        self.verify_session_exists(session).await?;
        self.cleanup_expired_locks(session, &now_str).await?;

        let ttl = if ttl_seconds > 0 {
            // SAFETY: ttl_seconds is validated to be <= 86400, which always fits in i64
            Duration::seconds(ttl_seconds as i64)
        } else {
            self.ttl
        };
        let expires_at = now + ttl;
        let expires_str = expires_at.to_rfc3339();
        let nanos = now.timestamp_nanos_opt().ok_or_else(|| {
            crate::error::Error::from(super::errors::LockErrorKind::Unknown(
                "Failed to get timestamp nanos".to_string(),
            ))
        })?;
        let lock_id = format!("lock-{session}-{nanos}");

        self.insert_new_lock(LockInsertParams {
            session,
            agent_id,
            lock_id: &lock_id,
            now_str: &now_str,
            expires_str: &expires_str,
        })
        .await?;

        if let Err(log_error) = self
            .log_operation(session, agent_id, LockOperation::Lock)
            .await
        {
            let _ = sqlx::query("DELETE FROM session_locks WHERE lock_id = ?")
                .bind(&lock_id)
                .execute(&self.db)
                .await;
            return Err(log_error);
        }

        Ok(LockResponse {
            lock_id,
            session: session.to_string(),
            agent_id: agent_id.to_string(),
            acquired_at: now,
            expires_at,
        })
    }

    /// Acquire an exclusive lock on a session with default TTL (300s).
    ///
    /// Wrapper around `lock_with_ttl` with `ttl_seconds=0`.
    pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
        self.lock_with_ttl(session, agent_id, 0).await
    }

    /// Check for an existing lock on the session.
    ///
    /// Returns `Ok(Some(response))` if the caller already holds a valid lock,
    /// `Ok(None)` if no conflicting lock exists, or `Err` if another agent holds the lock.
    async fn check_existing_lock(
        &self,
        session: &str,
        agent_id: &str,
        now_str: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<LockResponse>> {
        let existing: Option<(String, String, String)> = sqlx::query_as(
            "SELECT lock_id, agent_id, expires_at
             FROM session_locks
             WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

        let Some((existing_lock_id, holder_agent_id, existing_expires_str)) = existing else {
            return Ok(None);
        };

        if holder_agent_id == agent_id {
            let existing_expires = DateTime::parse_from_rfc3339(&existing_expires_str)
                .map_err(|e| {
                    crate::error::Error::from(super::errors::LockErrorKind::ParseError(
                        e.to_string(),
                    ))
                })?
                .with_timezone(&Utc);
            return Ok(Some(LockResponse {
                lock_id: existing_lock_id,
                session: session.to_string(),
                agent_id: agent_id.to_string(),
                acquired_at: now,
                expires_at: existing_expires,
            }));
        }

        Err(crate::error::Error::from(
            super::errors::LockErrorKind::SessionLocked {
                session: session.to_string(),
                holder: holder_agent_id,
            },
        ))
    }

    /// Remove expired lock rows for a session.
    async fn cleanup_expired_locks(
        &self,
        session: &str,
        now_str: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM session_locks WHERE session = ? AND expires_at < ?")
            .bind(session)
            .bind(now_str)
            .execute(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(
                    e.to_string(),
                ))
            })?;
        Ok(())
    }

    /// Insert a new lock row, handling constraint conflicts.
    async fn insert_new_lock(&self, params: LockInsertParams<'_>) -> Result<()> {
        let insert_result = sqlx::query(
            "INSERT INTO session_locks (lock_id, session, agent_id, acquired_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(params.lock_id)
        .bind(params.session)
        .bind(params.agent_id)
        .bind(params.now_str)
        .bind(params.expires_str)
        .execute(&self.db)
        .await;

        if let Err(e) = insert_result {
            if is_constraint_conflict_error(&e) {
                return self.handle_constraint_conflict(params.session).await;
            }
            return Err(crate::error::Error::from(
                super::errors::LockErrorKind::DatabaseError(format!(
                    "Failed to acquire lock with TTL: {e}"
                )),
            ));
        }
        Ok(())
    }

    /// Handle a constraint conflict on insert by querying the current lock holder.
    async fn handle_constraint_conflict(&self, session: &str) -> Result<()> {
        let holder: Option<(String,)> =
            sqlx::query_as("SELECT agent_id FROM session_locks WHERE session = ?")
                .bind(session)
                .fetch_optional(&self.db)
                .await
                .map_err(|db_err| {
                    crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(
                        format!("Failed to query lock holder after conflict: {db_err}"),
                    ))
                })?;

        let holder_agent_id = holder.map_or_else(|| "unknown".to_string(), |(id,)| id);
        Err(crate::error::Error::from(
            super::errors::LockErrorKind::SessionLocked {
                session: session.to_string(),
                holder: holder_agent_id,
            },
        ))
    }
}
