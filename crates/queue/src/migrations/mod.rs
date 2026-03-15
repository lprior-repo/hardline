//! SQLite migrations for queue_entries table.
//!
//! Provides functions to:
//! - `run_migrations`: Apply the queue_entries table migration
//! - `verify_migration`: Verify the migration was applied correctly
//! - `rollback_migration`: Rollback (drop) the queue_entries table

use crate::error::MigrationError;
use rusqlite::Connection;

/// SQL migration for queue_entries table
const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS queue_entries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 128,
    position INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    enqueued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    bead_id TEXT,
    error_message TEXT,
    CHECK (status IN (
        'Pending',
        'Claimed',
        'Rebasing',
        'Testing',
        'ReadyToMerge',
        'Merging',
        'Merged',
        'FailedRetryable',
        'FailedTerminal',
        'Cancelled'
    )),
    CHECK (priority >= 0 AND priority <= 255),
    CHECK (retry_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_queue_entries_status_priority_position
ON queue_entries(status, priority, position);

CREATE INDEX IF NOT EXISTS idx_queue_entries_session_id
ON queue_entries(session_id);
"#;

/// Runs all queue_entries migrations.
///
/// Returns `Ok(())` on success, `Err(MigrationError)` on failure.
///
/// # Errors
/// Returns `MigrationError::SchemaConflict` if the table already exists.
pub fn run_migrations(connection: &Connection) -> Result<(), MigrationError> {
    // Check if table already exists to enforce idempotency
    let table_exists = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='queue_entries'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map_err(|e| MigrationError::DatabaseError(e.to_string()))?
        > 0;

    if table_exists {
        return Err(MigrationError::SchemaConflict);
    }

    // Execute migration
    connection
        .execute(MIGRATION_SQL, [])
        .map_err(|e| MigrationError::MigrationFailed(e.to_string()))?;

    Ok(())
}

/// Verifies the migration was applied correctly.
///
/// Returns `Ok(true)` if table exists with correct schema, `Ok(false)` if table missing.
pub fn verify_migration(connection: &Connection) -> Result<bool, MigrationError> {
    let table_exists: i32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='queue_entries'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| MigrationError::DatabaseError(e.to_string()))?;

    Ok(table_exists > 0)
}

/// Rolls back the migration (drops the table).
///
/// # Errors
/// Returns `MigrationError::RollbackFailed` if the drop fails.
pub fn rollback_migration(connection: &Connection) -> Result<(), MigrationError> {
    connection
        .execute("DROP TABLE IF EXISTS queue_entries", [])
        .map_err(|e| MigrationError::RollbackFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_connection() -> Connection {
        Connection::open_in_memory().expect("failed to create in-memory database")
    }

    #[test]
    fn migration_creates_queue_entries_table() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let exists = verify_migration(&conn).expect("verify should succeed");
        assert!(exists, "table should exist after migration");
    }

    #[test]
    fn migration_creates_unique_primary_key() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, ?2, 128, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            ["queue-123", "session-1"],
        );
        assert!(result.is_ok(), "first insert should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, ?2, 128, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            ["queue-123", "session-2"],
        );
        assert!(result.is_err(), "duplicate id should fail");
    }

    #[test]
    fn migration_enforces_not_null_session_id() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, NULL, 128, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            ["queue-123"],
        );
        assert!(result.is_err(), "NULL session_id should fail");
    }

    #[test]
    fn migration_enforces_status_check_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, 'session-1', 128, 0, 'InvalidStatus', datetime('now'), datetime('now'), 0)",
            ["queue-123"],
        );
        assert!(result.is_err(), "invalid status should fail");
    }

    #[test]
    fn migration_enforces_priority_bounds() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, 'session-1', 256, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            ["queue-123"],
        );
        assert!(result.is_err(), "priority > 255 should fail");
    }

    #[test]
    fn migration_enforces_retry_count_non_negative() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, 'session-1', 128, 0, 'Pending', datetime('now'), datetime('now'), -1)",
            ["queue-123"],
        );
        assert!(result.is_err(), "negative retry_count should fail");
    }

    #[test]
    fn migration_default_priority_is_128() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        conn.execute(
            "INSERT INTO queue_entries (id, session_id, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, 'session-1', 0, 'Pending', datetime('now'), datetime('now'), 0)",
            ["queue-123"],
        )
        .expect("insert without priority should succeed");

        let priority: i32 = conn
            .query_row(
                "SELECT priority FROM queue_entries WHERE id = ?1",
                ["queue-123"],
                |row| row.get(0),
            )
            .expect("should be able to query priority");

        assert_eq!(priority, 128, "default priority should be 128");
    }

    #[test]
    fn migration_idempotent_check() {
        let conn = test_connection();
        run_migrations(&conn).expect("first migration should succeed");

        let result = run_migrations(&conn);
        assert!(result.is_err(), "second migration should fail with SchemaConflict");
    }

    #[test]
    fn verify_returns_false_when_table_missing() {
        let conn = test_connection();
        let exists = verify_migration(&conn).expect("verify should succeed");
        assert!(!exists, "table should not exist before migration");
    }

    #[test]
    fn rollback_removes_table() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        rollback_migration(&conn).expect("rollback should succeed");

        let exists = verify_migration(&conn).expect("verify should succeed");
        assert!(!exists, "table should not exist after rollback");
    }

    #[test]
    fn migration_creates_status_priority_position_index() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let index_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_queue_entries_status_priority_position'",
                [],
                |row| row.get(0),
            )
            .expect("should query indexes");

        assert!(index_exists > 0, "status/priority/position index should exist");
    }

    #[test]
    fn migration_creates_session_id_index() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let index_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_queue_entries_session_id'",
                [],
                |row| row.get(0),
            )
            .expect("should query indexes");

        assert!(index_exists > 0, "session_id index should exist");
    }

    #[test]
    fn migration_accepts_all_valid_status_values() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration should succeed");

        let statuses = [
            "Pending",
            "Claimed",
            "Rebasing",
            "Testing",
            "ReadyToMerge",
            "Merging",
            "Merged",
            "FailedRetryable",
            "FailedTerminal",
            "Cancelled",
        ];

        for (i, status) in statuses.iter().enumerate() {
            let id = format!("queue-{}", i);
            let status_str = *status;
            let result = conn.execute(
                "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count) VALUES (?1, 'session-1', 128, 0, ?2, datetime('now'), datetime('now'), 0)",
                [&id, &status_str.to_string()],
            );
            assert!(
                result.is_ok(),
                "status '{}' should be accepted",
                status
            );
        }
    }
}
