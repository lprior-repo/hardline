// Integration tests have relaxed clippy settings for test infrastructure.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Integration tests for `conflict_resolutions` table.
//!
//! Test plan based on contract: bd-2gj
//! Covers: schema initialization, insert operations, query operations,
//! invariants, error handling, and performance.

use std::time::Duration;

use isolate_core::coordination::{
    conflict_resolutions_entities::ConflictResolution, get_conflict_resolutions,
    get_resolutions_by_decider, get_resolutions_by_time_range, init_conflict_resolutions_schema,
    insert_conflict_resolution,
};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};

/// Helper: Create in-memory test database
async fn create_test_pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(1))
        .connect(":memory:")
        .await
        .expect("Failed to create test pool")
}

/// Helper: Create a test conflict resolution
fn create_test_resolution(
    id: i64,
    session: &str,
    file: &str,
    decider: &str,
    timestamp: &str,
) -> ConflictResolution {
    ConflictResolution {
        id,
        timestamp: timestamp.to_string(),
        session: session.to_string(),
        file: file.to_string(),
        strategy: "accept_theirs".to_string(),
        reason: Some("Test resolution".to_string()),
        confidence: Some("high".to_string()),
        decider: decider.to_string(),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SCHEMA TESTS (10 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_schema_init_creates_table() {
    let pool = create_test_pool().await;
    let result = init_conflict_resolutions_schema(&pool).await;
    assert!(result.is_ok(), "Schema initialization should succeed");

    // Verify table exists
    let check = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='conflict_resolutions'",
    )
    .fetch_one(&pool)
    .await;

    assert!(
        check.is_ok(),
        "conflict_resolutions table should exist in database"
    );
}

#[tokio::test]
async fn test_schema_init_is_idempotent() {
    let pool = create_test_pool().await;

    // First initialization
    let result1 = init_conflict_resolutions_schema(&pool).await;
    assert!(result1.is_ok());

    // Second initialization (should not fail)
    let result2 = init_conflict_resolutions_schema(&pool).await;
    assert!(
        result2.is_ok(),
        "Schema initialization should be idempotent"
    );
}

#[tokio::test]
async fn test_schema_init_creates_session_index() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Verify index exists
    let check = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_session'",
    )
    .fetch_one(&pool)
    .await;

    assert!(check.is_ok(), "session index should exist");
}

#[tokio::test]
async fn test_schema_init_creates_timestamp_index() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Verify index exists
    let check = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_timestamp'",
    )
    .fetch_one(&pool)
    .await;

    assert!(check.is_ok(), "timestamp index should exist");
}

#[tokio::test]
async fn test_schema_init_creates_decider_index() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Verify index exists
    let check = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_decider'",
    )
    .fetch_one(&pool)
    .await;

    assert!(check.is_ok(), "decider index should exist");
}

#[tokio::test]
async fn test_schema_init_creates_session_timestamp_composite_index() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Verify composite index exists
    let check = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_session_timestamp'",
    )
    .fetch_one(&pool)
    .await;

    assert!(
        check.is_ok(),
        "session_timestamp composite index should exist"
    );
}

#[tokio::test]
async fn test_schema_has_correct_columns() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Get table info
    let rows = sqlx::query("PRAGMA table_info(conflict_resolutions)")
        .fetch_all(&pool)
        .await
        .expect("Should fetch table info");

    let column_names: Vec<&str> = rows.iter().filter_map(|row| row.get("name")).collect();

    assert!(column_names.contains(&"id"), "Should have 'id' column");
    assert!(
        column_names.contains(&"timestamp"),
        "Should have 'timestamp' column"
    );
    assert!(
        column_names.contains(&"session"),
        "Should have 'session' column"
    );
    assert!(column_names.contains(&"file"), "Should have 'file' column");
    assert!(
        column_names.contains(&"strategy"),
        "Should have 'strategy' column"
    );
    assert!(
        column_names.contains(&"reason"),
        "Should have 'reason' column"
    );
    assert!(
        column_names.contains(&"confidence"),
        "Should have 'confidence' column"
    );
    assert!(
        column_names.contains(&"decider"),
        "Should have 'decider' column"
    );
}

#[tokio::test]
async fn test_schema_id_is_primary_key() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Check that id is PK
    let rows = sqlx::query("PRAGMA table_info(conflict_resolutions)")
        .fetch_all(&pool)
        .await
        .expect("Should fetch table info");

    let id_row = rows
        .iter()
        .find(|row| row.get::<String, _>("name") == "id")
        .expect("Should find id column");

    let pk: i32 = id_row.get("pk");
    assert_eq!(pk, 1, "id should be primary key");
}

#[tokio::test]
async fn test_schema_decider_has_check_constraint() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Get SQL for table
    let row = sqlx::query(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='conflict_resolutions'",
    )
    .fetch_one(&pool)
    .await
    .expect("Should get table SQL");

    let sql: String = row.get("sql");
    assert!(
        sql.contains("CHECK(decider IN ('ai', 'human'))"),
        "Should have CHECK constraint on decider column"
    );
}

#[tokio::test]
async fn test_schema_all_indexes_created() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Count indexes for conflict_resolutions table
    let rows = sqlx::query(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type='index' AND name LIKE 'idx_conflict_resolutions_%'",
    )
    .fetch_one(&pool)
    .await
    .expect("Should count indexes");

    let count: i64 = rows.get("count");
    assert_eq!(count, 4, "Should have 4 indexes");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// INSERT TESTS (15 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_insert_valid_resolution() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution = create_test_resolution(
        0,
        "test-session",
        "src/main.rs",
        "ai",
        "2025-02-18T12:34:56Z",
    );

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok(), "Insert should succeed");

    let id = result.unwrap();
    assert!(id > 0, "ID should be positive");
}

#[tokio::test]
async fn test_insert_auto_generates_id() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution1 =
        create_test_resolution(0, "session-1", "file1.rs", "ai", "2025-02-18T12:00:00Z");
    let resolution2 =
        create_test_resolution(0, "session-2", "file2.rs", "human", "2025-02-18T12:01:00Z");

    let id1 = insert_conflict_resolution(&pool, &resolution1)
        .await
        .expect("Insert should succeed");
    let id2 = insert_conflict_resolution(&pool, &resolution2)
        .await
        .expect("Insert should succeed");

    assert_eq!(id2, id1 + 1, "IDs should be monotonically increasing");
}

#[tokio::test]
async fn test_insert_ai_decider() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insert_human_decider() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "human", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insert_invalid_decider_fails() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "robot", "2025-02-18T12:00:00Z");

    // This should fail validation before reaching DB
    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should fail with invalid decider");
}

#[tokio::test]
async fn test_insert_with_reason() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    resolution.reason = Some("Incoming changes are more recent".to_string());

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insert_with_confidence() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    resolution.confidence = Some("0.95".to_string());

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insert_without_optional_fields() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    resolution.reason = None;
    resolution.confidence = None;

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok(), "Should allow optional fields to be NULL");
}

#[tokio::test]
async fn test_insert_empty_session_fails() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution = create_test_resolution(0, "", "file.rs", "ai", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should fail with empty session");
}

#[tokio::test]
async fn test_insert_empty_file_fails() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution = create_test_resolution(0, "session-1", "", "ai", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should fail with empty file");
}

#[tokio::test]
async fn test_insert_empty_strategy_fails() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    resolution.strategy = String::new();

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should fail with empty strategy");
}

#[tokio::test]
async fn test_insert_empty_timestamp_fails() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution = create_test_resolution(0, "session-1", "file.rs", "ai", "");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should fail with empty timestamp");
}

#[tokio::test]
async fn test_insert_multiple_resolutions() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    for i in 0..5 {
        let resolution = create_test_resolution(
            0,
            &format!("session-{i}"),
            &format!("file{i}.rs"),
            "ai",
            &format!("2025-02-18T12:00:{i:02}Z"),
        );
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // Verify count
    let row = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .expect("Should count rows");

    let count: i64 = row.get("count");
    assert_eq!(count, 5, "Should have 5 records");
}

#[tokio::test]
async fn test_insert_different_strategies() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let strategies = ["accept_theirs", "accept_ours", "manual_merge", "skip"];

    for strategy in strategies {
        let mut resolution =
            create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
        resolution.strategy = strategy.to_string();
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // All strategies should be stored
    let rows = sqlx::query("SELECT DISTINCT strategy FROM conflict_resolutions")
        .fetch_all(&pool)
        .await
        .expect("Should fetch strategies");

    assert_eq!(rows.len(), 4, "Should have 4 different strategies");
}

#[tokio::test]
async fn test_insert_preserves_all_fields() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut resolution = create_test_resolution(
        0,
        "test-session",
        "src/lib.rs",
        "human",
        "2025-02-18T15:30:45Z",
    );
    resolution.strategy = "manual_merge".to_string();
    resolution.reason = Some("Manual review required".to_string());
    resolution.confidence = Some("medium".to_string());

    let id = insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Fetch and verify
    let row = sqlx::query("SELECT * FROM conflict_resolutions WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("Should fetch inserted row");

    let session: String = row.get("session");
    let file: String = row.get("file");
    let strategy: String = row.get("strategy");
    let reason: Option<String> = row
        .try_get::<Option<String>, _>("reason")
        .expect("Should get reason");
    let confidence: Option<String> = row
        .try_get::<Option<String>, _>("confidence")
        .expect("Should get confidence");
    let decider: String = row.get("decider");

    assert_eq!(session, "test-session");
    assert_eq!(file, "src/lib.rs");
    assert_eq!(strategy, "manual_merge");
    assert_eq!(reason, Some("Manual review required".to_string()));
    assert_eq!(confidence, Some("medium".to_string()));
    assert_eq!(decider, "human");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUERY TESTS (10 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_get_resolutions_by_session() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Insert resolutions for two sessions
    for i in 0..3 {
        let resolution = create_test_resolution(
            0,
            "session-a",
            &format!("file{i}.rs"),
            "ai",
            "2025-02-18T12:00:00Z",
        );
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    for i in 0..2 {
        let resolution = create_test_resolution(
            0,
            "session-b",
            &format!("file{i}.rs"),
            "human",
            "2025-02-18T12:00:00Z",
        );
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // Query session-a
    let resolutions = get_conflict_resolutions(&pool, "session-a")
        .await
        .expect("Query should succeed");

    assert_eq!(
        resolutions.len(),
        3,
        "Should get 3 resolutions for session-a"
    );

    // Verify all are from session-a
    for r in &resolutions {
        assert_eq!(r.session, "session-a");
    }
}

#[tokio::test]
async fn test_get_resolutions_by_empty_session() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let result = get_conflict_resolutions(&pool, "").await;
    assert!(result.is_err(), "Should fail with empty session");
}

#[tokio::test]
async fn test_get_resolutions_by_nonexistent_session() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolutions = get_conflict_resolutions(&pool, "nonexistent")
        .await
        .expect("Query should succeed");

    assert_eq!(
        resolutions.len(),
        0,
        "Should return empty Vec for nonexistent session"
    );
}

#[tokio::test]
async fn test_get_resolutions_by_decider_ai() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Insert AI resolutions
    for _ in 0..3 {
        let resolution =
            create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // Insert human resolution
    let resolution =
        create_test_resolution(0, "session-2", "file.rs", "human", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Query AI resolutions
    let resolutions = get_resolutions_by_decider(&pool, "ai")
        .await
        .expect("Query should succeed");

    assert_eq!(resolutions.len(), 3, "Should get 3 AI resolutions");

    // Verify all are AI
    for r in &resolutions {
        assert_eq!(r.decider, "ai");
    }
}

#[tokio::test]
async fn test_get_resolutions_by_decider_human() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Insert human resolutions
    for _ in 0..2 {
        let resolution =
            create_test_resolution(0, "session-1", "file.rs", "human", "2025-02-18T12:00:00Z");
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // Insert AI resolution
    let resolution =
        create_test_resolution(0, "session-2", "file.rs", "ai", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Query human resolutions
    let resolutions = get_resolutions_by_decider(&pool, "human")
        .await
        .expect("Query should succeed");

    assert_eq!(resolutions.len(), 2, "Should get 2 human resolutions");

    // Verify all are human
    for r in &resolutions {
        assert_eq!(r.decider, "human");
    }
}

#[tokio::test]
async fn test_get_resolutions_by_invalid_decider() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let result = get_resolutions_by_decider(&pool, "robot").await;
    assert!(result.is_err(), "Should fail with invalid decider");
}

#[tokio::test]
async fn test_get_resolutions_by_time_range() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Insert resolutions at different times
    let timestamps = [
        "2025-02-18T10:00:00Z",
        "2025-02-18T11:00:00Z",
        "2025-02-18T12:00:00Z",
        "2025-02-18T13:00:00Z",
        "2025-02-18T14:00:00Z",
    ];

    for ts in &timestamps {
        let resolution = create_test_resolution(0, "session-1", "file.rs", "ai", ts);
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    // Query for range [11:00, 13:00) - should get 11:00 and 12:00
    let resolutions =
        get_resolutions_by_time_range(&pool, "2025-02-18T11:00:00Z", "2025-02-18T13:00:00Z")
            .await
            .expect("Query should succeed");

    assert_eq!(resolutions.len(), 2, "Should get 2 resolutions in range");
    assert_eq!(resolutions[0].timestamp, "2025-02-18T11:00:00Z");
    assert_eq!(resolutions[1].timestamp, "2025-02-18T12:00:00Z");
}

#[tokio::test]
async fn test_get_resolutions_by_time_range_inclusive_start() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Range start == timestamp - should include it
    let resolutions =
        get_resolutions_by_time_range(&pool, "2025-02-18T12:00:00Z", "2025-02-18T13:00:00Z")
            .await
            .expect("Query should succeed");

    assert_eq!(
        resolutions.len(),
        1,
        "Should include record at start boundary"
    );
}

#[tokio::test]
async fn test_get_resolutions_by_time_range_exclusive_end() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T13:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Range end == timestamp - should exclude it
    let resolutions =
        get_resolutions_by_time_range(&pool, "2025-02-18T12:00:00Z", "2025-02-18T13:00:00Z")
            .await
            .expect("Query should succeed");

    assert_eq!(
        resolutions.len(),
        0,
        "Should exclude record at end boundary"
    );
}

#[tokio::test]
async fn test_get_resolutions_by_time_range_invalid_range() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let result =
        get_resolutions_by_time_range(&pool, "2025-02-18T13:00:00Z", "2025-02-18T12:00:00Z").await;
    assert!(result.is_err(), "Should fail when start_time >= end_time");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// INVARIANT TESTS (8 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_append_only_no_update_operations() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    let id = insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Attempt UPDATE (this is just a check that we don't provide UPDATE functions)
    // The implementation doesn't expose UPDATE operations, so this test documents
    // the append-only design
    let check = sqlx::query("SELECT * FROM conflict_resolutions WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("Should fetch record");

    let file: String = check.get("file");
    assert_eq!(file, "file.rs", "Record should remain unchanged");
}

#[tokio::test]
async fn test_append_only_no_delete_operations() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Record should persist (we don't expose DELETE operations)
    let resolutions = get_conflict_resolutions(&pool, "session-1")
        .await
        .expect("Query should succeed");

    assert_eq!(resolutions.len(), 1, "Record should persist");
}

#[tokio::test]
async fn test_decider_constraint_enforced_ai() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Verify decider is stored correctly
    let resolutions = get_resolutions_by_decider(&pool, "ai")
        .await
        .expect("Query should succeed");

    assert_eq!(resolutions.len(), 1);
    assert_eq!(resolutions[0].decider, "ai");
}

#[tokio::test]
async fn test_decider_constraint_enforced_human() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "human", "2025-02-18T12:00:00Z");
    insert_conflict_resolution(&pool, &resolution)
        .await
        .expect("Insert should succeed");

    // Verify decider is stored correctly
    let resolutions = get_resolutions_by_decider(&pool, "human")
        .await
        .expect("Query should succeed");

    assert_eq!(resolutions.len(), 1);
    assert_eq!(resolutions[0].decider, "human");
}

#[tokio::test]
async fn test_decider_constraint_rejects_invalid_values() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Invalid decider should fail at validation layer
    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "robot", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err(), "Should reject invalid decider");
}

#[tokio::test]
async fn test_primary_key_uniqueness() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution1 =
        create_test_resolution(0, "session-1", "file1.rs", "ai", "2025-02-18T12:00:00Z");
    let resolution2 =
        create_test_resolution(0, "session-1", "file2.rs", "ai", "2025-02-18T12:01:00Z");

    let id1 = insert_conflict_resolution(&pool, &resolution1)
        .await
        .expect("Insert should succeed");
    let id2 = insert_conflict_resolution(&pool, &resolution2)
        .await
        .expect("Insert should succeed");

    assert_ne!(id1, id2, "IDs should be unique");
}

#[tokio::test]
async fn test_primary_key_monotonically_increasing() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let mut prev_id = 0;

    for i in 0..10 {
        let resolution = create_test_resolution(
            0,
            "session-1",
            &format!("file{i}.rs"),
            "ai",
            &format!("2025-02-18T12:00:{i:02}Z"),
        );
        let id = insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");

        assert!(id > prev_id, "ID should increase monotonically");
        prev_id = id;
    }
}

#[tokio::test]
async fn test_non_null_fields_enforced() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // All required fields present - should succeed
    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_ok());

    // Empty required fields should fail
    let empty_resolution = create_test_resolution(0, "", "file.rs", "ai", "2025-02-18T12:00:00Z");
    let result = insert_conflict_resolution(&pool, &empty_resolution).await;
    assert!(result.is_err(), "Empty session should fail");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ERROR TESTS (3 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_error_invalid_decider() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "invalid", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err());

    match result {
        Err(isolate_core::Error::ValidationError { .. }) => {}
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_error_empty_required_field() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let resolution = create_test_resolution(0, "session-1", "", "ai", "2025-02-18T12:00:00Z");

    let result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(result.is_err());

    match result {
        Err(isolate_core::Error::ValidationError { .. }) => {}
        _ => panic!("Expected Validation error"),
    }
}

#[tokio::test]
async fn test_error_invalid_time_range() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let result =
        get_resolutions_by_time_range(&pool, "2025-02-18T13:00:00Z", "2025-02-18T12:00:00Z").await;
    assert!(result.is_err());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PERFORMANCE TESTS (3 tests)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[tokio::test]
async fn test_performance_insert_latency() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let start = std::time::Instant::now();

    let resolution =
        create_test_resolution(0, "session-1", "file.rs", "ai", "2025-02-18T12:00:00Z");
    let result = insert_conflict_resolution(&pool, &resolution).await;

    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Insert should succeed");
    assert!(
        elapsed.as_millis() < 100,
        "Insert should complete in < 100ms, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_performance_query_latency() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    // Insert 100 records
    for i in 0..100 {
        let resolution = create_test_resolution(
            0,
            &format!("session-{i}"),
            "file.rs",
            "ai",
            "2025-02-18T12:00:00Z",
        );
        insert_conflict_resolution(&pool, &resolution)
            .await
            .expect("Insert should succeed");
    }

    let start = std::time::Instant::now();

    let resolutions = get_resolutions_by_decider(&pool, "ai")
        .await
        .expect("Query should succeed");

    let elapsed = start.elapsed();

    assert_eq!(resolutions.len(), 100);
    assert!(
        elapsed.as_millis() < 200,
        "Query should complete in < 200ms, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_performance_concurrent_inserts() {
    let pool = create_test_pool().await;
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Schema init should succeed");

    let start = std::time::Instant::now();

    // Spawn 50 concurrent insert tasks
    let mut handles = Vec::new();

    for i in 0..50 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let resolution = create_test_resolution(
                0,
                &format!("session-{i}"),
                "file.rs",
                if i % 2 == 0 { "ai" } else { "human" },
                "2025-02-18T12:00:00Z",
            );
            insert_conflict_resolution(&pool_clone, &resolution).await
        });
        handles.push(handle);
    }

    // Wait for all inserts
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(std::result::Result::ok)
        .collect();

    let elapsed = start.elapsed();

    // All inserts should succeed
    for result in results {
        assert!(result.is_ok(), "All concurrent inserts should succeed");
    }

    // Verify count
    let row = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .expect("Should count rows");

    let count: i64 = row.get("count");
    assert_eq!(count, 50, "Should have 50 records");

    // Should complete in reasonable time (< 5 seconds for 50 concurrent inserts)
    assert!(
        elapsed.as_secs() < 5,
        "Concurrent inserts should complete in < 5s, took {}s",
        elapsed.as_secs()
    );
}
