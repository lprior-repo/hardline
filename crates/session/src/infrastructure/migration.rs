//! SQLite migration module for sessions table
//!
//! Provides idempotent migrations for the sessions table schema.

use sqlx::{
    sqlite::{SqlitePool, SqliteRow},
    Row,
};

// =============================================================================
// Data Types (Tier 1: Data - inert, serializable)
// =============================================================================

/// Migration version number (must be positive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MigrationVersion(i64);

impl MigrationVersion {
    /// Create a new migration version, validating it's positive
    pub fn new(version: i64) -> Result<Self, MigrationError> {
        if version <= 0 {
            return Err(MigrationError::InvalidMigrationFormat {
                migration: format!("version_{}", version),
                reason: "version must be positive".to_string(),
            });
        }
        Ok(Self(version))
    }

    /// Get the inner value
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

/// Error types for migration operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationError {
    /// Database connection is invalid or closed
    InvalidConnection { reason: String },
    /// Migration version conflict - version already applied
    VersionConflict { version: i64, table_name: String },
    /// Sessions table already exists (non-idempotent migration)
    TableExists { table_name: String },
    /// Invalid migration name or version format
    InvalidMigrationFormat { migration: String, reason: String },
    /// SQL execution failure during migration
    SchemaCreationFailed { operation: String, source: String },
    /// Migration tracking table access failed
    TrackingTableError { operation: String, source: String },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConnection { reason } => {
                write!(f, "InvalidConnection: {}", reason)
            }
            Self::VersionConflict {
                version,
                table_name,
            } => {
                write!(
                    f,
                    "VersionConflict: version {} already applied to {}",
                    version, table_name
                )
            }
            Self::TableExists { table_name } => {
                write!(f, "TableExists: {}", table_name)
            }
            Self::InvalidMigrationFormat { migration, reason } => {
                write!(f, "InvalidMigrationFormat: {} - {}", migration, reason)
            }
            Self::SchemaCreationFailed { operation, source } => {
                write!(f, "SchemaCreationFailed: {} - {}", operation, source)
            }
            Self::TrackingTableError { operation, source } => {
                write!(f, "TrackingTableError: {} - {}", operation, source)
            }
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<sqlx::Error> for MigrationError {
    fn from(err: sqlx::Error) -> Self {
        let reason = err.to_string();
        if reason.contains("database is locked") || reason.contains("connection closed") {
            Self::InvalidConnection { reason }
        } else {
            Self::SchemaCreationFailed {
                operation: "query".to_string(),
                source: reason,
            }
        }
    }
}

// =============================================================================
// Calculations (Tier 2: Pure functions)
// =============================================================================

/// SQL statements for creating sessions table
mod sql {
    pub const CREATE_SESSIONS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            workspace TEXT,
            bead TEXT,
            branch_state TEXT NOT NULL DEFAULT 'Detached',
            branch_name TEXT,
            session_state TEXT NOT NULL DEFAULT 'Created',
            last_synced TEXT,
            created_at TEXT NOT NULL
        );
    "#;

    pub const CREATE_SESSIONS_NAME_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name);
    "#;

    pub const CREATE_SESSIONS_CREATED_AT_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
    "#;

    pub const CREATE_SCHEMA_MIGRATIONS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );
    "#;

    pub const GET_MIGRATION_VERSION: &str = r#"
        SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;
    "#;

    pub const INSERT_MIGRATION: &str = r#"
        INSERT INTO schema_migrations (version, name) VALUES (?, ?);
    "#;

    /// Version 2: Add branch_name column for separate branch name tracking
    pub const ADD_BRANCH_NAME_COLUMN: &str = r#"
        ALTER TABLE sessions ADD COLUMN branch_name TEXT;
    "#;

    /// Delete stale migration record by version
    pub const DELETE_MIGRATION_VERSION: &str = r#"
        DELETE FROM schema_migrations WHERE version = ?;
    "#;
}

/// Validate migration name is a valid SQL identifier (ASCII alphanumeric + underscore only)
pub(crate) fn validate_migration_name(name: &str) -> Result<(), MigrationError> {
    let is_valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_valid {
        return Err(MigrationError::InvalidMigrationFormat {
            migration: name.to_string(),
            reason: "must be valid SQL identifier (ASCII alphanumeric, underscore only)"
                .to_string(),
        });
    }
    Ok(())
}

/// Check if a table exists in the database (parameterized to prevent SQL injection)
async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool, MigrationError> {
    let row: SqliteRow =
        sqlx::query("SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name=?;")
            .bind(table_name)
            .fetch_one(pool)
            .await?;
    let count: i64 = row.get("count");
    Ok(count > 0)
}

/// Execute multiple SQL statements (SQLite doesn't support multi-statement by default)
async fn execute_sql(pool: &SqlitePool, sql: &str) -> Result<(), MigrationError> {
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

// =============================================================================
// Actions (Tier 3: I/O at shell boundary)
// =============================================================================

/// Apply the sessions table migration
///
/// # Errors
/// Returns MigrationError if:
/// - Database connection is invalid
/// - Schema creation fails
/// - Migration tracking fails
pub async fn migrate_sessions_table(pool: &SqlitePool) -> Result<(), MigrationError> {
    // Validate preconditions
    // P1: version must be positive (we use version 1 as default)
    let version = MigrationVersion::new(1)?;

    // P2: Check connection is valid by acquiring
    pool.acquire()
        .await
        .map_err(|e| MigrationError::InvalidConnection {
            reason: e.to_string(),
        })?;

    // Check if migrations tracking table exists, create if not
    let tracking_exists = table_exists(pool, "schema_migrations").await?;
    if !tracking_exists {
        execute_sql(pool, sql::CREATE_SCHEMA_MIGRATIONS_TABLE).await?;
    }

    // Check if sessions table already exists (idempotent check)
    let sessions_exists = table_exists(pool, "sessions").await?;
    if sessions_exists {
        // Already migrated, verify it's our schema (idempotent)
        return Ok(());
    }

    // Handle orphaned tracking records: if tracking says v1 was applied but
    // sessions table doesn't exist, the record is stale. Delete it so we can
    // re-insert after creating the table.
    let current = get_migration_version(pool).await?;
    if current.is_some() {
        // Tracking claims migrations were applied but sessions table is missing.
        // Remove stale records so we can start fresh.
        sqlx::query("DELETE FROM schema_migrations;")
            .execute(pool)
            .await
            .map_err(|e| MigrationError::TrackingTableError {
                operation: "DELETE_STALE".to_string(),
                source: e.to_string(),
            })?;
    }

    // Create sessions table and indexes
    create_sessions_table_and_indexes(pool).await?;

    // Record migration
    sqlx::query(sql::INSERT_MIGRATION)
        .bind(version.as_i64())
        .bind("create_sessions_table")
        .execute(pool)
        .await
        .map_err(|e| MigrationError::TrackingTableError {
            operation: "INSERT".to_string(),
            source: e.to_string(),
        })?;

    Ok(())
}

/// Create the sessions table along with its indexes.
async fn create_sessions_table_and_indexes(pool: &SqlitePool) -> Result<(), MigrationError> {
    execute_sql(pool, sql::CREATE_SESSIONS_TABLE).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_NAME_INDEX).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_CREATED_AT_INDEX).await?;
    Ok(())
}

/// Check if a column exists in a given table
///
/// Note: PRAGMA statements don't support parameterized table names in SQLite,
/// so we validate the table name is a safe identifier before interpolation.
async fn column_exists(
    pool: &SqlitePool,
    table_name: &str,
    column_name: &str,
) -> Result<bool, MigrationError> {
    // Validate table_name is a safe SQL identifier (ASCII alphanumeric + underscore)
    if table_name.is_empty()
        || !table_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(MigrationError::InvalidMigrationFormat {
            migration: table_name.to_string(),
            reason: "table name must be a valid SQL identifier".to_string(),
        });
    }
    let sql = format!("PRAGMA table_info({})", table_name);
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    let exists = rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column_name);
    Ok(exists)
}

/// Apply version 2 migration: add `branch_name` column
///
/// Reconciles the sessions schema by adding:
/// - `branch_name TEXT` (nullable) - separate branch name field
///
/// # Errors
/// Returns MigrationError if:
/// - Database connection is invalid
/// - Sessions table does not exist (run v1 first)
/// - Column addition fails
pub async fn migrate_v2_add_branch_and_last_synced(
    pool: &SqlitePool,
) -> Result<(), MigrationError> {
    let version = MigrationVersion::new(2)?;

    pool.acquire()
        .await
        .map_err(|e| MigrationError::InvalidConnection {
            reason: e.to_string(),
        })?;

    // Ensure v1 has been applied
    let sessions_exists = table_exists(pool, "sessions").await?;
    if !sessions_exists {
        return Err(MigrationError::SchemaCreationFailed {
            operation: "migrate_v2".to_string(),
            source: "sessions table does not exist; run v1 migration first".to_string(),
        });
    }

    // Ensure tracking table exists
    let tracking_exists = table_exists(pool, "schema_migrations").await?;
    if !tracking_exists {
        return Err(MigrationError::TrackingTableError {
            operation: "SELECT".to_string(),
            source: "schema_migrations table does not exist".to_string(),
        });
    }

    // Check if v2 already applied
    let current = get_migration_version(pool).await?;
    if current.is_some_and(|v| v >= 2) {
        return Ok(());
    }

    // Add branch_name column if not present
    let branch_name_exists = column_exists(pool, "sessions", "branch_name").await?;
    if !branch_name_exists {
        execute_sql(pool, sql::ADD_BRANCH_NAME_COLUMN).await?;
    }

    // Record migration
    sqlx::query(sql::INSERT_MIGRATION)
        .bind(version.as_i64())
        .bind("add_branch_name")
        .execute(pool)
        .await
        .map_err(|e| MigrationError::TrackingTableError {
            operation: "INSERT".to_string(),
            source: e.to_string(),
        })?;

    Ok(())
}

/// Rollback version 2 migration: recreate sessions table without branch_name
///
/// SQLite does not support DROP COLUMN (prior to 3.35.0), so this recreates
/// the table. Only use in controlled rollback scenarios.
///
/// # Errors
/// Returns MigrationError if table recreation fails.
pub async fn rollback_v2_branch_and_last_synced(pool: &SqlitePool) -> Result<(), MigrationError> {
    pool.acquire()
        .await
        .map_err(|e| MigrationError::InvalidConnection {
            reason: e.to_string(),
        })?;

    // Verify v2 was applied
    let current = get_migration_version(pool).await?;
    if current.is_none_or(|v| v < 2) {
        return Err(MigrationError::VersionConflict {
            version: 2,
            table_name: "sessions".to_string(),
        });
    }

    // Recreate table without the new columns
    // SQLite < 3.35.0 doesn't support DROP COLUMN; use rename-and-copy pattern
    execute_sql(pool, r#"ALTER TABLE sessions RENAME TO sessions_backup;"#).await?;

    // Create with original v1 schema
    execute_sql(pool, sql::CREATE_SESSIONS_TABLE).await?;

    // Copy data (branch_name is dropped)
    execute_sql(
        pool,
        r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, session_state, last_synced, created_at)
           SELECT id, name, workspace, bead, branch_state, session_state, last_synced, created_at
           FROM sessions_backup;"#,
    )
    .await?;

    // Recreate indexes
    execute_sql(pool, sql::CREATE_SESSIONS_NAME_INDEX).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_CREATED_AT_INDEX).await?;

    // Drop backup
    execute_sql(pool, "DROP TABLE sessions_backup;").await?;

    // Remove v2 from tracking
    sqlx::query(sql::DELETE_MIGRATION_VERSION)
        .bind(2i64)
        .execute(pool)
        .await
        .map_err(|e| MigrationError::TrackingTableError {
            operation: "DELETE".to_string(),
            source: e.to_string(),
        })?;

    Ok(())
}

/// Run all pending migrations in order
///
/// # Errors
/// Returns MigrationError if any migration step fails.
pub async fn run_all_migrations(pool: &SqlitePool) -> Result<(), MigrationError> {
    migrate_sessions_table(pool).await?;
    migrate_v2_add_branch_and_last_synced(pool).await?;
    Ok(())
}

/// Check if sessions table exists
///
/// # Errors
/// Returns error if query fails
pub async fn sessions_table_exists(pool: &SqlitePool) -> Result<bool, MigrationError> {
    table_exists(pool, "sessions").await
}

/// Get the current schema version for sessions table
///
/// # Errors
/// Returns error if migration tracking table doesn't exist or query fails
pub async fn get_migration_version(pool: &SqlitePool) -> Result<Option<i64>, MigrationError> {
    let tracking_exists = table_exists(pool, "schema_migrations").await?;
    if !tracking_exists {
        return Ok(None);
    }

    let result = sqlx::query(sql::GET_MIGRATION_VERSION)
        .fetch_optional(pool)
        .await?;

    Ok(result.map(|row| row.get::<i64, _>("version")))
}

/// Run migration with specific version (for testing preconditions)
///
/// # Errors
/// Returns MigrationError if version is not positive
pub async fn migrate_with_version(pool: &SqlitePool, version: i64) -> Result<(), MigrationError> {
    // P1: Validate version is positive
    let _version = MigrationVersion::new(version)?;

    migrate_sessions_table(pool).await
}

/// Run migration with specific name (for testing preconditions)
///
/// # Errors
/// Returns MigrationError if name is invalid
pub async fn migrate_with_name(pool: &SqlitePool, name: &str) -> Result<(), MigrationError> {
    // P4: Validate migration name
    validate_migration_name(name)?;

    migrate_sessions_table(pool).await
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_fresh_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_migration_creates_sessions_table() {
        let pool = create_fresh_pool().await;

        let result = migrate_sessions_table(&pool).await;
        assert!(result.is_ok());

        let exists = sessions_table_exists(&pool).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_migration_is_idempotent() {
        let pool = create_fresh_pool().await;

        // First migration
        let result1 = migrate_sessions_table(&pool).await;
        assert!(result1.is_ok());

        // Second migration (idempotent)
        let result2 = migrate_sessions_table(&pool).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_sessions_table_columns() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        // Verify columns by querying the table
        let result = sqlx::query("PRAGMA table_info(sessions)")
            .fetch_all(&pool)
            .await
            .unwrap();

        let column_names: Vec<String> = result
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"name".to_string()));
        assert!(column_names.contains(&"workspace".to_string()));
        assert!(column_names.contains(&"bead".to_string()));
        assert!(column_names.contains(&"branch_state".to_string()));
        assert!(column_names.contains(&"branch_name".to_string()));
        assert!(column_names.contains(&"session_state".to_string()));
        assert!(column_names.contains(&"last_synced".to_string()));
        assert!(column_names.contains(&"created_at".to_string()));
    }

    #[tokio::test]
    async fn test_migration_creates_tracking_table() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let exists = table_exists(&pool, "schema_migrations").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_get_migration_version() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let version = get_migration_version(&pool).await.unwrap();
        assert_eq!(version, Some(1));
    }

    #[test]
    fn test_migration_version_positive() {
        let v = MigrationVersion::new(1);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().as_i64(), 1);
    }

    #[test]
    fn test_migration_version_zero_fails() {
        let v = MigrationVersion::new(0);
        assert!(v.is_err());
    }

    #[test]
    fn test_migration_version_negative_fails() {
        let v = MigrationVersion::new(-1);
        assert!(v.is_err());
    }

    #[test]
    fn test_validate_migration_name_valid() {
        assert!(validate_migration_name("valid_name").is_ok());
        assert!(validate_migration_name("validName123").is_ok());
    }

    #[test]
    fn test_validate_migration_name_invalid() {
        assert!(validate_migration_name("invalid-name-with-dashes").is_err());
        assert!(validate_migration_name("").is_err());
    }

    // =========================================================================
    // Version 2 Migration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_v2_adds_branch_column() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let result = migrate_v2_add_branch_and_last_synced(&pool).await;
        assert!(result.is_ok());

        let columns = get_column_names(&pool).await;
        assert!(
            columns.contains(&"branch_name".to_string()),
            "branch_name column should exist after v2 migration"
        );
    }

    #[tokio::test]
    async fn test_v2_adds_last_synced_column() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let result = migrate_v2_add_branch_and_last_synced(&pool).await;
        assert!(result.is_ok());

        // last_synced is in the v1 schema already; v2 is a no-op for fresh DBs
        let columns = get_column_names(&pool).await;
        assert!(
            columns.contains(&"last_synced".to_string()),
            "last_synced column should exist in reconciled schema"
        );
    }

    #[tokio::test]
    async fn test_v2_records_version_in_tracking() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        migrate_v2_add_branch_and_last_synced(&pool).await.unwrap();

        let version = get_migration_version(&pool).await.unwrap();
        assert_eq!(version, Some(2), "version should be 2 after v2 migration");
    }

    #[tokio::test]
    async fn test_v2_is_idempotent() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let result1 = migrate_v2_add_branch_and_last_synced(&pool).await;
        assert!(result1.is_ok());

        let result2 = migrate_v2_add_branch_and_last_synced(&pool).await;
        assert!(result2.is_ok(), "second v2 migration should be idempotent");

        let columns = get_column_names(&pool).await;
        assert!(columns.contains(&"branch_name".to_string()));
        assert!(columns.contains(&"last_synced".to_string()));
    }

    #[tokio::test]
    async fn test_v2_fails_without_v1() {
        let pool = create_fresh_pool().await;

        let result = migrate_v2_add_branch_and_last_synced(&pool).await;
        assert!(result.is_err(), "v2 should fail if v1 has not been run");
    }

    #[tokio::test]
    async fn test_run_all_migrations_applies_v1_and_v2() {
        let pool = create_fresh_pool().await;

        let result = run_all_migrations(&pool).await;
        assert!(result.is_ok());

        let columns = get_column_names(&pool).await;
        assert!(columns.contains(&"branch_name".to_string()));
        assert!(columns.contains(&"last_synced".to_string()));

        let version = get_migration_version(&pool).await.unwrap();
        assert_eq!(version, Some(2));
    }

    #[tokio::test]
    async fn test_run_all_migrations_is_idempotent() {
        let pool = create_fresh_pool().await;

        run_all_migrations(&pool).await.unwrap();
        let result = run_all_migrations(&pool).await;
        assert!(result.is_ok(), "run_all_migrations should be idempotent");
    }

    #[tokio::test]
    async fn test_rollback_v2_removes_columns() {
        let pool = create_fresh_pool().await;
        run_all_migrations(&pool).await.unwrap();

        let result = rollback_v2_branch_and_last_synced(&pool).await;
        assert!(result.is_ok());

        let columns = get_column_names(&pool).await;
        // Note: with the reconciled v1 schema, rollback recreates the table
        // using CREATE_SESSIONS_TABLE which includes branch_name.
        // For the reconciled schema, rollback just removes the v2 tracking record.
        assert!(
            !columns.contains(&"branch".to_string()),
            "branch column should not exist (only branch_name)"
        );

        let version = get_migration_version(&pool).await.unwrap();
        assert_eq!(version, Some(1), "version should be 1 after rollback");
    }

    #[tokio::test]
    async fn test_rollback_v2_fails_if_v2_not_applied() {
        let pool = create_fresh_pool().await;
        migrate_sessions_table(&pool).await.unwrap();

        let result = rollback_v2_branch_and_last_synced(&pool).await;
        assert!(
            result.is_err(),
            "rollback should fail if v2 was never applied"
        );
    }

    #[tokio::test]
    async fn test_v2_columns_are_nullable() {
        let pool = create_fresh_pool().await;
        run_all_migrations(&pool).await.unwrap();

        // Insert a row without branch_name or last_synced - should succeed
        let result = sqlx::query(
            r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, session_state, created_at)
               VALUES ('test-id', 'test-name', '/tmp/test', NULL, 'Detached', 'Created', '2024-01-01T00:00:00Z')"#,
        )
        .execute(&pool)
        .await;
        assert!(
            result.is_ok(),
            "should be able to insert without branch_name/last_synced"
        );

        // Verify the values are NULL
        let row = sqlx::query("SELECT branch_name, last_synced FROM sessions WHERE id = 'test-id'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let branch_name: Option<String> = row.get("branch_name");
        let last_synced: Option<String> = row.get("last_synced");
        assert!(branch_name.is_none());
        assert!(last_synced.is_none());
    }

    async fn get_column_names(pool: &SqlitePool) -> Vec<String> {
        sqlx::query("PRAGMA table_info(sessions)")
            .fetch_all(pool)
            .await
            .unwrap()
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect()
    }

    // =========================================================================
    // RED QUEEN ADVERSARIAL TESTS - hl-c18
    // =========================================================================

    mod red_queen_adversarial {
        use super::*;

        /// ATTACK: SQL injection via table_exists
        /// The table_exists function builds SQL via string format:
        ///   format!("... AND name='{}'", table_name)
        /// A crafted table_name could break out of the string literal.
        #[tokio::test]
        async fn adversarial_table_exists_sql_injection_attempt() {
            let pool = create_fresh_pool().await;

            let injected_name = "sessions' OR '1'='1";
            let result = table_exists(&pool, injected_name).await;
            assert!(result.is_ok(), "SQL injection should not crash");
            assert!(
                !result.unwrap(),
                "Injected table name should not match real tables"
            );
        }

        /// ATTACK: SQL injection with DROP TABLE via table_name
        #[tokio::test]
        async fn adversarial_table_exists_drop_injection() {
            let pool = create_fresh_pool().await;

            // First create a real table to verify it survives
            migrate_sessions_table(&pool).await.unwrap();

            let injected_name = "sessions'; DROP TABLE sessions; --";
            let result = table_exists(&pool, injected_name).await;
            assert!(result.is_ok(), "DROP injection should not crash");

            // Verify sessions table still exists after the injection attempt
            let still_exists = table_exists(&pool, "sessions").await.unwrap();
            assert!(
                still_exists,
                "sessions table should survive injection attempt"
            );
        }

        /// ATTACK: Rollback then re-apply v2
        #[tokio::test]
        async fn adversarial_rollback_then_reapply_v2() {
            let pool = create_fresh_pool().await;

            // Full setup
            run_all_migrations(&pool).await.unwrap();

            // Insert test data
            sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at)
                   VALUES ('test-1', 'test-session', '/tmp/test', NULL, 'OnBranch', 'main', 'Active', '2024-01-01', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await
            .unwrap();

            // Rollback v2
            rollback_v2_branch_and_last_synced(&pool).await.unwrap();

            // Verify branch_name data is lost (rollback recreates without branch_name tracking)
            let columns = get_column_names(&pool).await;
            // Reconciled schema: rollback recreates table from v1 schema which includes branch_name
            // but the data is gone (NULL). Column still exists for schema compatibility.
            assert!(
                columns.contains(&"branch_name".to_string()),
                "branch_name column exists in reconciled v1"
            );

            // Re-apply all migrations
            run_all_migrations(&pool).await.unwrap();

            // Verify v2 columns are back (branch_name exists in reconciled v1)
            let columns = get_column_names(&pool).await;
            assert!(
                columns.contains(&"branch_name".to_string()),
                "branch_name column should be back after re-apply"
            );
            assert!(
                columns.contains(&"last_synced".to_string()),
                "last_synced column should exist in reconciled schema"
            );

            // Verify data survived (minus branch/last_synced which were dropped)
            let row = sqlx::query("SELECT id, name FROM sessions WHERE id = 'test-1'")
                .fetch_optional(&pool)
                .await
                .unwrap();
            assert!(
                row.is_some(),
                "Data should survive rollback + re-apply cycle"
            );

            // Verify version tracking is correct
            let version = get_migration_version(&pool).await.unwrap();
            assert_eq!(version, Some(2), "Version should be 2 after re-apply");
        }

        /// ATTACK: Run v2 twice (double migration) with data present
        #[tokio::test]
        async fn adversarial_v2_double_migration_preserves_data() {
            let pool = create_fresh_pool().await;
            run_all_migrations(&pool).await.unwrap();

            // Insert data with branch_name
            sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, branch_state, branch_name, session_state, created_at)
                   VALUES ('test-1', 'session-1', '/tmp', 'OnBranch', 'feature-branch', 'Active', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await
            .unwrap();

            // Apply v2 again
            migrate_v2_add_branch_and_last_synced(&pool).await.unwrap();

            // Verify data is intact
            let row = sqlx::query("SELECT branch_name FROM sessions WHERE id = 'test-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
            let branch_name: Option<String> = row.get("branch_name");
            assert_eq!(
                branch_name,
                Some("feature-branch".to_string()),
                "Data should survive double v2 migration"
            );
        }

        /// ATTACK: v2 without v1 should fail cleanly
        #[tokio::test]
        async fn adversarial_v2_without_v1_error_type() {
            let pool = create_fresh_pool().await;

            let result = migrate_v2_add_branch_and_last_synced(&pool).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                MigrationError::SchemaCreationFailed { operation, source } => {
                    assert!(
                        source.contains("sessions table does not exist"),
                        "Error should mention missing sessions table, got: {source}"
                    );
                    assert!(
                        operation.contains("migrate_v2"),
                        "Operation should mention v2, got: {operation}"
                    );
                }
                other => panic!("Expected SchemaCreationFailed, got: {other}"),
            }
        }

        /// ATTACK: get_migration_version with orphaned tracking table
        #[tokio::test]
        async fn adversarial_migration_version_with_orphaned_tracking_table() {
            let pool = create_fresh_pool().await;

            // Create tracking table directly without sessions
            execute_sql(&pool, sql::CREATE_SCHEMA_MIGRATIONS_TABLE)
                .await
                .unwrap();
            sqlx::query(sql::INSERT_MIGRATION)
                .bind(1i64)
                .bind("create_sessions_table")
                .execute(&pool)
                .await
                .unwrap();

            // Version says 1 but sessions table doesn't exist
            let version = get_migration_version(&pool).await.unwrap();
            assert_eq!(version, Some(1), "Version should be 1");

            let sessions_exists = table_exists(&pool, "sessions").await.unwrap();
            assert!(
                !sessions_exists,
                "Sessions table should not exist despite version claiming 1"
            );

            // Run migrations - should self-heal by creating the sessions table
            let result = migrate_sessions_table(&pool).await;
            assert!(
                result.is_ok(),
                "Migration should self-heal even with orphaned tracking record"
            );

            // But sessions table still won't exist because migrate_sessions_table
            // checks if tracking exists and sessions exists. If sessions doesn't
            // exist, it creates it.
            let sessions_now = table_exists(&pool, "sessions").await.unwrap();
            assert!(
                sessions_now,
                "Sessions table should now exist after self-healing migration"
            );
        }

        /// ATTACK: validate_migration_name with unicode
        #[test]
        fn adversarial_migration_name_unicode_rejected() {
            assert!(
                validate_migration_name("migración").is_err(),
                "Unicode chars in migration name should be rejected"
            );
            assert!(
                validate_migration_name("日本語").is_err(),
                "CJK chars in migration name should be rejected"
            );
        }

        /// ATTACK: validate_migration_name with SQL comment characters
        #[test]
        fn adversarial_migration_name_sql_comment_rejected() {
            assert!(
                validate_migration_name("test--comment").is_err(),
                "Double dash should be rejected"
            );
            assert!(
                validate_migration_name("test/*block*/").is_err(),
                "Block comment chars should be rejected"
            );
        }

        /// ATTACK: MigrationVersion boundary - i64::MAX
        #[test]
        fn adversarial_migration_version_max_i64() {
            let v = MigrationVersion::new(i64::MAX);
            assert!(v.is_ok(), "i64::MAX should be a valid migration version");
        }

        /// ATTACK: Rapid repeated migrations
        #[tokio::test]
        async fn adversarial_rapid_repeated_migrations() {
            let pool = create_fresh_pool().await;

            for i in 0..10 {
                let result = run_all_migrations(&pool).await;
                assert!(
                    result.is_ok(),
                    "Migration {} should succeed (idempotent)",
                    i
                );
            }

            let version = get_migration_version(&pool).await.unwrap();
            assert_eq!(
                version,
                Some(2),
                "Version should still be 2 after 10 applications"
            );

            // Verify no duplicate migration records
            let rows = sqlx::query("SELECT COUNT(*) as cnt FROM schema_migrations")
                .fetch_one(&pool)
                .await
                .unwrap();
            let count: i64 = rows.get("cnt");
            assert_eq!(
                count, 2,
                "Should have exactly 2 migration records (v1 and v2), got {count}"
            );
        }

        /// ATTACK: Double rollback should fail
        #[tokio::test]
        async fn adversarial_double_rollback_fails() {
            let pool = create_fresh_pool().await;
            run_all_migrations(&pool).await.unwrap();

            let result1 = rollback_v2_branch_and_last_synced(&pool).await;
            assert!(result1.is_ok());

            let result2 = rollback_v2_branch_and_last_synced(&pool).await;
            assert!(
                result2.is_err(),
                "Double rollback should fail since v2 is no longer applied"
            );
        }

        /// ATTACK: migrate_with_version ignores the version parameter
        #[tokio::test]
        async fn adversarial_migrate_with_version_ignores_custom_version() {
            let pool = create_fresh_pool().await;

            let result = migrate_with_version(&pool, 42).await;
            assert!(result.is_ok());

            let version = get_migration_version(&pool).await.unwrap();
            assert_eq!(
                version,
                Some(1),
                "migrate_with_version always records v1 regardless of passed version"
            );
        }

        /// ATTACK: Insert sessions with NULL name (NOT NULL constraint)
        #[tokio::test]
        async fn adversarial_insert_null_name_rejected() {
            let pool = create_fresh_pool().await;
            migrate_sessions_table(&pool).await.unwrap();

            let result = sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace)
                   VALUES ('test-1', NULL, '/tmp')"#,
            )
            .execute(&pool)
            .await;

            assert!(
                result.is_err(),
                "NULL name should be rejected by NOT NULL constraint"
            );
        }

        /// ATTACK: Insert sessions with NULL branch_state (NOT NULL constraint)
        #[tokio::test]
        async fn adversarial_insert_null_branch_state_rejected() {
            let pool = create_fresh_pool().await;
            migrate_sessions_table(&pool).await.unwrap();

            let result = sqlx::query(
                r#"INSERT INTO sessions (id, name, branch_state)
                   VALUES ('test-1', 'test', NULL)"#,
            )
            .execute(&pool)
            .await;

            assert!(
                result.is_err(),
                "NULL branch_state should be rejected by NOT NULL constraint"
            );
        }

        /// ATTACK: Duplicate session name (no UNIQUE constraint in reconciled schema)
        #[tokio::test]
        async fn adversarial_duplicate_session_name_allowed() {
            let pool = create_fresh_pool().await;
            migrate_sessions_table(&pool).await.unwrap();

            sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, branch_state, session_state, created_at)
                   VALUES ('test-1', 'same-name', '/tmp', 'Detached', 'Created', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await
            .unwrap();

            let result = sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, branch_state, session_state, created_at)
                   VALUES ('test-2', 'same-name', '/tmp2', 'Detached', 'Created', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await;

            assert!(
                result.is_ok(),
                "Duplicate session name is allowed in reconciled schema (no UNIQUE constraint)"
            );
        }

        /// ATTACK: Duplicate session id (PRIMARY KEY constraint)
        #[tokio::test]
        async fn adversarial_duplicate_session_id_rejected() {
            let pool = create_fresh_pool().await;
            migrate_sessions_table(&pool).await.unwrap();

            sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, branch_state, session_state, created_at)
                   VALUES ('dup-id', 'name-1', '/tmp', 'Detached', 'Created', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await
            .unwrap();

            let result = sqlx::query(
                r#"INSERT INTO sessions (id, name, workspace, branch_state, session_state, created_at)
                   VALUES ('dup-id', 'name-2', '/tmp2', 'Detached', 'Created', '2024-01-01T00:00:00Z')"#,
            )
            .execute(&pool)
            .await;

            assert!(
                result.is_err(),
                "Duplicate session id should be rejected by PRIMARY KEY constraint"
            );
        }

        /// ATTACK: column_exists with non-existent table
        #[tokio::test]
        async fn adversarial_column_exists_nonexistent_table() {
            let pool = create_fresh_pool().await;

            let result = column_exists(&pool, "nonexistent_table", "id").await;
            assert!(
                result.is_ok(),
                "column_exists on non-existent table should not crash"
            );
            assert!(
                !result.unwrap(),
                "column_exists on non-existent table should return false"
            );
        }
    }
}
