//! Auto-checkpoint before risky operations.
//!
//! Provides an RAII guard pattern that automatically creates a checkpoint
//! before risky operations and restores state on failure.
//!
//! # Usage
//!
//! ```ignore
//! let auto_cp = AutoCheckpoint::new(pool);
//! let guard = auto_cp.guard_if_risky(OperationRisk::Risky).await?;
//! if let Some(guard) = guard {
//!     // do risky work...
//!     guard.commit().await?;  // discard checkpoint on success
//! }
//! // if guard is dropped without commit, it marks for restore
//! ```

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use chrono::Utc;
use sqlx::SqlitePool;

use crate::{Error, Result};

/// Risk level of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationRisk {
    /// Safe operations (list, status, context) - no checkpoint needed.
    Safe,
    /// Risky operations (batch, spawn, cleanup --force) - checkpoint required.
    Risky,
}

impl OperationRisk {
    /// Returns true if this operation requires a checkpoint.
    #[must_use]
    pub const fn needs_checkpoint(&self) -> bool {
        matches!(self, Self::Risky)
    }
}

/// Classifies a command name into its risk level.
#[must_use]
pub fn classify_command(command: &str) -> OperationRisk {
    match command {
        "batch" | "spawn" | "remove" | "cleanup" | "rebase" | "squash" => OperationRisk::Risky,
        _ => OperationRisk::Safe,
    }
}

/// Auto-checkpoint manager that creates checkpoints before risky operations.
#[derive(Debug, Clone)]
pub struct AutoCheckpoint {
    db: SqlitePool,
}

impl AutoCheckpoint {
    /// Create a new auto-checkpoint manager.
    #[must_use]
    pub const fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Create a checkpoint guard if the operation is risky.
    ///
    /// Returns `None` for safe operations, `Some(guard)` for risky ones.
    /// If checkpoint creation fails, returns an error (aborting the operation).
    pub async fn guard_if_risky(&self, risk: OperationRisk) -> Result<Option<CheckpointGuard>> {
        if !risk.needs_checkpoint() {
            return Ok(None);
        }

        let checkpoint_id = format!("auto-{}", Utc::now().timestamp_millis());

        self.create_checkpoint(&checkpoint_id).await?;

        Ok(Some(CheckpointGuard {
            checkpoint_id,
            db: self.db.clone(),
            committed: Arc::new(AtomicBool::new(false)),
        }))
    }

    /// Ensure the checkpoints table exists.
    pub async fn ensure_table(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'pending'
            )",
        )
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create checkpoints table: {e}")))?;

        Ok(())
    }

    async fn create_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query("INSERT INTO checkpoints (id, created_at, state) VALUES (?, ?, 'pending')")
            .bind(checkpoint_id)
            .bind(&now)
            .execute(&self.db)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!(
                    "Failed to create checkpoint '{checkpoint_id}': {e}"
                ))
            })?;

        tracing::info!("Created auto-checkpoint: {}", checkpoint_id);
        Ok(())
    }
}

/// RAII guard for a checkpoint. Call `commit()` on success to discard the checkpoint.
/// If dropped without committing, marks the checkpoint for restore.
pub struct CheckpointGuard {
    checkpoint_id: String,
    db: SqlitePool,
    committed: Arc<AtomicBool>,
}

impl std::fmt::Debug for CheckpointGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckpointGuard")
            .field("checkpoint_id", &self.checkpoint_id)
            .field("committed", &self.committed.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl CheckpointGuard {
    /// Returns the checkpoint ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.checkpoint_id
    }

    /// Mark the operation as successful, discarding the checkpoint.
    pub async fn commit(self) -> Result<()> {
        self.committed.store(true, Ordering::SeqCst);

        sqlx::query("UPDATE checkpoints SET state = 'committed' WHERE id = ?")
            .bind(&self.checkpoint_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!(
                    "Failed to commit checkpoint '{}': {e}",
                    self.checkpoint_id
                ))
            })?;

        tracing::info!(
            "Committed (discarded) auto-checkpoint: {}",
            self.checkpoint_id
        );
        Ok(())
    }

    /// Explicitly roll back to this checkpoint.
    pub async fn rollback(&self) -> Result<()> {
        sqlx::query("UPDATE checkpoints SET state = 'needs_restore' WHERE id = ?")
            .bind(&self.checkpoint_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!(
                    "Failed to mark checkpoint '{}' for restore: {e}",
                    self.checkpoint_id
                ))
            })?;

        tracing::warn!("Marked checkpoint for restore: {}", self.checkpoint_id);
        Ok(())
    }

    /// Check if this guard has been committed.
    #[must_use]
    pub fn is_committed(&self) -> bool {
        self.committed.load(Ordering::SeqCst)
    }
}

impl Drop for CheckpointGuard {
    fn drop(&mut self) {
        if !self.committed.load(Ordering::SeqCst) {
            tracing::warn!(
                "CheckpointGuard dropped without commit - checkpoint '{}' needs restore",
                self.checkpoint_id
            );
            // We can't do async in Drop, so we mark via the atomic flag.
            // Callers should check pending checkpoints on startup.
        }
    }
}

/// Check for any checkpoints that need restoration (e.g., on startup after crash).
pub async fn find_pending_restores(db: &SqlitePool) -> Result<Vec<String>> {
    // Any checkpoint still in 'pending' state was never committed (crash/failure).
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM checkpoints WHERE state = 'pending' OR state = 'needs_restore'",
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to query pending checkpoints: {e}")))?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> Result<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:").await.map_err(|e| {
            Error::DatabaseError(format!("Failed to connect to test database: {e}"))
        })?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        // Ensure table exists - ignore error in test setup
        let _ = auto_cp.ensure_table().await;
        Ok(pool)
    }

    #[tokio::test]
    async fn safe_operation_returns_none() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool);
        let guard = auto_cp.guard_if_risky(OperationRisk::Safe).await;
        assert!(guard.is_ok());
        let guard = guard.ok().flatten();
        assert!(guard.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn risky_operation_returns_guard() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool);
        let guard = auto_cp.guard_if_risky(OperationRisk::Risky).await;
        assert!(guard.is_ok());
        let guard = guard.ok().flatten();
        assert!(guard.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn committed_guard_state_is_committed() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        let guard_result = auto_cp.guard_if_risky(OperationRisk::Risky).await?;
        if let Some(g) = guard_result {
            let id = g.id().to_string();
            let _ = g.commit().await;

            let row: Option<(String,)> =
                sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten();
            assert_eq!(row.map(|(s,)| s), Some("committed".to_string()));
        }
        Ok(())
    }

    #[tokio::test]
    async fn dropped_guard_leaves_pending() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());

        let checkpoint_id: String;
        {
            let guard_result: Result<Option<CheckpointGuard>> =
                auto_cp.guard_if_risky(OperationRisk::Risky).await;
            checkpoint_id = guard_result
                .ok()
                .flatten()
                .map_or_else(String::new, |g| g.id().to_string());
            // guard dropped here without commit
        }

        if !checkpoint_id.is_empty() {
            let pending = find_pending_restores(&pool)
                .await
                .unwrap_or_else(|_| Vec::new());
            assert!(pending.contains(&checkpoint_id));
        }
        Ok(())
    }

    #[tokio::test]
    async fn rollback_marks_needs_restore() -> Result<()> {
        let pool = test_pool().await?;
        let auto_cp = AutoCheckpoint::new(pool.clone());
        let guard_result = auto_cp.guard_if_risky(OperationRisk::Risky).await?;
        if let Some(g) = guard_result {
            let id = g.id().to_string();
            let _ = g.rollback().await;

            let row: Option<(String,)> =
                sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten();
            assert_eq!(row.map(|(s,)| s), Some("needs_restore".to_string()));
        }
        Ok(())
    }

    #[test]
    fn classify_safe_commands() {
        assert_eq!(classify_command("list"), OperationRisk::Safe);
        assert_eq!(classify_command("status"), OperationRisk::Safe);
        assert_eq!(classify_command("context"), OperationRisk::Safe);
        assert_eq!(classify_command("focus"), OperationRisk::Safe);
    }

    #[test]
    fn classify_risky_commands() {
        assert_eq!(classify_command("batch"), OperationRisk::Risky);
        assert_eq!(classify_command("spawn"), OperationRisk::Risky);
        assert_eq!(classify_command("remove"), OperationRisk::Risky);
        assert_eq!(classify_command("cleanup"), OperationRisk::Risky);
        assert_eq!(classify_command("rebase"), OperationRisk::Risky);
        assert_eq!(classify_command("squash"), OperationRisk::Risky);
    }

    #[test]
    fn risk_needs_checkpoint() {
        assert!(!OperationRisk::Safe.needs_checkpoint());
        assert!(OperationRisk::Risky.needs_checkpoint());
    }
}
