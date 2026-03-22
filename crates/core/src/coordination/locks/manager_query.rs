//! Lock query operations.

use chrono::{DateTime, Utc};

use crate::{Error, Result};

use super::types::{LockAuditEntry, LockInfo, LockState};
use super::manager::LockManager;

impl LockManager {
    /// Get all active (non-expired) locks.
    pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>> {
        let now_str = Utc::now().to_rfc3339();

        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT session, agent_id, acquired_at, expires_at FROM session_locks WHERE expires_at >= ?",
        )
        .bind(&now_str)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        rows.into_iter()
            .map(|(session, agent_id, acquired_str, expires_str)| {
                let acquired_at = DateTime::parse_from_rfc3339(&acquired_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
                    .with_timezone(&Utc);
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(LockInfo {
                    session,
                    agent_id,
                    acquired_at,
                    expires_at,
                })
            })
            .collect()
    }

    /// Get audit log for a session.
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
        .map_err(|e| Error::Database(e.to_string()))?;

        rows.into_iter()
            .map(|(session, agent_id, operation, timestamp_str)| {
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
                    .with_timezone(&Utc);
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
        .map_err(|e| Error::Database(e.to_string()))?;

        match existing {
            Some((holder, expires_str)) => {
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| Error::ValidationError(e.to_string()))?
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
}
