//! Database Infrastructure Service

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            max_connections: 5,
        }
    }

    pub fn in_memory() -> Self {
        Self::new(":memory:")
    }
}

pub trait DatabaseService: Send + Sync {
    fn execute(&self, query: &str) -> Result<()>;
    fn query<T: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Vec<T>>;
}

pub struct SqliteDatabaseService {
    #[allow(dead_code)]
    config: DatabaseConfig,
}

impl SqliteDatabaseService {
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }

    pub fn in_memory() -> Self {
        Self::new(DatabaseConfig::in_memory())
    }
}

impl DatabaseService for SqliteDatabaseService {
    fn execute(&self, _query: &str) -> Result<()> {
        Ok(())
    }

    fn query<T: for<'de> Deserialize<'de>>(&self, _query: &str) -> Result<Vec<T>> {
        Ok(vec![])
    }
}

pub fn create_database_service(config: DatabaseConfig) -> impl DatabaseService {
    SqliteDatabaseService::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config() {
        let config = DatabaseConfig::new("/tmp/test.db");
        assert_eq!(config.path, "/tmp/test.db");
    }

    #[test]
    fn test_in_memory_config() {
        let config = DatabaseConfig::in_memory();
        assert_eq!(config.path, ":memory:");
    }
}
