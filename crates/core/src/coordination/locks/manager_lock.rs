//! Lock acquisition operations.

use chrono::{DateTime, Duration, Utc};

use super::helpers::is_constraint_conflict_error;
use super::manager::LockManager;
use super::types::{LockOperation, LockResponse};
use crate::Result;

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
        // Validate inputs
        Self::validate_session_name(session)?;
        Self::validate_agent_id(agent_id)?;
        let _ttl = Self::validate_ttl(ttl_seconds)?;

        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let existing: Option<(String, String, String)> = sqlx::query_as(
            "SELECT lock_id, agent_id, expires_at
             FROM session_locks
             WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

        if let Some((existing_lock_id, holder_agent_id, existing_expires_str)) = existing {
            if holder_agent_id == agent_id {
                let existing_expires = DateTime::parse_from_rfc3339(&existing_expires_str)
                    .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::ParseError(e.to_string())))?
                    .with_timezone(&Utc);
                return Ok(LockResponse {
                    lock_id: existing_lock_id,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    acquired_at: now,
                    expires_at: existing_expires,
                });
            }
            return Err(crate::error::Error::from(super::errors::LockErrorKind::SessionLocked {
                session: session.to_string(),
                holder: holder_agent_id,
            }));
        }

        self.verify_session_exists(session).await?;

        sqlx::query("DELETE FROM session_locks WHERE session = ? AND expires_at < ?")
            .bind(session)
            .bind(&now_str)
            .execute(&self.db)
            .await
            .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

        let ttl = if ttl_seconds > 0 {
            Duration::seconds(i64::try_from(ttl_seconds).unwrap_or(300))
        } else {
            self.ttl
        };
        
        let expires_at = now + ttl;
        let expires_str = expires_at.to_rfc3339();
        let nanos = now
            .timestamp_nanos_opt()
            .ok_or_else(|| crate::error::Error::from(super::errors::LockErrorKind::Unknown("Failed to get timestamp nanos".to_string())))?;
        let lock_id = format!("lock-{session}-{nanos}");

        let insert_result = sqlx::query(
            "INSERT INTO session_locks (lock_id, session, agent_id, acquired_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&lock_id)
        .bind(session)
        .bind(agent_id)
        .bind(&now_str)
        .bind(&expires_str)
        .execute(&self.db)
        .await;

        if let Err(e) = insert_result {
            if is_constraint_conflict_error(&e) {
                let holder: Option<(String,)> =
                    sqlx::query_as("SELECT agent_id FROM session_locks WHERE session = ?")
                        .bind(session)
                        .fetch_optional(&self.db)
                        .await
                        .map_err(|db_err| {
                            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(format!(
                                "Failed to query lock holder after conflict: {db_err}"
                            )))
                        })?;

                let holder_agent_id = holder.map_or_else(|| "unknown".to_string(), |(id,)| id);
                return Err(crate::error::Error::from(super::errors::LockErrorKind::SessionLocked {
                    session: session.to_string(),
                    holder: holder_agent_id,
                }));
            }

            return Err(crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(format!(
                "Failed to acquire lock with TTL: {e}"
            ))));
        }

        if let Err(log_error) = self.log_operation(session, agent_id, LockOperation::Lock).await {
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
}
