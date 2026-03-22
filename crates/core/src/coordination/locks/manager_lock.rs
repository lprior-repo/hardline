//! Lock acquisition operations.

use chrono::{DateTime, Duration, Utc};

use crate::{Error, Result};

use super::helpers::is_constraint_conflict_error;
use super::types::LockResponse;
use super::manager::LockManager;

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
        let ttl = if ttl_seconds > 0 {
            Duration::seconds(i64::try_from(ttl_seconds).map_or(300, |v| v))
        } else {
            self.ttl
        };

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
        .map_err(|e| Error::Database(e.to_string()))?;

        if let Some((existing_lock_id, holder_agent_id, existing_expires_str)) = existing {
            if holder_agent_id == agent_id {
                let existing_expires = DateTime::parse_from_rfc3339(&existing_expires_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
                    .with_timezone(&Utc);
                return Ok(LockResponse {
                    lock_id: existing_lock_id,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    expires_at: existing_expires,
                });
            }
            return Err(Error::SessionLocked(session.to_string(), holder_agent_id));
        }

        self.verify_session_exists(session).await?;

        sqlx::query("DELETE FROM session_locks WHERE session = ? AND expires_at < ?")
            .bind(session)
            .bind(&now_str)
            .execute(&self.db)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let expires_at = now + ttl;
        let expires_str = expires_at.to_rfc3339();
        let nanos = now
            .timestamp_nanos_opt()
            .ok_or_else(|| Error::ValidationError("Failed to get timestamp nanos".into()))?;
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
                            Error::Database(format!(
                                "Failed to query lock holder after conflict: {db_err}"
                            ))
                        })?;

                let holder_agent_id = holder.map_or_else(|| "unknown".to_string(), |(id,)| id);
                return Err(Error::SessionLocked(session.to_string(), holder_agent_id));
            }

            return Err(Error::Database(format!(
                "Failed to acquire lock with TTL: {e}"
            )));
        }

        if let Err(log_error) = self.log_operation(session, agent_id, "lock").await {
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
            expires_at,
        })
    }

    /// Acquire an exclusive lock on a session.
    ///
    /// Returns `SessionLocked` error if another agent holds a valid lock.
    /// Returns `SessionNotFound` error if the session doesn't exist.
    pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
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
        .map_err(|e| Error::Database(e.to_string()))?;

        if let Some((existing_lock_id, holder_agent_id, existing_expires_str)) = existing {
            if holder_agent_id == agent_id {
                let existing_expires = DateTime::parse_from_rfc3339(&existing_expires_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
                    .with_timezone(&Utc);
                return Ok(LockResponse {
                    lock_id: existing_lock_id,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    expires_at: existing_expires,
                });
            }
            return Err(Error::SessionLocked(session.to_string(), holder_agent_id));
        }

        self.verify_session_exists(session).await?;

        sqlx::query("DELETE FROM session_locks WHERE session = ? AND expires_at < ?")
            .bind(session)
            .bind(&now_str)
            .execute(&self.db)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let expires_at = now + self.ttl;
        let expires_str = expires_at.to_rfc3339();
        let nanos = now
            .timestamp_nanos_opt()
            .ok_or_else(|| Error::ValidationError("Failed to get timestamp nanos".into()))?;
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

        match insert_result {
            Ok(_) => {
                if let Err(log_error) = self.log_operation(session, agent_id, "lock").await {
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
                    expires_at,
                })
            }
            Err(e) => {
                if is_constraint_conflict_error(&e) {
                    let holder: Option<(String,)> =
                        sqlx::query_as("SELECT agent_id FROM session_locks WHERE session = ?")
                            .bind(session)
                            .fetch_optional(&self.db)
                            .await
                            .map_err(|db_err| {
                                Error::Database(format!(
                                    "Failed to query lock holder after conflict: {db_err}"
                                ))
                            })?;

                    let holder_agent_id = holder.map_or_else(|| "unknown".to_string(), |(id,)| id);

                    Err(Error::SessionLocked(session.to_string(), holder_agent_id))
                } else {
                    Err(Error::Database(format!("Failed to acquire lock: {e}")))
                }
            }
        }
    }
}
