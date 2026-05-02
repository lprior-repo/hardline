//! SQLite-backed step journal for durable workflow execution.
//!
//! Persists [`StepRecord`] and [`OperationRecord`] entries to `SQLite` so that
//! multi-step operations survive process crashes. On restart the
//! [`RecoveryScanner`] queries the journal to find incomplete operations and
//! resume from the last completed step.
//!
//! ## Schema
//!
//! Two tables:
//! - `operation_journal` — one row per operation, tracks state and timestamps.
//! - `step_journal` — one row per step, ordered by `step_order`.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

use std::time::Duration;

use sqlx::SqlitePool;
#[cfg(test)]
use tempfile::NamedTempFile;

use crate::{
    domain::workflow::{
        records::{OperationRecord, RecoveryTask, StepRecord},
        states::{OperationState, StepStatus},
    },
    Error,
};

// ---------------------------------------------------------------------------
// SQL schema
// ---------------------------------------------------------------------------

const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS operation_journal (
    operation_id   TEXT PRIMARY KEY,
    status         TEXT    NOT NULL DEFAULT 'started',
    author_id      TEXT    NOT NULL DEFAULT '',
    description    TEXT    NOT NULL DEFAULT '',
    current_step   INTEGER NOT NULL DEFAULT 0,
    total_steps    INTEGER NOT NULL DEFAULT 0,
    started_at     INTEGER NOT NULL DEFAULT 0,
    completed_at   INTEGER,
    final_revision INTEGER,
    error_message  TEXT,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS step_journal (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id    TEXT    NOT NULL,
    step_name       TEXT    NOT NULL,
    step_order      INTEGER NOT NULL,
    status          TEXT    NOT NULL DEFAULT 'pending',
    event_revision  INTEGER,
    created_at      INTEGER NOT NULL DEFAULT 0,
    started_at      INTEGER,
    completed_at    INTEGER,
    error_message   TEXT,
    FOREIGN KEY (operation_id) REFERENCES operation_journal(operation_id)
        ON DELETE CASCADE
)";

// ---------------------------------------------------------------------------
// Row helpers (private, used for sqlx::query_as)
// ---------------------------------------------------------------------------

/// Result of a `SELECT MAX(...)` query where the aggregate may be NULL.
#[derive(Debug, sqlx::FromRow)]
struct MaxResult {
    max_step_order: Option<i64>,
}

/// Mirrors the columns returned when loading an operation from `SQLite`.
#[derive(Debug, sqlx::FromRow)]
struct OpRow {
    operation_id: String,
    status: String,
    author_id: String,
    description: String,
    current_step: i64,
    total_steps: i64,
    started_at: i64,
    completed_at: Option<i64>,
    final_revision: Option<i64>,
    error_message: Option<String>,
}

impl OpRow {
    fn into_record(self) -> Option<OperationRecord> {
        let state = self.status.parse::<OperationState>().ok()?;
        Some(OperationRecord {
            operation_id: self.operation_id,
            state,
            current_step: u32::try_from(self.current_step).ok()?,
            total_steps: u32::try_from(self.total_steps).ok()?,
            started_at: self.started_at,
            completed_at: self.completed_at,
            final_revision: self.final_revision,
            error_message: self.error_message,
            author_id: self.author_id,
            description: self.description,
        })
    }
}

/// Mirrors the columns returned when loading a step from `SQLite`.
#[derive(Debug, sqlx::FromRow)]
struct StepRow {
    operation_id: String,
    step_order: i64,
    step_name: String,
    status: String,
    event_revision: Option<i64>,
    created_at: i64,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    error_message: Option<String>,
}

impl StepRow {
    fn into_record(self) -> Option<StepRecord> {
        let status = self.status.parse::<StepStatus>().ok()?;
        Some(StepRecord {
            operation_id: self.operation_id,
            step_index: u32::try_from(self.step_order).ok()?,
            step_name: self.step_name,
            status,
            event_revision: self.event_revision,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            error_message: self.error_message,
        })
    }
}

// ---------------------------------------------------------------------------
// SqliteJournal
// ---------------------------------------------------------------------------

/// SQLite-backed step journal.
///
/// Call [`SqliteJournal::init`] once after creating the pool to ensure the
/// schema tables exist.
#[derive(Debug, Clone)]
pub struct SqliteJournal {
    pool: SqlitePool,
}

impl SqliteJournal {
    /// Create a new journal backed by the given connection pool.
    ///
    /// Call [`Self::init`] before first use to create the tables.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Ensure the journal tables exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the CREATE TABLE statements fail.
    pub async fn init(&self) -> Result<(), Error> {
        sqlx::query(SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::database(format!("Failed to create journal tables: {e}")))?;
        Ok(())
    }

    /// Persist a single step for an operation.
    ///
    /// If the operation row does not yet exist it is created with
    /// `OperationState::InProgress`.
    ///
    /// # Errors
    ///
    /// Returns an error if the INSERT fails.
    pub async fn save_step(&self, operation_id: &str, step: &StepRecord) -> Result<(), Error> {
        self.ensure_operation(operation_id).await?;

        sqlx::query(
            "INSERT INTO step_journal
                (operation_id, step_name, step_order, status, event_revision,
                 created_at, started_at, completed_at, error_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(operation_id)
        .bind(&step.step_name)
        .bind(step.step_index)
        .bind(step.status.as_str())
        .bind(step.event_revision)
        .bind(step.created_at)
        .bind(step.started_at)
        .bind(step.completed_at)
        .bind(&step.error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::database(format!(
                "Failed to save step '{}/{}': {e}",
                operation_id, step.step_name
            ))
        })?;

        self.touch_operation(operation_id).await
    }

    /// Load all steps for an operation, ordered by `step_order`.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn load_steps(&self, operation_id: &str) -> Result<Vec<StepRecord>, Error> {
        let rows: Vec<StepRow> = sqlx::query_as(
            "SELECT operation_id, step_order, step_name, status,
                    event_revision, created_at, started_at, completed_at,
                    error_message
             FROM step_journal
             WHERE operation_id = ?
             ORDER BY step_order ASC",
        )
        .bind(operation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::database(format!("Failed to load steps for '{operation_id}': {e}")))?;

        Ok(rows.into_iter().filter_map(StepRow::into_record).collect())
    }

    /// Load all operations that are not in a terminal state.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn load_pending_operations(&self) -> Result<Vec<OperationRecord>, Error> {
        let rows: Vec<OpRow> = sqlx::query_as(
            "SELECT operation_id, status, author_id, description,
                    current_step, total_steps, started_at, completed_at,
                    final_revision, error_message
             FROM operation_journal
             WHERE status NOT IN ('completed', 'failed')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::database(format!("Failed to load pending operations: {e}")))?;

        Ok(rows.into_iter().filter_map(OpRow::into_record).collect())
    }

    /// Mark an operation as completed and record its final state.
    ///
    /// # Errors
    ///
    /// Returns an error if the UPDATE fails.
    pub async fn mark_operation_completed(&self, operation_id: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE operation_journal
             SET status = 'completed',
                 completed_at = strftime('%s', 'now'),
                 updated_at = datetime('now')
             WHERE operation_id = ?",
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::database(format!(
                "Failed to mark operation '{operation_id}' completed: {e}"
            ))
        })?;
        Ok(())
    }

    /// Mark an operation as failed with an error message.
    ///
    /// # Errors
    ///
    /// Returns an error if the UPDATE fails.
    pub async fn mark_operation_failed(
        &self,
        operation_id: &str,
        error: &str,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE operation_journal
             SET status = 'failed',
                 error_message = ?,
                 completed_at = strftime('%s', 'now'),
                 updated_at = datetime('now')
             WHERE operation_id = ?",
        )
        .bind(error)
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::database(format!(
                "Failed to mark operation '{operation_id}' failed: {e}"
            ))
        })?;
        Ok(())
    }

    /// Load recovery tasks for all incomplete operations.
    ///
    /// For each pending operation, determines the step to resume from by
    /// inspecting the step journal. If no steps exist the resume index is 0.
    ///
    /// # Errors
    ///
    /// Returns an error if the queries fail.
    pub async fn recovery_tasks(&self) -> Result<Vec<RecoveryTask>, Error> {
        let operations = self.load_pending_operations().await?;
        let mut tasks = Vec::with_capacity(operations.len());

        for op in operations {
            let resume_from = self.last_completed_step_index(&op.operation_id).await?;
            tasks.push(RecoveryTask::new(op.operation_id, resume_from));
        }

        Ok(tasks)
    }

    /// Delete completed operations (and their steps) older than `older_than`.
    ///
    /// Returns the number of operations deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the DELETE fails.
    pub async fn cleanup_old_operations(&self, older_than: Duration) -> Result<u64, Error> {
        let cutoff_secs = i64::try_from(older_than.as_secs())
            .map_err(|_| Error::database("Duration too large for i64".to_string()))?;

        let result = sqlx::query(
            "DELETE FROM operation_journal
             WHERE status IN ('completed', 'failed')
               AND completed_at IS NOT NULL
               AND (strftime('%s', 'now') - completed_at) > ?",
        )
        .bind(cutoff_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::database(format!("Failed to cleanup old operations: {e}")))?;

        Ok(result.rows_affected())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Insert the operation row if it does not already exist.
    async fn ensure_operation(&self, operation_id: &str) -> Result<(), Error> {
        sqlx::query(
            "INSERT OR IGNORE INTO operation_journal
                (operation_id, status, created_at, updated_at)
             VALUES (?, 'in_progress', datetime('now'), datetime('now'))",
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::database(format!("Failed to ensure operation '{operation_id}': {e}"))
        })?;
        Ok(())
    }

    /// Update the `updated_at` timestamp for an operation.
    async fn touch_operation(&self, operation_id: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE operation_journal SET updated_at = datetime('now')
             WHERE operation_id = ?",
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::database(format!("Failed to touch operation '{operation_id}': {e}")))?;
        Ok(())
    }

    /// Find the index of the last completed step, or 0 if none.
    async fn last_completed_step_index(&self, operation_id: &str) -> Result<u32, Error> {
        let row: MaxResult = sqlx::query_as(
            "SELECT MAX(step_order) AS max_step_order FROM step_journal
             WHERE operation_id = ? AND status = 'completed'",
        )
        .bind(operation_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            Error::database(format!(
                "Failed to query last completed step for '{operation_id}': {e}"
            ))
        })?;

        let last = row.max_step_order.and_then(|idx| u32::try_from(idx).ok());

        // Resume from the step *after* the last completed one.
        Ok(last.map_or(0, |idx| idx.saturating_add(1)))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workflow::states::StepStatus;

    /// Create a temporary SQLite pool with the journal schema.
    ///
    /// Each call creates a fresh temporary file so that parallel tokio tests
    /// are fully isolated.
    async fn test_pool() -> SqlitePool {
        let tmp = NamedTempFile::new().expect("create temp file");
        let db_url = format!("sqlite:{}", tmp.path().display());
        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("connect to temp db");
        let journal = SqliteJournal::new(pool.clone());
        journal.init().await.expect("init journal tables");
        // Keep the temp file alive for the pool's lifetime.
        std::mem::forget(tmp);
        pool
    }

    fn make_step(operation_id: &str, index: u32, name: &str) -> StepRecord {
        StepRecord::new(operation_id.to_string(), index, name.to_string())
    }

    fn make_completed_step(operation_id: &str, index: u32, name: &str) -> StepRecord {
        let mut step = make_step(operation_id, index, name);
        step.start();
        step.complete();
        step
    }

    // -----------------------------------------------------------------------
    // init
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn init_creates_tables() {
        let pool = test_pool().await;

        // Verify tables exist by querying them.
        let result: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table'")
                .fetch_all(&pool)
                .await
                .expect("query sqlite_master");

        let names: Vec<&str> = result.iter().map(|(n,)| n.as_str()).collect();
        assert!(names.contains(&"operation_journal"));
        assert!(names.contains(&"step_journal"));
    }

    // -----------------------------------------------------------------------
    // save_step / load_steps
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn save_and_load_steps() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-save-load";

        let step1 = make_completed_step(op_id, 0, "create-db");
        let step2 = make_completed_step(op_id, 1, "create-dir");

        journal.save_step(op_id, &step1).await.expect("save step1");
        journal.save_step(op_id, &step2).await.expect("save step2");

        let loaded = journal.load_steps(op_id).await.expect("load steps");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].step_index, 0);
        assert_eq!(loaded[0].step_name, "create-db");
        assert_eq!(loaded[0].status, StepStatus::Completed);
        assert_eq!(loaded[1].step_index, 1);
        assert_eq!(loaded[1].step_name, "create-dir");
    }

    #[tokio::test]
    async fn load_steps_empty_for_unknown_operation() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let loaded = journal
            .load_steps("nonexistent-op")
            .await
            .expect("load steps");
        assert!(loaded.is_empty());
    }

    // -----------------------------------------------------------------------
    // load_pending_operations
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn load_pending_returns_in_progress() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-pending";

        let step = make_step(op_id, 0, "step-a");
        journal.save_step(op_id, &step).await.expect("save");

        let pending = journal
            .load_pending_operations()
            .await
            .expect("load pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].operation_id, op_id);
        assert_eq!(pending[0].state, OperationState::InProgress);
    }

    #[tokio::test]
    async fn load_pending_excludes_completed() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-completed-filter";
        let step = make_step(op_id, 0, "step-a");
        journal.save_step(op_id, &step).await.expect("save");
        journal
            .mark_operation_completed(op_id)
            .await
            .expect("complete");

        let pending = journal
            .load_pending_operations()
            .await
            .expect("load pending");
        assert!(pending.is_empty());
    }

    // -----------------------------------------------------------------------
    // mark_operation_completed / mark_operation_failed
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn mark_completed_updates_state() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-mark-completed";
        let step = make_step(op_id, 0, "step-a");
        journal.save_step(op_id, &step).await.expect("save");

        journal
            .mark_operation_completed(op_id)
            .await
            .expect("complete");

        let pending = journal
            .load_pending_operations()
            .await
            .expect("load pending");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn mark_failed_updates_state() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool.clone());

        let op_id = "op-mark-failed";
        let step = make_step(op_id, 0, "step-a");
        journal.save_step(op_id, &step).await.expect("save");

        journal
            .mark_operation_failed(op_id, "disk full")
            .await
            .expect("fail");

        let pending = journal
            .load_pending_operations()
            .await
            .expect("load pending");
        assert!(pending.is_empty());

        // Load via raw query to verify error message persisted.
        let row: Option<(String,)> =
            sqlx::query_as("SELECT error_message FROM operation_journal WHERE operation_id = ?")
                .bind(op_id)
                .fetch_optional(&pool)
                .await
                .expect("query error_message");
        assert_eq!(row, Some(("disk full".to_string(),)));
    }

    // -----------------------------------------------------------------------
    // recovery_tasks
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn recovery_tasks_for_incomplete_operations() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        // op-1: two steps completed, third pending.
        let op1 = "op-recovery-1";
        journal
            .save_step(op1, &make_completed_step(op1, 0, "step-0"))
            .await
            .expect("save");
        journal
            .save_step(op1, &make_completed_step(op1, 1, "step-1"))
            .await
            .expect("save");
        journal
            .save_step(op1, &make_step(op1, 2, "step-2"))
            .await
            .expect("save");

        // op-2: no steps at all.
        let op2 = "op-recovery-2";
        journal
            .save_step(op2, &make_step(op2, 0, "step-0"))
            .await
            .expect("save");

        let tasks = journal.recovery_tasks().await.expect("recovery tasks");

        assert_eq!(tasks.len(), 2);

        let t1 = tasks
            .iter()
            .find(|t| t.operation_id == op1)
            .expect("find op1");
        assert_eq!(t1.resume_from_step, 2); // after step-1 (index 1)

        let t2 = tasks
            .iter()
            .find(|t| t.operation_id == op2)
            .expect("find op2");
        assert_eq!(t2.resume_from_step, 0); // no completed steps
    }

    #[tokio::test]
    async fn recovery_tasks_excludes_completed_operations() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-done-recovery";
        let step = make_completed_step(op_id, 0, "step-0");
        journal.save_step(op_id, &step).await.expect("save");
        journal
            .mark_operation_completed(op_id)
            .await
            .expect("complete");

        let tasks = journal.recovery_tasks().await.expect("recovery tasks");
        assert!(tasks.is_empty());
    }

    // -----------------------------------------------------------------------
    // cleanup_old_operations
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn cleanup_removes_old_completed_operations() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool.clone());

        let op_id = "op-cleanup";
        let step = make_completed_step(op_id, 0, "step-0");
        journal.save_step(op_id, &step).await.expect("save");
        journal
            .mark_operation_completed(op_id)
            .await
            .expect("complete");

        // Manually backdate the completed_at so cleanup will pick it up.
        let _ = sqlx::query(
            "UPDATE operation_journal SET completed_at = strftime('%s','now') - 999999
             WHERE operation_id = ?",
        )
        .bind(op_id)
        .execute(&pool)
        .await;

        let deleted = journal
            .cleanup_old_operations(Duration::from_secs(0))
            .await
            .expect("cleanup");
        assert_eq!(deleted, 1);

        // Verify steps are also gone (ON DELETE CASCADE).
        let steps = journal.load_steps(op_id).await.expect("load steps");
        assert!(steps.is_empty());
    }

    #[tokio::test]
    async fn cleanup_preserves_recent_operations() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-recent-cleanup";
        let step = make_completed_step(op_id, 0, "step-0");
        journal.save_step(op_id, &step).await.expect("save");
        journal
            .mark_operation_completed(op_id)
            .await
            .expect("complete");

        // Use a very long duration so the just-completed op is preserved.
        let deleted = journal
            .cleanup_old_operations(Duration::from_secs(999_999))
            .await
            .expect("cleanup");
        assert_eq!(deleted, 0);
    }

    // -----------------------------------------------------------------------
    // Step status roundtrip
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn step_status_roundtrips_through_sqlite() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        let op_id = "op-status-rt";

        let statuses = [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ];

        for (i, status) in statuses.iter().enumerate() {
            let mut step = make_step(op_id, u32::try_from(i).expect("index"), "step");
            match status {
                StepStatus::Pending => {}
                StepStatus::Running => step.start(),
                StepStatus::Completed => {
                    step.start();
                    step.complete();
                }
                StepStatus::Failed => {
                    step.start();
                    step.fail("err".to_string());
                }
                StepStatus::Skipped => step.skip(),
            }
            journal.save_step(op_id, &step).await.expect("save");
        }

        let loaded = journal.load_steps(op_id).await.expect("load");
        assert_eq!(loaded.len(), 5);
        for (i, expected) in statuses.iter().enumerate() {
            assert_eq!(loaded[i].status, *expected, "status mismatch at index {i}");
        }
    }

    // -----------------------------------------------------------------------
    // Idempotent init
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn init_is_idempotent() {
        let pool = test_pool().await;
        let journal = SqliteJournal::new(pool);

        // Second init should not fail.
        journal.init().await.expect("second init");
    }
}
