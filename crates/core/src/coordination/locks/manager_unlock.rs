//! Lock release and heartbeat operations.

use chrono::Utc;

use super::manager::LockManager;
use super::types::{LockOperation, LockResponse};
use crate::Result;

impl LockManager {
    /// Release a lock held by the caller.
    ///
    /// Returns `NotLockHolder` error if agent does not hold the lock.
    /// Returns `Ok(())` for double-unlock with audit warning.
    pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query("DELETE FROM session_locks WHERE session = ? AND agent_id = ?")
                    .bind(session)
                    .bind(agent_id)
                    .execute(&self.db)
                    .await
                    .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

                self.log_operation(session, agent_id, LockOperation::Unlock).await?;
                Ok(())
            }
            Some(_) => Err(crate::error::Error::from(super::errors::LockErrorKind::NotLockHolder {
                session: session.to_string(),
                agent_id: agent_id.to_string(),
            })),
            None => {
                self.log_operation(session, agent_id, LockOperation::DoubleUnlockWarning)
                    .await?;
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

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT lock_id, agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

        match existing {
            Some((lock_id, holder)) if holder == agent_id => {
                sqlx::query(
                    "UPDATE session_locks SET expires_at = ? WHERE session = ? AND agent_id = ?",
                )
                .bind(&new_expires_str)
                .bind(session)
                .bind(agent_id)
                .execute(&self.db)
                .await
                .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

                self.log_operation(session, agent_id, LockOperation::Heartbeat).await?;

                Ok(LockResponse {
                    lock_id,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    acquired_at: now,
                    expires_at: new_expires,
                })
            }
            Some(_) => Err(crate::error::Error::from(super::errors::LockErrorKind::NotLockHolder {
                session: session.to_string(),
                agent_id: agent_id.to_string(),
            })),
            None => Err(crate::error::Error::from(super::errors::LockErrorKind::NotFound(format!(
                "No active lock for session '{session}'"
            )))),
        }
    }
}
