//! Event Store Lock Repository
//!
//! This module provides database operations for the event_store_locks table,
//! implementing distributed locking for ordered event stream processing.
//!
//! # Operations
//!
//! - `acquire_stream_lock` - Insert a new lock for a stream position
//! - `release_stream_lock` - Remove a lock by stream_id, stream_seq, and holder_id
//! - `is_stream_locked` - Check if a stream position has an active (non-expired) lock
//! - `get_stream_locks` - Get all active locks for a stream
//! - `get_next_sequence` - Get the next available sequence number for a stream
//! - `cleanup_expired_stream_locks` - Remove all expired locks
//! - `locks_by_holder` - Get all locks held by a specific agent

use sqlx::Row;
use sqlx::SqlitePool;

use super::event_store_lock_schema::ensure_event_store_lock_schema;
use super::event_store_lock_types::{
    parse_event_store_lock_row, EventStoreLock, EventStoreLockError,
};

/// Acquire a stream lock for a specific position in the event stream.
///
/// Inserts a new lock entry with the given parameters. The UNIQUE constraint
/// on (stream_id, stream_seq) prevents duplicate sequence assignments.
///
/// # Errors
///
/// Returns `EventStoreLockError` if:
/// - Validation fails (empty stream_id, holder_id, or invalid timestamps)
/// - The lock already exists (UNIQUE constraint violation)
/// - The database operation fails
pub async fn acquire_stream_lock(
    pool: &SqlitePool,
    lock: &EventStoreLock,
) -> Result<EventStoreLock, EventStoreLockError> {
    if lock.stream_id.is_empty() {
        return Err(EventStoreLockError::ValidationFailed(
            "stream_id cannot be empty".to_string(),
        ));
    }
    if lock.holder_id.is_empty() {
        return Err(EventStoreLockError::ValidationFailed(
            "holder_id cannot be empty".to_string(),
        ));
    }

    let result = sqlx::query(
        "INSERT INTO event_store_locks (stream_id, stream_seq, holder_id, acquired_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&lock.stream_id)
    .bind(lock.stream_seq)
    .bind(&lock.holder_id)
    .bind(lock.acquired_at)
    .bind(lock.expires_at)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(lock.clone()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed") || msg.contains("constraint") {
                Err(EventStoreLockError::LockConflict(format!(
                    "Lock already exists for stream_id='{}', stream_seq={}",
                    lock.stream_id, lock.stream_seq
                )))
            } else {
                Err(EventStoreLockError::DatabaseError(format!(
                    "Failed to acquire stream lock: {e}"
                )))
            }
        }
    }
}

/// Release a stream lock held by a specific agent.
///
/// Only the lock holder can release the lock. Returns an error if no matching
/// lock is found (i.e., the holder_id does not match).
///
/// # Errors
///
/// Returns `EventStoreLockError::NotFound` if no matching lock exists.
/// Returns `EventStoreLockError::DatabaseError` if the query fails.
pub async fn release_stream_lock(
    pool: &SqlitePool,
    stream_id: &str,
    stream_seq: i64,
    holder_id: &str,
) -> Result<(), EventStoreLockError> {
    let result = sqlx::query(
        "DELETE FROM event_store_locks
         WHERE stream_id = ?1 AND stream_seq = ?2 AND holder_id = ?3",
    )
    .bind(stream_id)
    .bind(stream_seq)
    .bind(holder_id)
    .execute(pool)
    .await
    .map_err(|e| {
        EventStoreLockError::DatabaseError(format!("Failed to release stream lock: {e}"))
    })?;

    if result.rows_affected() == 0 {
        return Err(EventStoreLockError::NotFound(format!(
            "No lock found for stream_id='{stream_id}', stream_seq={stream_seq}, holder_id='{holder_id}'"
        )));
    }

    Ok(())
}

/// Check if a specific stream position has an active (non-expired) lock.
///
/// A lock is considered active if its expires_at is greater than the
/// provided current timestamp.
///
/// # Errors
///
/// Returns `EventStoreLockError::QueryFailed` if the query fails.
pub async fn is_stream_locked(
    pool: &SqlitePool,
    stream_id: &str,
    stream_seq: i64,
    now: i64,
) -> Result<bool, EventStoreLockError> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM event_store_locks
         WHERE stream_id = ?1 AND stream_seq = ?2 AND expires_at > ?3",
    )
    .bind(stream_id)
    .bind(stream_seq)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to check stream lock: {e}")))?;

    let count: i64 = row
        .try_get("count")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'count' error: {e}")))?;

    Ok(count > 0)
}

/// Get all active (non-expired) locks for a stream.
///
/// Returns locks ordered by stream_seq ascending.
///
/// # Errors
///
/// Returns `EventStoreLockError::QueryFailed` if the query fails.
pub async fn get_stream_locks(
    pool: &SqlitePool,
    stream_id: &str,
    now: i64,
) -> Result<Vec<EventStoreLock>, EventStoreLockError> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT stream_id, stream_seq, holder_id, acquired_at, expires_at
         FROM event_store_locks
         WHERE stream_id = ?1 AND expires_at > ?2
         ORDER BY stream_seq ASC",
    )
    .bind(stream_id)
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to get stream locks: {e}")))?;

    rows.iter()
        .map(parse_event_store_lock_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to parse lock rows: {e}")))
}

/// Get the next available sequence number for a stream.
///
/// This is one more than the maximum stream_seq currently locked for the stream.
/// Returns 0 if no locks exist for the stream.
///
/// # Errors
///
/// Returns `EventStoreLockError::QueryFailed` if the query fails.
pub async fn get_next_sequence(
    pool: &SqlitePool,
    stream_id: &str,
) -> Result<i64, EventStoreLockError> {
    let row = sqlx::query(
        "SELECT COALESCE(MAX(stream_seq), -1) + 1 as next_seq
         FROM event_store_locks
         WHERE stream_id = ?1",
    )
    .bind(stream_id)
    .fetch_one(pool)
    .await
    .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to get next sequence: {e}")))?;

    let next_seq: i64 = row
        .try_get("next_seq")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'next_seq' error: {e}")))?;

    Ok(next_seq)
}

/// Remove all expired locks from the event_store_locks table.
///
/// Returns the number of locks that were cleaned up.
///
/// # Errors
///
/// Returns `EventStoreLockError::DatabaseError` if the query fails.
pub async fn cleanup_expired_stream_locks(
    pool: &SqlitePool,
    now: i64,
) -> Result<u64, EventStoreLockError> {
    let result = sqlx::query("DELETE FROM event_store_locks WHERE expires_at <= ?1")
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| {
            EventStoreLockError::DatabaseError(format!("Failed to cleanup expired locks: {e}"))
        })?;

    Ok(result.rows_affected())
}

/// Get all locks held by a specific agent.
///
/// Returns all locks (including expired ones) for the given holder.
///
/// # Errors
///
/// Returns `EventStoreLockError::QueryFailed` if the query fails.
pub async fn locks_by_holder(
    pool: &SqlitePool,
    holder_id: &str,
) -> Result<Vec<EventStoreLock>, EventStoreLockError> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT stream_id, stream_seq, holder_id, acquired_at, expires_at
         FROM event_store_locks
         WHERE holder_id = ?1
         ORDER BY stream_id, stream_seq",
    )
    .bind(holder_id)
    .fetch_all(pool)
    .await
    .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to get locks by holder: {e}")))?;

    rows.iter()
        .map(parse_event_store_lock_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Failed to parse lock rows: {e}")))
}

/// Ensure the event_store_locks schema exists before performing an operation.
///
/// This is a convenience function that initializes the schema if needed.
/// It is idempotent and safe to call multiple times.
///
/// # Errors
///
/// Returns `EventStoreLockError::DatabaseError` if schema creation fails.
pub async fn ensure_event_store_locks(pool: &SqlitePool) -> Result<(), EventStoreLockError> {
    ensure_event_store_lock_schema(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    async fn create_test_pool() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        ensure_event_store_lock_schema(&pool)
            .await
            .expect("Failed to create schema");

        (pool, temp_dir)
    }

    fn now_seconds() -> i64 {
        chrono::Utc::now().timestamp()
    }

    #[tokio::test]
    async fn given_valid_lock_when_acquire_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        let result = acquire_stream_lock(&pool, &lock).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn given_duplicate_lock_when_acquire_then_returns_conflict() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock)
            .await
            .expect("First acquire should succeed");

        let lock2 = EventStoreLock::new("session-queue", 1, "agent-2", now, now + 300).unwrap();
        let result = acquire_stream_lock(&pool, &lock2).await;

        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::LockConflict(msg)) => {
                assert!(msg.contains("session-queue"));
            }
            other => panic!("Expected LockConflict, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn given_active_lock_when_is_locked_then_true() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock)
            .await
            .expect("Acquire should succeed");

        let result = is_stream_locked(&pool, "session-queue", 1, now + 1).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn given_expired_lock_when_is_locked_then_false() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock =
            EventStoreLock::new("session-queue", 1, "agent-1", now - 600, now - 300).unwrap();
        acquire_stream_lock(&pool, &lock)
            .await
            .expect("Acquire should succeed");

        let result = is_stream_locked(&pool, "session-queue", 1, now).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn given_no_lock_when_is_locked_then_false() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result = is_stream_locked(&pool, "nonexistent", 1, now_seconds()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn given_active_lock_when_release_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock)
            .await
            .expect("Acquire should succeed");

        let result = release_stream_lock(&pool, "session-queue", 1, "agent-1").await;
        assert!(result.is_ok());

        let locked = is_stream_locked(&pool, "session-queue", 1, now + 1)
            .await
            .unwrap();
        assert!(!locked);
    }

    #[tokio::test]
    async fn given_wrong_holder_when_release_then_not_found() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock)
            .await
            .expect("Acquire should succeed");

        let result = release_stream_lock(&pool, "session-queue", 1, "agent-2").await;
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::NotFound(msg)) => {
                assert!(msg.contains("agent-2"));
            }
            other => panic!("Expected NotFound, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn given_multiple_locks_when_get_stream_locks_then_returns_active() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock1 = EventStoreLock::new("session-queue", 1, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("session-queue", 2, "agent-1", now, now + 300).unwrap();
        // This one is expired
        let lock3 =
            EventStoreLock::new("session-queue", 3, "agent-2", now - 600, now - 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();
        acquire_stream_lock(&pool, &lock3).await.unwrap();

        let locks = get_stream_locks(&pool, "session-queue", now + 1)
            .await
            .unwrap();
        assert_eq!(locks.len(), 2);
        assert_eq!(locks[0].stream_seq, 1);
        assert_eq!(locks[1].stream_seq, 2);
    }

    #[tokio::test]
    async fn given_no_locks_when_get_next_sequence_then_zero() {
        let (pool, _temp_dir) = create_test_pool().await;

        let next = get_next_sequence(&pool, "session-queue").await.unwrap();
        assert_eq!(next, 0);
    }

    #[tokio::test]
    async fn given_existing_locks_when_get_next_sequence_then_max_plus_one() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock1 = EventStoreLock::new("session-queue", 0, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("session-queue", 5, "agent-1", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();

        let next = get_next_sequence(&pool, "session-queue").await.unwrap();
        assert_eq!(next, 6);
    }

    #[tokio::test]
    async fn given_expired_locks_when_cleanup_then_removes_them() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let active = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        let expired = EventStoreLock::new("stream-1", 2, "agent-2", now - 600, now - 300).unwrap();

        acquire_stream_lock(&pool, &active).await.unwrap();
        acquire_stream_lock(&pool, &expired).await.unwrap();

        let cleaned = cleanup_expired_stream_locks(&pool, now).await.unwrap();
        assert_eq!(cleaned, 1);

        let remaining = get_stream_locks(&pool, "stream-1", now + 1).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].holder_id, "agent-1");
    }

    #[tokio::test]
    async fn given_holder_locks_when_locks_by_holder_then_returns_all() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock1 = EventStoreLock::new("stream-a", 1, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("stream-b", 1, "agent-1", now, now + 300).unwrap();
        let lock3 = EventStoreLock::new("stream-a", 2, "agent-2", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();
        acquire_stream_lock(&pool, &lock3).await.unwrap();

        let locks = locks_by_holder(&pool, "agent-1").await.unwrap();
        assert_eq!(locks.len(), 2);
    }

    #[tokio::test]
    async fn given_lock_with_empty_holder_when_acquire_then_fails() {
        let (pool, _temp_dir) = create_test_pool().await;
        // Construct a lock directly (bypassing EventStoreLock::new) with empty holder_id
        let lock = EventStoreLock {
            stream_id: "session-queue".to_string(),
            stream_seq: 1,
            holder_id: String::new(),
            acquired_at: 1000,
            expires_at: 1600,
        };

        let result = acquire_stream_lock(&pool, &lock).await;
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("holder_id"));
            }
            other => panic!("Expected ValidationFailed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn given_different_streams_when_acquire_same_seq_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock1 = EventStoreLock::new("stream-a", 1, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("stream-b", 1, "agent-2", now, now + 300).unwrap();

        assert!(acquire_stream_lock(&pool, &lock1).await.is_ok());
        assert!(acquire_stream_lock(&pool, &lock2).await.is_ok());
    }

    #[tokio::test]
    async fn given_no_schema_when_ensure_then_creates() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        let result = ensure_event_store_locks(&pool).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // release_stream_lock additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_nonexistent_stream_when_release_then_not_found() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result = release_stream_lock(&pool, "nonexistent-stream", 1, "agent-1").await;
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::NotFound(msg)) => {
                assert!(msg.contains("nonexistent-stream"));
                assert!(msg.contains("agent-1"));
            }
            other => panic!("Expected NotFound, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn given_released_lock_when_acquire_again_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();
        release_stream_lock(&pool, "stream-1", 1, "agent-1")
            .await
            .unwrap();

        // Same stream_id + stream_seq can be acquired again after release
        let new_lock = EventStoreLock::new("stream-1", 1, "agent-2", now, now + 300).unwrap();
        let result = acquire_stream_lock(&pool, &new_lock).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // locks_by_holder additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_no_locks_when_locks_by_holder_then_empty() {
        let (pool, _temp_dir) = create_test_pool().await;

        let locks = locks_by_holder(&pool, "nonexistent-agent").await.unwrap();
        assert!(locks.is_empty());
    }

    #[tokio::test]
    async fn given_holder_locks_when_locks_by_holder_then_sorted() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // Insert in non-sorted order
        let lock1 = EventStoreLock::new("stream-b", 3, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("stream-a", 1, "agent-1", now, now + 300).unwrap();
        let lock3 = EventStoreLock::new("stream-a", 2, "agent-1", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();
        acquire_stream_lock(&pool, &lock3).await.unwrap();

        let locks = locks_by_holder(&pool, "agent-1").await.unwrap();
        assert_eq!(locks.len(), 3);

        // Should be sorted by (stream_id, stream_seq)
        assert_eq!(locks[0].stream_id, "stream-a");
        assert_eq!(locks[0].stream_seq, 1);
        assert_eq!(locks[1].stream_id, "stream-a");
        assert_eq!(locks[1].stream_seq, 2);
        assert_eq!(locks[2].stream_id, "stream-b");
        assert_eq!(locks[2].stream_seq, 3);
    }

    #[tokio::test]
    async fn given_locks_by_holder_when_includes_expired() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // Expired lock
        let expired = EventStoreLock::new("stream-1", 1, "agent-1", now - 600, now - 300).unwrap();
        acquire_stream_lock(&pool, &expired).await.unwrap();

        // locks_by_holder returns ALL locks including expired
        let locks = locks_by_holder(&pool, "agent-1").await.unwrap();
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].stream_seq, 1);
    }

    // =========================================================================
    // cleanup_expired_stream_locks additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_no_expired_locks_when_cleanup_then_zero() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();

        let cleaned = cleanup_expired_stream_locks(&pool, now).await.unwrap();
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    async fn given_all_expired_locks_when_cleanup_then_removes_all() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock1 = EventStoreLock::new("stream-1", 1, "agent-1", now - 600, now - 300).unwrap();
        let lock2 = EventStoreLock::new("stream-1", 2, "agent-2", now - 600, now - 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();

        let cleaned = cleanup_expired_stream_locks(&pool, now).await.unwrap();
        assert_eq!(cleaned, 2);

        let locks = get_stream_locks(&pool, "stream-1", now + 1).await.unwrap();
        assert!(locks.is_empty());
    }

    #[tokio::test]
    async fn given_boundary_expiry_when_cleanup_then_correct() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // expires_at == now: should be cleaned up (expires_at <= now)
        let boundary = EventStoreLock::new("stream-1", 1, "agent-1", now - 300, now).unwrap();
        // expires_at == now + 1: should NOT be cleaned up
        let active = EventStoreLock::new("stream-1", 2, "agent-2", now, now + 1).unwrap();

        acquire_stream_lock(&pool, &boundary).await.unwrap();
        acquire_stream_lock(&pool, &active).await.unwrap();

        let cleaned = cleanup_expired_stream_locks(&pool, now).await.unwrap();
        assert_eq!(cleaned, 1);
    }

    // =========================================================================
    // get_stream_locks additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_nonexistent_stream_when_get_stream_locks_then_empty() {
        let (pool, _temp_dir) = create_test_pool().await;

        let locks = get_stream_locks(&pool, "nonexistent-stream", now_seconds())
            .await
            .unwrap();
        assert!(locks.is_empty());
    }

    #[tokio::test]
    async fn given_mixed_streams_when_get_stream_locks_then_only_matching() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock_a = EventStoreLock::new("stream-a", 1, "agent-1", now, now + 300).unwrap();
        let lock_b = EventStoreLock::new("stream-b", 1, "agent-2", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock_a).await.unwrap();
        acquire_stream_lock(&pool, &lock_b).await.unwrap();

        let locks_a = get_stream_locks(&pool, "stream-a", now + 1).await.unwrap();
        assert_eq!(locks_a.len(), 1);
        assert_eq!(locks_a[0].stream_id, "stream-a");

        let locks_b = get_stream_locks(&pool, "stream-b", now + 1).await.unwrap();
        assert_eq!(locks_b.len(), 1);
        assert_eq!(locks_b[0].stream_id, "stream-b");
    }

    // =========================================================================
    // get_next_sequence additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_gaps_in_sequences_when_get_next_sequence_then_max_plus_one() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // Locks at seq 1 and 5, gap at 2-4
        let lock1 = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("stream-1", 5, "agent-1", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();

        let next = get_next_sequence(&pool, "stream-1").await.unwrap();
        assert_eq!(next, 6);
    }

    #[tokio::test]
    async fn given_nonexistent_stream_when_get_next_sequence_then_zero() {
        let (pool, _temp_dir) = create_test_pool().await;

        let next = get_next_sequence(&pool, "nonexistent-stream")
            .await
            .unwrap();
        assert_eq!(next, 0);
    }

    // =========================================================================
    // acquire_stream_lock validation
    // =========================================================================

    #[tokio::test]
    async fn given_empty_stream_id_when_acquire_then_validation_fails() {
        let (pool, _temp_dir) = create_test_pool().await;
        // Construct directly to bypass EventStoreLock::new validation
        let lock = EventStoreLock {
            stream_id: String::new(),
            stream_seq: 1,
            holder_id: "agent-1".to_string(),
            acquired_at: 1000,
            expires_at: 1600,
        };

        let result = acquire_stream_lock(&pool, &lock).await;
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("stream_id"));
            }
            other => panic!("Expected ValidationFailed, got: {other:?}"),
        }
    }

    // =========================================================================
    // is_stream_locked additional cases
    // =========================================================================

    #[tokio::test]
    async fn given_boundary_expiry_when_is_locked_then_false() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // expires_at == now: should be considered expired (expires_at > ? where ? == now)
        let lock = EventStoreLock::new("stream-1", 1, "agent-1", now - 300, now).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();

        let locked = is_stream_locked(&pool, "stream-1", 1, now).await.unwrap();
        assert!(!locked);
    }

    #[tokio::test]
    async fn given_nonexistent_stream_when_is_locked_then_false() {
        let (pool, _temp_dir) = create_test_pool().await;

        let locked = is_stream_locked(&pool, "nonexistent-stream", 999, now_seconds())
            .await
            .unwrap();
        assert!(!locked);
    }

    #[tokio::test]
    async fn given_lock_on_different_seq_when_is_locked_then_false() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();

        // Different seq should not be locked
        let locked = is_stream_locked(&pool, "stream-1", 2, now + 1)
            .await
            .unwrap();
        assert!(!locked);
    }

    // =========================================================================
    // Full lifecycle integration test
    // =========================================================================

    #[tokio::test]
    async fn given_full_lifecycle_when_acquire_check_release_then_consistent() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // 1. Lock is available
        assert!(!is_stream_locked(&pool, "lifecycle-stream", 1, now)
            .await
            .unwrap());

        // 2. Acquire lock
        let lock = EventStoreLock::new("lifecycle-stream", 1, "agent-1", now, now + 300).unwrap();
        let acquired = acquire_stream_lock(&pool, &lock).await.unwrap();
        assert_eq!(acquired.stream_id, "lifecycle-stream");

        // 3. Lock is now held
        assert!(is_stream_locked(&pool, "lifecycle-stream", 1, now + 1)
            .await
            .unwrap());

        // 4. Next sequence should be 2
        assert_eq!(
            get_next_sequence(&pool, "lifecycle-stream").await.unwrap(),
            2
        );

        // 5. Get locks returns our lock
        let locks = get_stream_locks(&pool, "lifecycle-stream", now + 1)
            .await
            .unwrap();
        assert_eq!(locks.len(), 1);

        // 6. Release lock
        release_stream_lock(&pool, "lifecycle-stream", 1, "agent-1")
            .await
            .unwrap();

        // 7. Lock is now released
        assert!(!is_stream_locked(&pool, "lifecycle-stream", 1, now + 1)
            .await
            .unwrap());

        // 8. Same slot can be re-acquired
        let new_lock =
            EventStoreLock::new("lifecycle-stream", 1, "agent-2", now, now + 300).unwrap();
        assert!(acquire_stream_lock(&pool, &new_lock).await.is_ok());

        // 9. Cleanup expired (none should be cleaned)
        let cleaned = cleanup_expired_stream_locks(&pool, now).await.unwrap();
        assert_eq!(cleaned, 0);
    }

    // =========================================================================
    // Concurrent stream isolation
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_streams_when_operations_then_independent() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // Create locks across different streams
        let lock_a1 = EventStoreLock::new("stream-a", 1, "agent-1", now, now + 300).unwrap();
        let lock_b1 = EventStoreLock::new("stream-b", 1, "agent-2", now, now + 300).unwrap();
        let lock_c1 = EventStoreLock::new("stream-c", 1, "agent-3", now, now + 300).unwrap();

        assert!(acquire_stream_lock(&pool, &lock_a1).await.is_ok());
        assert!(acquire_stream_lock(&pool, &lock_b1).await.is_ok());
        assert!(acquire_stream_lock(&pool, &lock_c1).await.is_ok());

        // Release one stream, others remain
        release_stream_lock(&pool, "stream-b", 1, "agent-2")
            .await
            .unwrap();

        assert!(is_stream_locked(&pool, "stream-a", 1, now + 1)
            .await
            .unwrap());
        assert!(!is_stream_locked(&pool, "stream-b", 1, now + 1)
            .await
            .unwrap());
        assert!(is_stream_locked(&pool, "stream-c", 1, now + 1)
            .await
            .unwrap());

        // Each stream's next sequence is independent
        assert_eq!(get_next_sequence(&pool, "stream-a").await.unwrap(), 2);
        assert_eq!(get_next_sequence(&pool, "stream-b").await.unwrap(), 0); // lock released
        assert_eq!(get_next_sequence(&pool, "stream-c").await.unwrap(), 2);
    }

    // =========================================================================
    // get_stream_locks ordering
    // =========================================================================

    #[tokio::test]
    async fn given_unordered_inserts_when_get_stream_locks_then_ordered_by_seq() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        // Insert in reverse sequence order
        let lock3 = EventStoreLock::new("stream-1", 3, "agent-1", now, now + 300).unwrap();
        let lock1 = EventStoreLock::new("stream-1", 1, "agent-1", now, now + 300).unwrap();
        let lock2 = EventStoreLock::new("stream-1", 2, "agent-1", now, now + 300).unwrap();

        acquire_stream_lock(&pool, &lock3).await.unwrap();
        acquire_stream_lock(&pool, &lock1).await.unwrap();
        acquire_stream_lock(&pool, &lock2).await.unwrap();

        let locks = get_stream_locks(&pool, "stream-1", now + 1).await.unwrap();
        assert_eq!(locks.len(), 3);
        assert_eq!(locks[0].stream_seq, 1);
        assert_eq!(locks[1].stream_seq, 2);
        assert_eq!(locks[2].stream_seq, 3);
    }

    // =========================================================================
    // parse_event_store_lock_row integration via repository
    // =========================================================================

    #[tokio::test]
    async fn given_inserted_lock_when_get_stream_locks_then_values_roundtrip() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("roundtrip-stream", 42, "holder-x", now, now + 999).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();

        let locks = get_stream_locks(&pool, "roundtrip-stream", now + 1)
            .await
            .unwrap();
        assert_eq!(locks.len(), 1);

        let retrieved = &locks[0];
        assert_eq!(retrieved.stream_id, lock.stream_id);
        assert_eq!(retrieved.stream_seq, lock.stream_seq);
        assert_eq!(retrieved.holder_id, lock.holder_id);
        assert_eq!(retrieved.acquired_at, lock.acquired_at);
        assert_eq!(retrieved.expires_at, lock.expires_at);
    }

    #[tokio::test]
    async fn given_inserted_lock_when_locks_by_holder_then_values_roundtrip() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = now_seconds();

        let lock = EventStoreLock::new("holder-stream", 99, "my-agent", now, now + 500).unwrap();
        acquire_stream_lock(&pool, &lock).await.unwrap();

        let locks = locks_by_holder(&pool, "my-agent").await.unwrap();
        assert_eq!(locks.len(), 1);

        let retrieved = &locks[0];
        assert_eq!(retrieved, &lock);
    }
}
