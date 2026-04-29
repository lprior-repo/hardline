//! Lock release and heartbeat operations.

use chrono::Utc;

use super::{
    manager::LockManager,
    types::{LockOperation, LockResponse},
};
use crate::Result;

/// Map a sqlx transaction error to our error type.
fn db_err(e: sqlx::Error) -> crate::error::Error {
    crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
}

/// Error for when an agent is not the lock holder.
fn not_holder_error(session: &str, agent_id: &str) -> crate::error::Error {
    crate::error::Error::from(super::errors::LockErrorKind::NotLockHolder {
        session: session.to_string(),
        agent_id: agent_id.to_string(),
    })
}

/// Error for when no active lock exists.
fn not_found_error(session: &str) -> crate::error::Error {
    crate::error::Error::from(super::errors::LockErrorKind::NotFound(format!(
        "No active lock for session {session}"
    )))
}

impl LockManager {
    /// Release a lock held by the caller.
    ///
    /// Returns `NotLockHolder` error if agent does not hold the lock.
    /// Returns `Ok(())` for double-unlock with audit warning.
    pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        let mut tx = self.db.begin().await.map_err(db_err)?;

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query("DELETE FROM session_locks WHERE session = ? AND agent_id = ?")
                    .bind(session)
                    .bind(agent_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;

                sqlx::query("INSERT INTO session_lock_audit (session, agent_id, operation, timestamp) VALUES (?, ?, ?, ?)")
                    .bind(session).bind(agent_id).bind(LockOperation::Unlock.as_str()).bind(&now_str)
                    .execute(&mut *tx).await.map_err(db_err)?;

                tx.commit().await.map_err(db_err)?;
                Ok(())
            }
            Some(_) => Err(not_holder_error(session, agent_id)),
            None => {
                sqlx::query("INSERT INTO session_lock_audit (session, agent_id, operation, timestamp) VALUES (?, ?, ?, ?)")
                    .bind(session).bind(agent_id).bind(LockOperation::DoubleUnlockWarning.as_str()).bind(&now_str)
                    .execute(&mut *tx).await.map_err(db_err)?;

                tx.commit().await.map_err(db_err)?;
                Ok(())
            }
        }
    }

    /// Extend lock TTL via heartbeat (must be lock holder).
    ///
    /// Returns `NotFound` error if no active lock exists (lock missing or expired).
    pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let new_expires = now + self.ttl;
        let new_expires_str = new_expires.to_rfc3339();

        let mut tx = self.db.begin().await.map_err(db_err)?;

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT lock_id, agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;

        match existing {
            Some((lock_id, holder)) if holder == agent_id => {
                sqlx::query(
                    "UPDATE session_locks SET expires_at = ? WHERE session = ? AND agent_id = ?",
                )
                .bind(&new_expires_str)
                .bind(session)
                .bind(agent_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;

                sqlx::query("INSERT INTO session_lock_audit (session, agent_id, operation, timestamp) VALUES (?, ?, ?, ?)")
                    .bind(session).bind(agent_id).bind(LockOperation::Heartbeat.as_str()).bind(&now_str)
                    .execute(&mut *tx).await.map_err(db_err)?;

                tx.commit().await.map_err(db_err)?;

                Ok(LockResponse {
                    lock_id,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    acquired_at: now,
                    expires_at: new_expires,
                })
            }
            Some(_) => Err(not_holder_error(session, agent_id)),
            None => Err(not_found_error(session)),
        }
    }
}
