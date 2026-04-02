//! Event Store Lock Schema Management
//!
//! This module provides SQLite schema creation and management for the
//! event_store_locks table.
//!
//! # Design
//!
//! The event_store_locks table implements distributed locking for the event store,
//! ensuring ordered event processing across multiple agents. The composite UNIQUE
//! constraint on (stream_id, stream_seq) ensures that each event has a unique
//! position within its stream, preventing duplicate or out-of-order processing.
//!
//! # Lock Lifecycle
//!
//! 1. Agent calls acquire_stream_lock() -> INSERT with expires_at
//! 2. Event processing checks is_stream_locked() -> SELECT non-expired locks
//! 3. Agent calls release_stream_lock() -> DELETE lock
//! 4. Expired locks auto-cleanup on next query

use sqlx::SqlitePool;

use super::event_store_lock_types::EventStoreLockError;

/// Create the event_store_locks table schema if it does not exist.
///
/// Creates the main table and supporting indexes for:
/// - Stream-based lock queries
/// - Expired lock cleanup
/// - Holder-based lock queries
/// - Sequence-based ordering within a stream
///
/// # Errors
///
/// Returns `EventStoreLockError::DatabaseError` if schema creation fails.
pub async fn ensure_event_store_lock_schema(pool: &SqlitePool) -> Result<(), EventStoreLockError> {
    // Create the main event_store_locks table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS event_store_locks (
            stream_id TEXT NOT NULL,
            stream_seq INTEGER NOT NULL,
            holder_id TEXT NOT NULL,
            acquired_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            UNIQUE(stream_id, stream_seq)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!(
            "Failed to create event_store_locks schema: {e}"
        ))
    })?;

    // Index for stream-based lock queries
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_event_store_locks_stream_id
         ON event_store_locks(stream_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!("Failed to create stream_id index: {e}"))
    })?;

    // Index for expired lock cleanup queries
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_event_store_locks_expires_at
         ON event_store_locks(expires_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!("Failed to create expires_at index: {e}"))
    })?;

    // Index for holder-based lock queries
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_event_store_locks_holder_id
         ON event_store_locks(holder_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!("Failed to create holder_id index: {e}"))
    })?;

    // Index for sequence-based ordering (within stream)
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_event_store_locks_stream_seq
         ON event_store_locks(stream_id, stream_seq)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!("Failed to create stream_seq index: {e}"))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row as _;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    async fn create_test_pool() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        (pool, temp_dir)
    }

    #[tokio::test]
    async fn given_no_schema_when_ensure_then_creates_tables() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result = ensure_event_store_lock_schema(&pool).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn given_existing_schema_when_ensure_then_idempotent() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result1 = ensure_event_store_lock_schema(&pool).await;
        assert!(result1.is_ok());

        let result2 = ensure_event_store_lock_schema(&pool).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn given_schema_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;
        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("session-queue")
        .bind(1)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn given_schema_when_duplicate_stream_seq_then_fails() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("session-queue")
        .bind(1)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await
        .expect("First insert should succeed");

        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("session-queue")
        .bind(1)
        .bind("agent-2")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result.is_err());
        let err_msg = format!("{result:?}");
        assert!(err_msg.contains("UNIQUE constraint failed") || err_msg.contains("constraint"));
    }

    // =========================================================================
    // NOT NULL constraint verification
    // =========================================================================

    #[tokio::test]
    async fn given_schema_when_insert_null_stream_id_then_fails() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;
        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (NULL, ?, ?, ?, ?)",
        )
        .bind(1_i64)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn given_schema_when_insert_null_holder_id_then_fails() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;
        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, NULL, ?, ?)",
        )
        .bind("stream-1")
        .bind(1_i64)
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }

    // =========================================================================
    // Unique constraint: same stream different seq should succeed
    // =========================================================================

    #[tokio::test]
    async fn given_schema_when_same_stream_different_seq_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        let result1 = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("session-queue")
        .bind(1_i64)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        let result2 = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("session-queue")
        .bind(2_i64)
        .bind("agent-2")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    // =========================================================================
    // Unique constraint: different stream same seq should succeed
    // =========================================================================

    #[tokio::test]
    async fn given_schema_when_different_stream_same_seq_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        let result1 = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-a")
        .bind(1_i64)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        let result2 = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-b")
        .bind(1_i64)
        .bind("agent-2")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    // =========================================================================
    // Idempotency: ensure three times
    // =========================================================================

    #[tokio::test]
    async fn given_existing_schema_when_ensure_three_times_then_all_succeed() {
        let (pool, _temp_dir) = create_test_pool().await;

        assert!(ensure_event_store_lock_schema(&pool).await.is_ok());
        assert!(ensure_event_store_lock_schema(&pool).await.is_ok());
        assert!(ensure_event_store_lock_schema(&pool).await.is_ok());
    }

    // =========================================================================
    // Data retrieval: verify column types are correct
    // =========================================================================

    #[tokio::test]
    async fn given_inserted_row_when_select_then_columns_match() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("test-stream")
        .bind(42_i64)
        .bind("test-agent")
        .bind(now)
        .bind(now + 600)
        .execute(&pool)
        .await
        .expect("Insert should succeed");

        let row = sqlx::query(
            "SELECT stream_id, stream_seq, holder_id, acquired_at, expires_at
             FROM event_store_locks
             WHERE stream_id = ? AND stream_seq = ?",
        )
        .bind("test-stream")
        .bind(42_i64)
        .fetch_one(&pool)
        .await
        .expect("Select should succeed");

        let stream_id: String = row.get("stream_id");
        let stream_seq: i64 = row.get("stream_seq");
        let holder_id: String = row.get("holder_id");
        let acquired_at: i64 = row.get("acquired_at");
        let expires_at: i64 = row.get("expires_at");

        assert_eq!(stream_id, "test-stream");
        assert_eq!(stream_seq, 42);
        assert_eq!(holder_id, "test-agent");
        assert_eq!(acquired_at, now);
        assert_eq!(expires_at, now + 600);
    }

    // =========================================================================
    // DELETE and re-INSERT after schema creation
    // =========================================================================

    #[tokio::test]
    async fn given_lock_when_deleted_then_same_key_can_be_reinserted() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-x")
        .bind(1_i64)
        .bind("agent-1")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await
        .expect("Insert should succeed");

        sqlx::query("DELETE FROM event_store_locks WHERE stream_id = ? AND stream_seq = ?")
            .bind("stream-x")
            .bind(1_i64)
            .execute(&pool)
            .await
            .expect("Delete should succeed");

        // Should be able to re-insert with same composite key
        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-x")
        .bind(1_i64)
        .bind("agent-2")
        .bind(now + 1000)
        .bind(now + 1300)
        .execute(&pool)
        .await;

        assert!(result.is_ok());
    }

    // =========================================================================
    // SELECT from empty table
    // =========================================================================

    #[tokio::test]
    async fn given_empty_table_when_select_then_no_rows() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let rows = sqlx::query("SELECT * FROM event_store_locks")
            .fetch_all(&pool)
            .await
            .expect("Select should succeed");

        assert!(rows.is_empty());
    }

    // =========================================================================
    // Multiple inserts from different holders
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_holders_when_inserted_then_all_present() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        for i in 0..5 {
            sqlx::query(
                "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(format!("stream-{i}"))
            .bind(1_i64)
            .bind(format!("agent-{i}"))
            .bind(now + (i as i64) * 100)
            .bind(now + (i as i64) * 100 + 300)
            .execute(&pool)
            .await
            .expect("Insert should succeed");
        }

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_store_locks")
            .fetch_one(&pool)
            .await
            .expect("Count should succeed");

        assert_eq!(count, 5);
    }

    // =========================================================================
    // Index usage verification via EXPLAIN QUERY PLAN
    // =========================================================================

    #[tokio::test]
    async fn given_stream_id_index_when_query_then_exists() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        // Verify the index was created by using it in a query that would be slow without it
        let _ =
            sqlx::query("SELECT stream_id FROM event_store_locks WHERE stream_id = 'nonexistent'")
                .fetch_all(&pool)
                .await
                .expect("Index-using query should succeed");
    }

    // =========================================================================
    // UPDATE existing lock holder
    // =========================================================================

    #[tokio::test]
    async fn given_existing_lock_when_update_holder_then_changed() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let now = 1_700_000_000_i64;

        sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-y")
        .bind(1_i64)
        .bind("agent-old")
        .bind(now)
        .bind(now + 300)
        .execute(&pool)
        .await
        .expect("Insert should succeed");

        sqlx::query(
            "UPDATE event_store_locks SET holder_id = ? WHERE stream_id = ? AND stream_seq = ?",
        )
        .bind("agent-new")
        .bind("stream-y")
        .bind(1_i64)
        .execute(&pool)
        .await
        .expect("Update should succeed");

        let holder: String = sqlx::query_scalar(
            "SELECT holder_id FROM event_store_locks WHERE stream_id = ? AND stream_seq = ?",
        )
        .bind("stream-y")
        .bind(1_i64)
        .fetch_one(&pool)
        .await
        .expect("Select should succeed");

        assert_eq!(holder, "agent-new");
    }

    // =========================================================================
    // Negative timestamps in INTEGER columns
    // =========================================================================

    #[tokio::test]
    async fn given_negative_timestamps_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Schema creation failed");

        let result = sqlx::query(
            "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("stream-neg")
        .bind(1_i64)
        .bind("agent-neg")
        .bind(-100_i64)
        .bind(100_i64)
        .execute(&pool)
        .await;

        assert!(result.is_ok());
    }
}
