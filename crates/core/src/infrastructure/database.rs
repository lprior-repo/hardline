//! Database Infrastructure Service
//!
//! Provides async SQLite database operations with WAL mode enabled.

use async_trait::async_trait;
use sqlx::{
    sqlite::{SqlitePoolOptions, SqliteRow},
    Column, Row, SqlitePool, TypeInfo,
};
use std::path::Path;

use crate::error::{Error, Result};

use super::database_types::{DatabasePath, MaxConnections};

/// Database configuration with validated newtypes
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file or ":memory:" for in-memory database
    pub path: DatabasePath,
    /// Maximum number of connections in the pool
    pub max_connections: MaxConnections,
}

impl DatabaseConfig {
    /// Create a new database configuration with validation
    pub fn new(path: impl Into<String>) -> Result<Self> {
        Ok(Self {
            path: DatabasePath::new(path)?,
            max_connections: MaxConnections::default_value(),
        })
    }

    /// Create a new database configuration with explicit max connections
    pub fn with_connections(path: impl Into<String>, max_connections: u32) -> Result<Self> {
        Ok(Self {
            path: DatabasePath::new(path)?,
            max_connections: MaxConnections::new(max_connections)?,
        })
    }

    /// Create an in-memory database configuration
    pub fn in_memory() -> Self {
        Self {
            path: DatabasePath::in_memory(),
            max_connections: MaxConnections::default_value(),
        }
    }

    /// Get the SQLite connection URL
    fn connection_url(&self) -> String {
        if self.path.is_in_memory() {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", self.path.as_str())
        }
    }
}

/// Database service trait for async operations
#[async_trait]
pub trait DatabaseService: Send + Sync + 'static {
    /// Execute a query that doesn't return rows
    async fn execute(&self, query: &str) -> Result<()>;

    /// Execute a query that returns rows
    async fn query(&self, query: &str) -> Result<Vec<Vec<String>>>;

    /// Get a reference to the underlying connection pool
    fn pool(&self) -> &SqlitePool;
}

/// SQLite database service with connection pool
pub struct SqliteDatabaseService {
    pool: SqlitePool,
}

impl SqliteDatabaseService {
    /// Create a new SQLite database service from configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections.value())
            .connect(&config.connection_url())
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Enable WAL mode, NORMAL sync, and foreign keys
        for pragma in &["PRAGMA journal_mode=WAL", "PRAGMA synchronous=NORMAL", "PRAGMA foreign_keys=ON"] {
            sqlx::query(pragma)
                .execute(&pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        }

        tracing::info!("SQLite database initialized at: {}", config.path.as_str());
        Ok(Self { pool })
    }

    /// Create an in-memory SQLite database
    pub async fn in_memory() -> Result<Self> {
        Self::new(DatabaseConfig::in_memory()).await
    }

    /// Create from an existing pool
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create using a file path
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = DatabaseConfig::new(path.as_ref().to_string_lossy().to_string())?;
        Self::new(config).await
    }

    /// Close the database connection pool
    pub async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}

/// Parse a single column value to string based on SQLite type info
fn parse_column_value(row: &SqliteRow, col_idx: usize) -> Result<String> {
    let col = &row.columns()[col_idx];
    let type_name = col.type_info().name();

    match type_name {
        "INTEGER" | "INT" => row.try_get::<i64, _>(col_idx).map(|v| v.to_string())
            .map_err(|e| Error::Database(format!("INTEGER: {}", e))),
        "REAL" | "FLOAT" | "DOUBLE" => row.try_get::<f64, _>(col_idx).map(|v| v.to_string())
            .map_err(|e| Error::Database(format!("REAL: {}", e))),
        "TEXT" | "BLOB" => row.try_get::<String, _>(col_idx)
            .map_err(|e| Error::Database(format!("TEXT: {}", e))),
        "BOOLEAN" => row.try_get::<bool, _>(col_idx).map(|v| v.to_string())
            .map_err(|e| Error::Database(format!("BOOLEAN: {}", e))),
        _ => row.try_get::<String, _>(col_idx)
            .map_err(|e| Error::Database(format!("{}: {}", type_name, e))),
    }
}

/// Parse all values from a row into a vector of strings
fn parse_row_values(row: &SqliteRow) -> Result<Vec<String>> {
    (0..row.columns().len()).map(|i| parse_column_value(row, i)).collect()
}

#[async_trait]
impl DatabaseService for SqliteDatabaseService {
    async fn execute(&self, query: &str) -> Result<()> {
        sqlx::query(query).execute(&self.pool).await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<Vec<String>>> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await
            .map_err(|e| Error::Database(e.to_string()))?;
        rows.iter().map(parse_row_values).collect()
    }

    fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// Create a database service from configuration
pub async fn create_database_service(config: DatabaseConfig) -> Result<impl DatabaseService> {
    SqliteDatabaseService::new(config).await
}

/// Create an in-memory database service
pub async fn create_in_memory_database() -> Result<impl DatabaseService> {
    SqliteDatabaseService::in_memory().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database_types::{DatabasePath, MaxConnections};

    #[tokio::test]
    async fn test_in_memory_database() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await?;
        db.execute("INSERT INTO test (name) VALUES ('test1')").await?;
        db.execute("INSERT INTO test (name) VALUES ('test2')").await?;
        let results: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM test ORDER BY id")
            .fetch_all(db.pool()).await.map_err(|e| Error::Database(e.to_string()))?;
        assert_eq!(results.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_query_method() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await?;
        db.execute("INSERT INTO test (name) VALUES ('test1')").await?;
        let results = db.query("SELECT id, name FROM test").await?;
        assert_eq!(results[0], vec!["1".to_string(), "test1".to_string()]);
        Ok(())
    }

    #[test]
    fn test_database_path_validation() {
        assert!(DatabasePath::new("/tmp/test.db").is_ok());
        assert!(DatabasePath::new("").is_err());
        assert!(DatabasePath::in_memory().is_in_memory());
    }

    #[test]
    fn test_max_connections_validation() {
        assert!(MaxConnections::new(5).is_ok());
        assert!(MaxConnections::new(0).is_err());
    }

    #[tokio::test]
    async fn test_database_config() -> Result<()> {
        let config = DatabaseConfig::new("/tmp/test.db")?;
        assert_eq!(config.path.as_str(), "/tmp/test.db");
        assert_eq!(config.max_connections.value(), 5);
        assert!(config.connection_url().contains("sqlite:"));
        Ok(())
    }

    #[tokio::test]
    async fn test_config_with_explicit_connections() -> Result<()> {
        let config = DatabaseConfig::with_connections("/tmp/test.db", 10)?;
        assert_eq!(config.max_connections.value(), 10);
        assert!(DatabaseConfig::with_connections("/tmp/test.db", 0).is_err());
        assert!(DatabaseConfig::new("").is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_in_memory_config() {
        let config = DatabaseConfig::in_memory();
        assert!(config.path.is_in_memory());
    }

    #[tokio::test]
    async fn test_pragma_settings() -> Result<()> {
        // In-memory databases don't support WAL, but we verify pragma functionality
        let db = SqliteDatabaseService::in_memory().await?;
        let result: (i32,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(db.pool()).await.map_err(|e| Error::Database(e.to_string()))?;
        assert_eq!(result.0, 1);
        Ok(())
    }
}
