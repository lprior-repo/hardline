//! Operation Log Database Schema
//!
//! This module provides SQLite schema and operations for the operation_log table,
//! which implements event sourcing for tracking all state changes in the system.
//!
//! # Design
//!
//! The operation_log is an append-only event store that enables:
//! - Event sourcing (rebuilding state from event history)
//! - Audit logging (complete history of all changes)
//! - Projections (deriving read models from event stream)
//! - Temporal queries (state at any point in time)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

// Re-export types from sibling modules
pub use crate::infrastructure::operation_log_repository::{
    get_stream_version, insert_operation_log, query_all_operations, query_stream_events,
};
pub use crate::infrastructure::operation_log_schema::ensure_operation_log_schema;
pub use crate::infrastructure::operation_log_types::{
    parse_datetime, parse_operation_log_row, OperationLogEntry, OperationLogError,
};

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use crate::infrastructure::operation_log_repository::{
        get_stream_version, insert_operation_log, query_all_operations, query_stream_events,
    };
    use crate::infrastructure::operation_log_schema::ensure_operation_log_schema;
    use crate::infrastructure::operation_log_types::OperationLogEntry;

    async fn create_test_pool() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        ensure_operation_log_schema(&pool)
            .await
            .expect("Failed to create schema");

        (pool, temp_dir)
    }

    // Behavior: Creating a valid operation log entry succeeds
    #[tokio::test]
    async fn given_valid_entry_when_create_then_succeeds() {
        let entry = OperationLogEntry::new(
            "session_created",
            r#"{"session_id": "s1", "name": "test"}"#,
            "session-s1",
            1,
        )
        .expect("Failed to create entry");

        assert!(!entry.event_type.is_empty());
        assert!(!entry.stream_id.is_empty());
    }

    // Behavior: Creating entry with empty event_type fails
    #[tokio::test]
    async fn given_empty_event_type_when_create_then_returns_validation_error() {
        let result = OperationLogEntry::new("", r#"{"data": "test"}"#, "stream-1", 1);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(
                e,
                crate::infrastructure::operation_log_types::OperationLogError::ValidationFailed(_)
            ));
        }
    }

    // Behavior: Creating entry with empty stream_id fails
    #[tokio::test]
    async fn given_empty_stream_id_when_create_then_returns_validation_error() {
        let result = OperationLogEntry::new("test_event", r#"{"data": "test"}"#, "", 1);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(
                e,
                crate::infrastructure::operation_log_types::OperationLogError::ValidationFailed(_)
            ));
        }
    }

    // Behavior: Insert a valid operation log entry into database
    #[tokio::test]
    async fn given_valid_entry_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        let entry = OperationLogEntry::new(
            "session_created",
            r#"{"session_id": "s1", "name": "test"}"#,
            "session-s1",
            1,
        )
        .expect("Failed to create entry");

        let result = insert_operation_log(&pool, &entry).await;
        assert!(result.is_ok());

        let inserted = result.unwrap();
        assert!(inserted.id > 0);
        assert_eq!(inserted.event_type, "session_created");
    }

    // Behavior: Query stream events returns all events for that stream
    #[tokio::test]
    async fn given_multiple_events_when_query_stream_then_returns_all() {
        let (pool, _temp_dir) = create_test_pool().await;

        let events = vec![
            OperationLogEntry::new(
                "session_created",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                1,
            )
            .unwrap(),
            OperationLogEntry::new(
                "session_activated",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                2,
            )
            .unwrap(),
            OperationLogEntry::new(
                "session_completed",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                3,
            )
            .unwrap(),
        ];

        for event in &events {
            insert_operation_log(&pool, event)
                .await
                .expect("Insert failed");
        }

        let results = query_stream_events(&pool, "session-s1")
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].stream_version, 1);
        assert_eq!(results[1].stream_version, 2);
        assert_eq!(results[2].stream_version, 3);
    }

    // Behavior: Get stream version returns correct version
    #[tokio::test]
    async fn given_events_when_get_stream_version_then_returns_max() {
        let (pool, _temp_dir) = create_test_pool().await;

        let events = vec![
            OperationLogEntry::new("event1", "{}", "stream-1", 1).unwrap(),
            OperationLogEntry::new("event2", "{}", "stream-1", 2).unwrap(),
            OperationLogEntry::new("event3", "{}", "stream-1", 3).unwrap(),
        ];

        for event in &events {
            insert_operation_log(&pool, event)
                .await
                .expect("Insert failed");
        }

        let version = get_stream_version(&pool, "stream-1")
            .await
            .expect("Query failed");

        assert_eq!(version, 3);
    }

    // Behavior: Query all operations with limit
    #[tokio::test]
    async fn given_many_events_when_query_with_limit_then_respects_limit() {
        let (pool, _temp_dir) = create_test_pool().await;

        for i in 0..10 {
            let entry = OperationLogEntry::new(
                format!("event_{}", i),
                format!(r#"{{"i": {}}}"#, i),
                format!("stream-{}", i % 3),
                1,
            )
            .unwrap();
            insert_operation_log(&pool, &entry)
                .await
                .expect("Insert failed");
        }

        let results = query_all_operations(&pool, Some(5))
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 5);
    }

    // Behavior: Schema creation is idempotent
    #[tokio::test]
    async fn given_schema_exists_when_create_again_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result1 = ensure_operation_log_schema(&pool).await;
        assert!(result1.is_ok());

        let result2 = ensure_operation_log_schema(&pool).await;
        assert!(result2.is_ok());
    }

    // =========================================================================
    // query_stream_events: empty stream returns empty vec
    // =========================================================================

    #[tokio::test]
    async fn given_no_events_when_query_stream_then_empty() {
        let (pool, _temp_dir) = create_test_pool().await;

        let results = query_stream_events(&pool, "nonexistent-stream")
            .await
            .expect("Query should succeed");

        assert!(results.is_empty());
    }

    // =========================================================================
    // query_stream_events: stream isolation
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_streams_when_query_then_only_returns_matching() {
        let (pool, _temp_dir) = create_test_pool().await;

        insert_operation_log(
            &pool,
            &OperationLogEntry::new("evt-a", "{}", "stream-1", 1).unwrap(),
        )
        .await
        .expect("Insert failed");
        insert_operation_log(
            &pool,
            &OperationLogEntry::new("evt-b", "{}", "stream-2", 1).unwrap(),
        )
        .await
        .expect("Insert failed");
        insert_operation_log(
            &pool,
            &OperationLogEntry::new("evt-c", "{}", "stream-1", 2).unwrap(),
        )
        .await
        .expect("Insert failed");

        let results = query_stream_events(&pool, "stream-1")
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].event_type, "evt-a");
        assert_eq!(results[1].event_type, "evt-c");
    }

    // =========================================================================
    // get_stream_version: empty stream returns 0
    // =========================================================================

    #[tokio::test]
    async fn given_no_events_when_get_stream_version_then_zero() {
        let (pool, _temp_dir) = create_test_pool().await;

        let version = get_stream_version(&pool, "nonexistent-stream")
            .await
            .expect("Query should succeed");

        assert_eq!(version, 0);
    }

    // =========================================================================
    // query_all_operations: no limit returns all
    // =========================================================================

    #[tokio::test]
    async fn given_events_when_query_all_no_limit_then_returns_all() {
        let (pool, _temp_dir) = create_test_pool().await;

        for i in 0..5 {
            insert_operation_log(
                &pool,
                &OperationLogEntry::new("evt", "{}", format!("s-{i}"), 1).unwrap(),
            )
            .await
            .expect("Insert failed");
        }

        let results = query_all_operations(&pool, None)
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 5);
    }

    // =========================================================================
    // query_all_operations: empty table returns empty vec
    // =========================================================================

    #[tokio::test]
    async fn given_no_events_when_query_all_then_empty() {
        let (pool, _temp_dir) = create_test_pool().await;

        let results = query_all_operations(&pool, None)
            .await
            .expect("Query should succeed");

        assert!(results.is_empty());
    }

    // =========================================================================
    // query_all_operations: limit greater than count returns all
    // =========================================================================

    #[tokio::test]
    async fn given_three_events_when_query_all_limit_100_then_returns_three() {
        let (pool, _temp_dir) = create_test_pool().await;

        for i in 0..3 {
            insert_operation_log(
                &pool,
                &OperationLogEntry::new("evt", "{}", format!("s-{i}"), 1).unwrap(),
            )
            .await
            .expect("Insert failed");
        }

        let results = query_all_operations(&pool, Some(100))
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 3);
    }

    // =========================================================================
    // Insert returns entry with correct fields
    // =========================================================================

    #[tokio::test]
    async fn given_valid_entry_when_insert_then_returns_entry_with_id() {
        let (pool, _temp_dir) = create_test_pool().await;

        let entry = OperationLogEntry::new("test_event", r#"{"key": "val"}"#, "stream-x", 5)
            .expect("Failed to create entry");

        let inserted = insert_operation_log(&pool, &entry)
            .await
            .expect("Insert failed");

        assert!(inserted.id > 0);
        assert_eq!(inserted.event_type, "test_event");
        assert_eq!(inserted.payload, r#"{"key": "val"}"#);
        assert_eq!(inserted.stream_id, "stream-x");
        assert_eq!(inserted.stream_version, 5);
    }

    // =========================================================================
    // Insert with large payload
    // =========================================================================

    #[tokio::test]
    async fn given_entry_with_large_payload_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let large_payload = "x".repeat(10_000);

        let entry = OperationLogEntry::new("big_event", &large_payload, "stream-big", 1)
            .expect("Failed to create entry");

        let inserted = insert_operation_log(&pool, &entry)
            .await
            .expect("Insert failed");

        assert_eq!(inserted.payload.len(), 10_000);
    }

    // =========================================================================
    // get_stream_version: multiple streams
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_streams_when_get_version_then_returns_correct() {
        let (pool, _temp_dir) = create_test_pool().await;

        insert_operation_log(&pool, &OperationLogEntry::new("e", "{}", "s-1", 1).unwrap())
            .await
            .expect("Insert failed");
        insert_operation_log(&pool, &OperationLogEntry::new("e", "{}", "s-1", 2).unwrap())
            .await
            .expect("Insert failed");
        insert_operation_log(&pool, &OperationLogEntry::new("e", "{}", "s-2", 1).unwrap())
            .await
            .expect("Insert failed");

        let v1 = get_stream_version(&pool, "s-1").await.expect("Query failed");
        let v2 = get_stream_version(&pool, "s-2").await.expect("Query failed");
        let v3 = get_stream_version(&pool, "s-3").await.expect("Query failed");

        assert_eq!(v1, 2);
        assert_eq!(v2, 1);
        assert_eq!(v3, 0);
    }
}
