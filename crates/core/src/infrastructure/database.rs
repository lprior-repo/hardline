//! Database Infrastructure Service
//!
//! Provides async SQLite database operations with WAL mode enabled.

use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;

use crate::error::{Error, Result};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file or ":memory:" for in-memory database
    pub path: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
}

impl DatabaseConfig {
    /// Create a new database configuration
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            max_connections: 5,
        }
    }

    /// Create an in-memory database configuration
    pub fn in_memory() -> Self {
        Self::new(":memory:")
    }

    /// Get the SQLite connection URL
    fn connection_url(&self) -> String {
        if self.path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", self.path)
        }
    }
}

/// Database service trait for async operations
#[async_trait]
pub trait DatabaseService: Send + Sync + 'static {
    /// Execute a query that doesn't return rows (INSERT, UPDATE, DELETE, CREATE TABLE, etc.)
    async fn execute(&self, query: &str) -> Result<()>;

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
            .max_connections(config.max_connections)
            .connect(&config.connection_url())
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Set synchronous to NORMAL for better performance while maintaining durability
        sqlx::query("PRAGMA synchronous=NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        tracing::info!("SQLite database initialized with WAL mode at: {}", config.path);

        Ok(Self { pool })
    }

    /// Create an in-memory SQLite database
    pub async fn in_memory() -> Result<Self> {
        Self::new(DatabaseConfig::in_memory()).await
    }

    /// Create from an existing pool (for testing or external pool management)
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new SQLite database service synchronously using a file path
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        Self::new(DatabaseConfig::new(path_str)).await
    }

    /// Close the database connection pool
    pub async fn close(&self) -> Result<()> {
        self.pool.close().await;
        tracing::info!("SQLite database connection pool closed");
        Ok(())
    }
}

#[async_trait]
impl DatabaseService for SqliteDatabaseService {
    async fn execute(&self, query: &str) -> Result<()> {
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
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

    #[tokio::test]
    async fn test_in_memory_database() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        
        // Create a test table
        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await?;
        
        // Insert data
        db.execute("INSERT INTO test (name) VALUES ('test1')").await?;
        db.execute("INSERT INTO test (name) VALUES ('test2')").await?;
        
        // Query data
        let results: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM test ORDER BY id")
            .fetch_all(db.pool())
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (1, "test1".to_string()));
        assert_eq!(results[1], (2, "test2".to_string()));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_database_config() {
        let config = DatabaseConfig::new("/tmp/test.db");
        assert_eq!(config.path, "/tmp/test.db");
        assert_eq!(config.max_connections, 5);
        
        let url = config.connection_url();
        assert!(url.contains("sqlite:"));
        assert!(url.contains("/tmp/test.db"));
    }

    #[tokio::test]
    async fn test_in_memory_config() {
        let config = DatabaseConfig::in_memory();
        assert_eq!(config.path, ":memory:");
        assert_eq!(config.connection_url(), "sqlite::memory:");
    }

    #[tokio::test]
    async fn test_wal_mode_enabled() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        
        // Verify WAL mode is enabled
        let result: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(db.pool())
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        
        assert_eq!(result.0.to_lowercase(), "wal");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_foreign_keys_enabled() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        
        // Verify foreign keys are enabled
        let result: (i32,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(db.pool())
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        
        assert_eq!(result.0, 1);
        
        Ok(())
    }
}
