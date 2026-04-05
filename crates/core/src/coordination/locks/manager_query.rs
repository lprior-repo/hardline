//! Lock query operations.

use chrono::{DateTime, Utc};

use super::manager::LockManager;
use super::types::{LockAuditEntry, LockInfo, LockOperation, LockState};
use crate::Result;

impl LockManager {
    /// Get all active (non-expired) locks across all sessions.
    ///
    /// Returns locks sorted by expires_at ASC, then by lock_id for ties.
    /// Never returns expired locks.
    pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>> {
        let now_str = Utc::now().to_rfc3339();

        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT lock_id, session, agent_id, acquired_at, expires_at FROM session_locks WHERE expires_at >= ? ORDER BY expires_at ASC, lock_id ASC",
        )
        .bind(&now_str)
        .fetch_all(&self.db)
        .await
        .map_err(|e| crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string())))?;

        rows.into_iter()
            .map(|(lock_id, session, agent_id, acquired_str, expires_str)| {
                let acquired_at = DateTime::parse_from_rfc3339(&acquired_str)
                    .map_err(|e| {
                        crate::error::Error::from(super::errors::LockErrorKind::ParseError(
                            e.to_string(),
                        ))
                    })?
                    .with_timezone(&Utc);
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| {
                        crate::error::Error::from(super::errors::LockErrorKind::ParseError(
                            e.to_string(),
                        ))
                    })?
                    .with_timezone(&Utc);
                Ok(LockInfo {
                    lock_id,
                    session,
                    agent_id,
                    acquired_at,
                    expires_at,
                })
            })
            .collect()
    }

    /// Get audit log entries for a specific session.
    ///
    /// Returns entries ordered by timestamp ASC.
    /// Returns empty Vec if no audit history for session.
    pub async fn get_lock_audit_log(&self, session: &str) -> Result<Vec<LockAuditEntry>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT session, agent_id, operation, timestamp
             FROM session_lock_audit
             WHERE session = ?
             ORDER BY id ASC",
        )
        .bind(session)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

        rows.into_iter()
            .map(|(session, agent_id, operation_str, timestamp_str)| {
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| {
                        crate::error::Error::from(super::errors::LockErrorKind::ParseError(
                            e.to_string(),
                        ))
                    })?
                    .with_timezone(&Utc);

                let operation = match operation_str.as_str() {
                    "lock" => LockOperation::Lock,
                    "unlock" => LockOperation::Unlock,
                    "heartbeat" => LockOperation::Heartbeat,
                    "double_unlock_warning" => LockOperation::DoubleUnlockWarning,
                    _ => {
                        return Err(crate::error::Error::from(
                            super::errors::LockErrorKind::Unknown(format!(
                                "Unknown operation: {operation_str}"
                            )),
                        ));
                    }
                };

                Ok(LockAuditEntry {
                    session,
                    agent_id,
                    operation,
                    timestamp,
                })
            })
            .collect()
    }

    /// Get current lock state for a session.
    ///
    /// Returns LockState with holder=None and expires_at=None if no active lock.
    pub async fn get_lock_state(&self, session: &str) -> Result<LockState> {
        let now_str = Utc::now().to_rfc3339();

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT agent_id, expires_at FROM session_locks
             WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(e.to_string()))
        })?;

        match existing {
            Some((holder, expires_str)) => {
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| {
                        crate::error::Error::from(super::errors::LockErrorKind::ParseError(
                            e.to_string(),
                        ))
                    })?
                    .with_timezone(&Utc);
                Ok(LockState {
                    session: session.to_string(),
                    holder: Some(holder),
                    expires_at: Some(expires_at),
                })
            }
            None => Ok(LockState {
                session: session.to_string(),
                holder: None,
                expires_at: None,
            }),
        }
    }

    /// Remove all expired locks from the database.
    ///
    /// Returns the number of locks that were cleaned up.
    /// This is used by the doctor command for periodic maintenance.
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let now_str = Utc::now().to_rfc3339();

        let result = sqlx::query("DELETE FROM session_locks WHERE expires_at < ?")
            .bind(&now_str)
            .execute(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::from(super::errors::LockErrorKind::DatabaseError(
                    e.to_string(),
                ))
            })?;

        Ok(result.rows_affected())
    }
}
