//! Lock release and heartbeat operations.

use chrono::Utc;

use crate::{Error, Result};

use super::types::LockResponse;
use super::manager::LockManager;

impl LockManager {
    /// Release a lock. Only the holder can release it.
    pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query("DELETE FROM session_locks WHERE session = ? AND agent_id = ?")
                    .bind(session)
                    .bind(agent_id)
                    .execute(&self.db)
                    .await
                    .map_err(|e| Error::Database(e.to_string()))?;

                self.log_operation(session, agent_id, "unlock").await?;
                Ok(())
            }
            Some(_) => Err(Error::NotLockHolder(
                session.to_string(),
                agent_id.to_string(),
            )),
            None => {
                self.log_operation(session, agent_id, "double_unlock_warning")
                    .await?;
                Ok(())
            }
        }
    }

    /// Extend a lock's TTL (heartbeat).
    pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let new_expires = now + self.ttl;
        let new_expires_str = new_expires.to_rfc3339();

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query(
                    "UPDATE session_locks SET expires_at = ? WHERE session = ? AND agent_id = ?",
                )
                .bind(&new_expires_str)
                .bind(session)
                .bind(agent_id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                let row: (String,) =
                    sqlx::query_as("SELECT lock_id FROM session_locks WHERE session = ?")
                        .bind(session)
                        .fetch_one(&self.db)
                        .await
                        .map_err(|e| Error::Database(e.to_string()))?;

                Ok(LockResponse {
                    lock_id: row.0,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    expires_at: new_expires,
                })
            }
            Some(_) => Err(Error::NotLockHolder(
                session.to_string(),
                agent_id.to_string(),
            )),
            None => Err(Error::NotFound(format!(
                "No active lock for session '{session}'"
            ))),
        }
    }
}
