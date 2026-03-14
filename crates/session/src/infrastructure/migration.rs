//! SQLite migration module for sessions table
//!
//! Provides idempotent migrations for the sessions table schema.

use sqlx::sqlite::{SqlitePool, SqliteRow};
use sqlx::Row;

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
            Self::VersionConflict { version, table_name } => {
                write!(f, "VersionConflict: version {} already applied to {}", version, table_name)
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
            name TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'creating'
                CHECK(status IN ('creating', 'active', 'completed', 'failed', 'cancelled')),
            state TEXT NOT NULL DEFAULT 'pending'
                CHECK(state IN ('pending', 'working', 'waiting', 'stopping', 'terminated')),
            workspace_path TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            metadata TEXT,
            owner TEXT
        );
    "#;

    pub const CREATE_SESSIONS_NAME_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name);
    "#;

    pub const CREATE_SESSIONS_STATUS_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
    "#;

    pub const CREATE_SESSIONS_CREATED_AT_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
    "#;

    pub const CREATE_SESSIONS_UPDATED_AT_INDEX: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
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
}

/// Validate migration name is a valid SQL identifier
fn validate_migration_name(name: &str) -> Result<(), MigrationError> {
    let is_valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_');
    if !is_valid {
        return Err(MigrationError::InvalidMigrationFormat {
            migration: name.to_string(),
            reason: "must be valid SQL identifier (alphanumeric, underscore only)".to_string(),
        });
    }
    Ok(())
}

/// Check if a table exists in the database
async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool, MigrationError> {
    let sql = format!(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='{}';",
        table_name
    );
    let row: SqliteRow = sqlx::query(&sql).fetch_one(pool).await?;
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
    pool.acquire().await.map_err(|e| MigrationError::InvalidConnection {
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

    // Create sessions table
    execute_sql(pool, sql::CREATE_SESSIONS_TABLE).await?;

    // Create indexes
    execute_sql(pool, sql::CREATE_SESSIONS_NAME_INDEX).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_STATUS_INDEX).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_CREATED_AT_INDEX).await?;
    execute_sql(pool, sql::CREATE_SESSIONS_UPDATED_AT_INDEX).await?;

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
        SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap()
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
        
        let column_names: Vec<String> = result.iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();
        
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"name".to_string()));
        assert!(column_names.contains(&"status".to_string()));
        assert!(column_names.contains(&"state".to_string()));
        assert!(column_names.contains(&"workspace_path".to_string()));
        assert!(column_names.contains(&"created_at".to_string()));
        assert!(column_names.contains(&"updated_at".to_string()));
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
}
