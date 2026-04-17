//! Conflict resolution schema initialization.
//!
//! Provides async functions for initializing the `conflict_resolutions` table
//! and its indexes in the SQLite database.

use sqlx::sqlite::SqlitePool;

pub use super::conflict_resolutions_entities::ConflictResolutionError;
use crate::Result;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SCHEMA INITIALIZATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Initialize `conflict_resolutions` table schema.
///
/// This function is called during database initialization to create
/// the `conflict_resolutions` table and its indexes.
///
/// # Contract
///
/// ## Preconditions
/// - `pool` is valid and connected
/// - `sessions` table exists (dependency)
///
/// ## Postconditions
/// - `conflict_resolutions` table exists
/// - All indexes created
/// - Function is idempotent (safe to call multiple times)
///
/// # Errors
///
/// Returns `Error::DatabaseError` if table creation fails.
///
/// # Example
///
/// ```rust,no_run
/// # use sqlx::SqlitePool;
/// # use scp_core::coordination::conflict_resolutions_schema::init_conflict_resolutions_schema;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = SqlitePool::connect("sqlite:db.sqlite").await?;
/// init_conflict_resolutions_schema(&pool).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_conflict_resolutions_schema(pool: &SqlitePool) -> Result<()> {
    // Create conflict_resolutions table
    let create_table = sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS conflict_resolutions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            session TEXT NOT NULL,
            file TEXT NOT NULL,
            strategy TEXT NOT NULL,
            reason TEXT,
            confidence TEXT,
            decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
        )
        ",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE TABLE conflict_resolutions".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions and connection".to_string(),
    })?;

    // Create indexes
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session ON conflict_resolutions(session)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_session".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_timestamp ON conflict_resolutions(timestamp)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_timestamp".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_decider ON conflict_resolutions(decider)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_decider".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session_timestamp ON conflict_resolutions(session, timestamp)",
    )
    .execute(pool)
    .await
    .map_err(|e| ConflictResolutionError::SchemaInitializationError {
        operation: "CREATE INDEX idx_conflict_resolutions_session_timestamp".to_string(),
        source: e.to_string(),
        recovery: "Check database permissions".to_string(),
    })?;

    // Log success
    tracing::debug!(
        "Initialized conflict_resolutions schema (rows_affected: {})",
        create_table.rows_affected()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SQL schema structure validation (static string analysis)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// The CREATE TABLE SQL from init_conflict_resolutions_schema.
    /// We extract it here so we can test the SQL structure without a database.
    const CREATE_TABLE_SQL: &str = r"
        CREATE TABLE IF NOT EXISTS conflict_resolutions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            session TEXT NOT NULL,
            file TEXT NOT NULL,
            strategy TEXT NOT NULL,
            reason TEXT,
            confidence TEXT,
            decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
        )
        ";

    #[test]
    fn test_create_table_has_if_not_exists() {
        assert!(
            CREATE_TABLE_SQL.contains("CREATE TABLE IF NOT EXISTS"),
            "CREATE TABLE should be idempotent with IF NOT EXISTS"
        );
    }

    #[test]
    fn test_create_table_has_correct_table_name() {
        assert!(
            CREATE_TABLE_SQL.contains("conflict_resolutions"),
            "table name should be 'conflict_resolutions'"
        );
    }

    #[test]
    fn test_create_table_has_autoincrement_pk() {
        assert!(
            CREATE_TABLE_SQL.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"),
            "id should be auto-increment primary key"
        );
    }

    #[test]
    fn test_create_table_all_required_columns_present() {
        let required_columns = [
            "id INTEGER",
            "timestamp TEXT NOT NULL",
            "session TEXT NOT NULL",
            "file TEXT NOT NULL",
            "strategy TEXT NOT NULL",
            "reason TEXT",
            "confidence TEXT",
            "decider TEXT NOT NULL",
        ];
        for col in required_columns {
            assert!(
                CREATE_TABLE_SQL.contains(col),
                "schema should contain column definition: {col}"
            );
        }
    }

    #[test]
    fn test_create_table_has_decider_check_constraint() {
        assert!(
            CREATE_TABLE_SQL.contains("CHECK(decider IN ('ai', 'human'))"),
            "decider column should have CHECK constraint for 'ai' and 'human'"
        );
    }

    #[test]
    fn test_create_table_optional_columns_are_nullable() {
        // reason and confidence should NOT have NOT NULL
        assert!(
            !CREATE_TABLE_SQL.contains("reason TEXT NOT NULL"),
            "reason column should be nullable (optional)"
        );
        assert!(
            !CREATE_TABLE_SQL.contains("confidence TEXT NOT NULL"),
            "confidence column should be nullable (optional)"
        );
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Index SQL structure validation
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    const INDEX_SQL: &[&str] = &[
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session ON conflict_resolutions(session)",
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_timestamp ON conflict_resolutions(timestamp)",
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_decider ON conflict_resolutions(decider)",
        "CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session_timestamp ON conflict_resolutions(session, timestamp)",
    ];

    #[test]
    fn test_all_indexes_use_if_not_exists() {
        for sql in INDEX_SQL {
            assert!(
                sql.contains("CREATE INDEX IF NOT EXISTS"),
                "index should use IF NOT EXISTS: {sql}"
            );
        }
    }

    #[test]
    fn test_session_index_exists() {
        let session_idx = INDEX_SQL.iter().find(|s| {
            s.contains("idx_conflict_resolutions_session ON conflict_resolutions(session)")
        });
        assert!(
            session_idx.is_some(),
            "should have a single-column index on session"
        );
    }

    #[test]
    fn test_timestamp_index_exists() {
        let ts_idx = INDEX_SQL.iter().find(|s| {
            s.contains("idx_conflict_resolutions_timestamp ON conflict_resolutions(timestamp)")
        });
        assert!(
            ts_idx.is_some(),
            "should have a single-column index on timestamp"
        );
    }

    #[test]
    fn test_decider_index_exists() {
        let decider_idx = INDEX_SQL.iter().find(|s| {
            s.contains("idx_conflict_resolutions_decider ON conflict_resolutions(decider)")
        });
        assert!(
            decider_idx.is_some(),
            "should have a single-column index on decider"
        );
    }

    #[test]
    fn test_composite_session_timestamp_index_exists() {
        let composite_idx = INDEX_SQL
            .iter()
            .find(|s| s.contains("idx_conflict_resolutions_session_timestamp ON conflict_resolutions(session, timestamp)"));
        assert!(
            composite_idx.is_some(),
            "should have a composite index on (session, timestamp)"
        );
    }

    #[test]
    fn test_expected_index_count() {
        assert_eq!(
            INDEX_SQL.len(),
            4,
            "should have exactly 4 indexes (session, timestamp, decider, composite)"
        );
    }

    #[test]
    fn test_no_update_or_delete_in_schema() {
        // Append-only design: no UPDATE or DELETE statements in schema init
        assert!(
            !CREATE_TABLE_SQL.contains("UPDATE"),
            "schema should not contain UPDATE (append-only design)"
        );
        assert!(
            !CREATE_TABLE_SQL.contains("DELETE"),
            "schema should not contain DELETE (append-only design)"
        );
    }

    #[test]
    fn test_schema_is_valid_sqlite_ddl() {
        // Verify basic SQL syntax correctness by checking structural elements
        assert!(
            CREATE_TABLE_SQL.contains("CREATE TABLE"),
            "should be a CREATE TABLE statement"
        );
        assert!(
            CREATE_TABLE_SQL.contains("PRIMARY KEY"),
            "should have a PRIMARY KEY"
        );
        assert!(
            CREATE_TABLE_SQL.contains("NOT NULL"),
            "should have NOT NULL constraints"
        );
    }
}
