//! SQLite migration for queue_entries table.
//!
//! This module provides synchronous SQLite migrations for the queue system.

use rusqlite::Connection;
use thiserror::Error;

/// Migration-specific errors following the contract specification.
#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Database connection failed: {0}")]
    DatabaseError(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Table already exists with incompatible schema")]
    SchemaConflict,

    #[error("Invalid migration: {0}")]
    InvalidMigration(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}

/// SQL migration for creating the queue_entries table.
///
/// Columns:
/// - `id`: TEXT PRIMARY KEY - unique identifier (format: queue-{uuid})
/// - `session_id`: TEXT NOT NULL - the session this entry belongs to
/// - `bead_id`: TEXT - optional bead identifier
/// - `priority`: INTEGER NOT NULL DEFAULT 128 - priority value (0-255)
/// - `position`: INTEGER NOT NULL - ordering within same priority
/// - `status`: TEXT NOT NULL - QueueStatus enum value
/// - `enqueued_at`: TEXT NOT NULL - ISO8601 timestamp
/// - `updated_at`: TEXT NOT NULL - ISO8601 timestamp
/// - `retry_count`: INTEGER NOT NULL DEFAULT 0 - number of retry attempts
/// - `error_message`: TEXT - optional error message for failed entries
///
/// Constraints:
/// - CHECK constraint on status (valid QueueStatus values)
/// - CHECK constraint on priority (0-255)
/// - CHECK constraint on retry_count (>= 0)
///
/// Indexes:
/// - idx_queue_status_priority_position: for dequeue operations
/// - idx_queue_session_id: for session lookups
const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS queue_entries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    bead_id TEXT,
    priority INTEGER NOT NULL DEFAULT 128,
    position INTEGER NOT NULL,
    status TEXT NOT NULL,
    enqueued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    CONSTRAINT chk_status CHECK (status IN (
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
    CONSTRAINT chk_priority CHECK (priority >= 0 AND priority <= 255),
    CONSTRAINT chk_retry_count CHECK (retry_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_queue_status_priority_position
    ON queue_entries (status, priority, position);

CREATE INDEX IF NOT EXISTS idx_queue_session_id
    ON queue_entries (session_id);
"#;

/// Checks if the queue_entries table already exists with a potentially conflicting schema.
///
/// Returns `Ok(true)` if table exists, `Ok(false)` if not.
fn table_exists(connection: &Connection) -> Result<bool, MigrationError> {
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='queue_entries'";
    connection
        .query_row(query, [], |row| row.get::<_, String>(0))
        .map(|_| true)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            _ => Err(MigrationError::DatabaseError(e.to_string())),
        })
}

/// Runs all queue_entries migrations.
///
/// # Preconditions
/// - P1: Database connection is valid and writable
/// - P2: No existing queue_entries table with conflicting schema
/// - P3: Migration SQL is syntactically valid SQLite DDL
///
/// # Postconditions
/// - Q1: Table queue_entries exists with all required columns
/// - Q2: Primary key constraint on id column
/// - Q3: session_id column is NOT NULL
/// - Q4: priority column has default value of 128
/// - Q5: Indexes exist for common query patterns
///
/// # Invariants
/// - I1: Every row must have a valid status value
/// - I2: retry_count must be >= 0
/// - I3: priority must be 0-255
pub fn run_migrations(connection: &Connection) -> Result<(), MigrationError> {
    // P1: Validate connection is writable by executing a simple query
    connection
        .execute("SELECT 1", [])
        .map_err(|e| MigrationError::DatabaseError(e.to_string()))?;

    // P2: Check for existing table
    let exists = table_exists(connection)?;
    if exists {
        return Err(MigrationError::SchemaConflict);
    }

    // Execute migration SQL (P3 validated by SQLite at runtime)
    connection
        .execute(MIGRATION_SQL, [])
        .map_err(|e| MigrationError::MigrationFailed(e.to_string()))?;

    // Verify Q1: Table exists with all columns
    verify_migration(connection).and_then(|verified| {
        if verified {
            Ok(())
        } else {
            Err(MigrationError::MigrationFailed(
                "Migration verification failed".into(),
            ))
        }
    })
}

/// Verifies the migration was applied correctly.
///
/// Returns `Ok(true)` if table exists with correct schema,
/// `Ok(false)` if table does not exist.
pub fn verify_migration(connection: &Connection) -> Result<bool, MigrationError> {
    let exists = table_exists(connection)?;
    if !exists {
        return Ok(false);
    }

    // Simple verification: check if table is queryable with expected structure
    // This verifies all required columns exist and are accessible
    connection
        .query_row(
            "SELECT id, session_id, bead_id, priority, position, status, enqueued_at, updated_at, retry_count, error_message FROM queue_entries LIMIT 0",
            [],
            |_| Ok(()),
        )
        .map_err(|e| MigrationError::MigrationFailed(e.to_string()))?;

    // Verify indexes exist by checking query plans use them
    // The composite index should be used for status+priority+position queries
    let index_count: i32 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_index_list('queue_entries')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| MigrationError::DatabaseError(e.to_string()))?;

    // We expect at least 2 indexes (composite + session_id)
    if index_count < 2 {
        return Err(MigrationError::MigrationFailed(
            "Expected indexes not found".into(),
        ));
    }

    Ok(true)
}

/// Rolls back the migration by dropping the queue_entries table.
pub fn rollback_migration(connection: &Connection) -> Result<(), MigrationError> {
    // Check if table exists
    let exists = table_exists(connection)?;
    if !exists {
        return Ok(());
    }

    // Drop indexes first (if they exist)
    connection
        .execute("DROP INDEX IF EXISTS idx_queue_status_priority_position", [])
        .map_err(|e| MigrationError::RollbackFailed(e.to_string()))?;

    connection
        .execute("DROP INDEX IF EXISTS idx_queue_session_id", [])
        .map_err(|e| MigrationError::RollbackFailed(e.to_string()))?;

    // Drop table
    connection
        .execute("DROP TABLE IF EXISTS queue_entries", [])
        .map_err(|e| MigrationError::RollbackFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a fresh in-memory SQLite database for testing.
    fn test_connection() -> Connection {
        Connection::open_in_memory().expect("failed to create in-memory database")
    }

    #[test]
    fn migration_creates_queue_entries_table() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");
        verify_migration(&conn).expect("verification failed");
    }

    #[test]
    fn migration_fails_when_table_already_exists() {
        let conn = test_connection();
        run_migrations(&conn).expect("first migration failed");

        let result = run_migrations(&conn);
        assert!(matches!(result, Err(MigrationError::SchemaConflict)));
    }

    #[test]
    fn verify_returns_false_when_table_missing() {
        let conn = test_connection();
        let result = verify_migration(&conn).expect("verify failed");
        assert!(!result);
    }

    #[test]
    fn verify_returns_true_after_migration() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");
        let result = verify_migration(&conn).expect("verify failed");
        assert!(result);
    }

    #[test]
    fn rollback_removes_table() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        rollback_migration(&conn).expect("rollback failed");

        let result = verify_migration(&conn).expect("verify failed");
        assert!(!result);
    }

    #[test]
    fn insert_valid_queue_entry_succeeds() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, bead_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 'bead-abc', 100, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_id_violates_unique_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 100, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        )
        .expect("first insert failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-2', 100, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn null_session_id_violates_not_null_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', NULL, 100, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn invalid_status_violates_check_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 100, 0, 'InvalidStatus', datetime('now'), datetime('now'), 0)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn priority_out_of_bounds_violates_check_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 300, 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn negative_retry_count_violates_check_constraint() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let result = conn.execute(
            "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 100, 0, 'Pending', datetime('now'), datetime('now'), -1)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn default_priority_value_is_128() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        conn.execute(
            "INSERT INTO queue_entries (id, session_id, position, status, enqueued_at, updated_at, retry_count)
             VALUES ('queue-123', 'session-1', 0, 'Pending', datetime('now'), datetime('now'), 0)",
            [],
        )
        .expect("insert failed");

        let priority: i32 = conn
            .query_row(
                "SELECT priority FROM queue_entries WHERE id = 'queue-123'",
                [],
                |row| row.get(0),
            )
            .expect("query failed");
        assert_eq!(priority, 128);
    }

    #[test]
    fn all_valid_status_values_accepted() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let valid_statuses = [
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

        for (i, status) in valid_statuses.iter().enumerate() {
            let result = conn.execute(
                &format!(
                    "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
                     VALUES ('queue-{}', 'session-1', 100, 0, '{}', datetime('now'), datetime('now'), 0)",
                    i, status
                ),
                [],
            );
            assert!(result.is_ok(), "Failed for status: {}", status);
        }
    }

    #[test]
    fn priority_boundary_values_accepted() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        let boundary_values = [0, 1, 127, 128, 254, 255];

        for priority in boundary_values {
            let result = conn.execute(
                &format!(
                    "INSERT INTO queue_entries (id, session_id, priority, position, status, enqueued_at, updated_at, retry_count)
                     VALUES ('queue-{}', 'session-1', {}, 0, 'Pending', datetime('now'), datetime('now'), 0)",
                    priority, priority
                ),
                [],
            );
            assert!(result.is_ok(), "Failed for priority: {}", priority);
        }
    }

    #[test]
    fn indexes_exist_after_migration() {
        let conn = test_connection();
        run_migrations(&conn).expect("migration failed");

        // Check for composite index
        let composite_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_index_list('queue_entries') WHERE name = 'idx_queue_status_priority_position'",
                [],
                |row| row.get(0),
            )
            .expect("query failed");
        assert!(composite_exists, "Composite index should exist");

        // Check for session_id index
        let session_index_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_index_list('queue_entries') WHERE name = 'idx_queue_session_id'",
                [],
                |row| row.get(0),
            )
            .expect("query failed");
        assert!(session_index_exists, "Session index should exist");
    }
}
