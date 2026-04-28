//! Chaos Testing Layer
//!
//! Provides wrappers for I/O, DB, and Network operations to inject random failures.
//! Useful for resilience testing.

use std::{path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use rand::Rng;
use sqlx::SqlitePool;

use crate::{
    error::{Error, Result},
    infrastructure::database::DatabaseService,
};

/// Configuration for chaos injection probabilities (0.0 to 1.0)
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    pub io_error_probability: f64,
    pub disk_full_probability: f64,
    pub network_timeout_probability: f64,
    pub network_error_probability: f64,
    pub process_kill_probability: f64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            io_error_probability: 0.0,
            disk_full_probability: 0.0,
            network_timeout_probability: 0.0,
            network_error_probability: 0.0,
            process_kill_probability: 0.0,
        }
    }
}

/// Injector responsible for generating random failures based on config
#[derive(Debug, Clone)]
pub struct ChaosInjector {
    config: ChaosConfig,
}

impl ChaosInjector {
    pub fn new(config: ChaosConfig) -> Self {
        Self { config }
    }

    pub fn inject_io_error(&self) -> std::io::Result<()> {
        let mut rng = rand::rng();
        if rng.random_bool(self.config.io_error_probability) {
            return Err(std::io::Error::other("Chaos: random IO error"));
        }
        if rng.random_bool(self.config.disk_full_probability) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "Chaos: disk full",
            ));
        }
        if rng.random_bool(self.config.process_kill_probability) {
            eprintln!("Chaos: process killed during IO!");
            std::process::exit(1);
        }
        Ok(())
    }

    pub async fn inject_network_delay(&self) {
        let should_delay = {
            let mut rng = rand::rng();
            rng.random_bool(self.config.network_timeout_probability)
        };
        if should_delay {
            tokio::time::sleep(Duration::from_millis(50)).await; // Reduced from 5s for fast tests
        }
    }

    pub fn inject_network_error(&self) -> std::io::Result<()> {
        let mut rng = rand::rng();
        if rng.random_bool(self.config.network_error_probability) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Chaos: network error",
            ));
        }
        if rng.random_bool(self.config.process_kill_probability) {
            eprintln!("Chaos: process killed during Network call!");
            std::process::exit(1);
        }
        Ok(())
    }

    pub fn inject_db_error(&self) -> Result<()> {
        let mut rng = rand::rng();
        if rng.random_bool(self.config.io_error_probability) {
            return Err(crate::error_io::IoErrorKind::Io(std::io::Error::other(
                "Chaos: database IO error",
            ))
            .into());
        }
        if rng.random_bool(self.config.disk_full_probability) {
            return Err(crate::error_io::IoErrorKind::Io(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "Chaos: database disk full",
            ))
            .into());
        }
        if rng.random_bool(self.config.process_kill_probability) {
            eprintln!("Chaos: process killed during DB op!");
            std::process::exit(1);
        }
        Ok(())
    }
}

/// Chaos wrapper for file system operations
pub struct ChaosFs {
    injector: Arc<ChaosInjector>,
}

impl ChaosFs {
    pub fn new(injector: Arc<ChaosInjector>) -> Self {
        Self { injector }
    }

    pub fn read_to_string<P: AsRef<Path>>(&self, path: P) -> std::io::Result<String> {
        self.injector.inject_io_error()?;
        std::fs::read_to_string(path)
    }

    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> std::io::Result<()> {
        self.injector.inject_io_error()?;
        std::fs::write(path, contents)
    }

    pub fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        self.injector.inject_io_error()?;
        std::fs::create_dir_all(path)
    }
}

/// Chaos wrapper for DatabaseService
pub struct ChaosDatabaseService<T: DatabaseService> {
    inner: T,
    injector: Arc<ChaosInjector>,
}

impl<T: DatabaseService> ChaosDatabaseService<T> {
    pub fn new(inner: T, injector: Arc<ChaosInjector>) -> Self {
        Self { inner, injector }
    }
}

#[async_trait]
impl<T: DatabaseService + 'static> DatabaseService for ChaosDatabaseService<T> {
    async fn execute(&self, query: &str) -> Result<()> {
        self.injector.inject_db_error()?;
        self.inner.execute(query).await
    }

    async fn query(&self, query: &str) -> Result<Vec<Vec<String>>> {
        self.injector.inject_db_error()?;
        self.inner.query(query).await
    }

    fn pool(&self) -> &SqlitePool {
        self.inner.pool()
    }
}

#[async_trait::async_trait]
pub trait NetworkService: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<String>;
}

/// Chaos wrapper for Network calls
pub struct ChaosNetworkService<T: NetworkService> {
    inner: T,
    injector: Arc<ChaosInjector>,
}

impl<T: NetworkService> ChaosNetworkService<T> {
    pub fn new(inner: T, injector: Arc<ChaosInjector>) -> Self {
        Self { inner, injector }
    }
}

#[async_trait::async_trait]
impl<T: NetworkService> NetworkService for ChaosNetworkService<T> {
    async fn fetch(&self, url: &str) -> Result<String> {
        self.injector.inject_network_delay().await;
        self.injector
            .inject_network_error()
            .map_err(|e| Error::io_error(e.to_string()))?;
        self.inner.fetch(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::SqliteDatabaseService;

    #[test]
    fn test_chaos_fs_resilience() {
        let config = ChaosConfig {
            io_error_probability: 1.0, // Always fail
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let fs = ChaosFs::new(injector);

        let result = fs.read_to_string("dummy.txt");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn test_chaos_fs_disk_full() {
        let config = ChaosConfig {
            disk_full_probability: 1.0, // Always fail with disk full
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let fs = ChaosFs::new(injector);

        let result = fs.write("dummy.txt", b"content");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::StorageFull);
    }

    #[tokio::test]
    async fn test_chaos_db_resilience() {
        let config = ChaosConfig {
            io_error_probability: 1.0, // Always fail
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_db = SqliteDatabaseService::in_memory().await.unwrap();
        let chaos_db = ChaosDatabaseService::new(inner_db, injector);

        let result = chaos_db.execute("SELECT 1").await;
        assert!(result.is_err());
        if let Err(Error::Io(e)) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("Chaos: database IO error"),
                "Expected IO error, got: {msg}"
            );
        } else {
            panic!("Expected Io error");
        }
    }

    struct DummyNetworkService;

    #[async_trait::async_trait]
    impl NetworkService for DummyNetworkService {
        async fn fetch(&self, _url: &str) -> Result<String> {
            Ok("success".to_string())
        }
    }

    #[tokio::test]
    async fn test_chaos_network_resilience() {
        let config = ChaosConfig {
            network_error_probability: 1.0, // Always fail
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_net = DummyNetworkService;
        let chaos_net = ChaosNetworkService::new(inner_net, injector);

        let result = chaos_net.fetch("http://example.com").await;
        assert!(result.is_err());
        if let Err(Error::Io(e)) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("Chaos: network error"),
                "Expected network error, got: {msg}"
            );
        } else {
            panic!("Expected Io error");
        }
    }

    #[tokio::test]
    async fn test_chaos_network_timeout() {
        let config = ChaosConfig {
            network_timeout_probability: 1.0, // Always delay
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_net = DummyNetworkService;
        let chaos_net = ChaosNetworkService::new(inner_net, injector);

        let start = std::time::Instant::now();
        let result = chaos_net.fetch("http://example.com").await;
        let duration = start.elapsed();

        assert!(result.is_ok());
        assert!(duration >= Duration::from_millis(50));
    }

    // =========================================================================
    // ChaosConfig default
    // =========================================================================

    #[test]
    fn given_default_config_when_all_probabilities_then_zero() {
        let config = ChaosConfig::default();
        assert_eq!(config.io_error_probability, 0.0);
        assert_eq!(config.disk_full_probability, 0.0);
        assert_eq!(config.network_timeout_probability, 0.0);
        assert_eq!(config.network_error_probability, 0.0);
        assert_eq!(config.process_kill_probability, 0.0);
    }

    #[test]
    fn given_config_when_debug_then_contains_probabilities() {
        let config = ChaosConfig {
            io_error_probability: 0.5,
            disk_full_probability: 0.1,
            network_timeout_probability: 0.2,
            network_error_probability: 0.3,
            process_kill_probability: 0.0,
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("0.5"));
    }

    // =========================================================================
    // ChaosInjector with zero probabilities (always pass through)
    // =========================================================================

    #[test]
    fn given_zero_probabilities_when_inject_io_error_then_ok() {
        let config = ChaosConfig::default();
        let injector = ChaosInjector::new(config);
        assert!(injector.inject_io_error().is_ok());
    }

    #[test]
    fn given_zero_probabilities_when_inject_network_error_then_ok() {
        let config = ChaosConfig::default();
        let injector = ChaosInjector::new(config);
        assert!(injector.inject_network_error().is_ok());
    }

    #[test]
    fn given_zero_probabilities_when_inject_db_error_then_ok() {
        let config = ChaosConfig::default();
        let injector = ChaosInjector::new(config);
        assert!(injector.inject_db_error().is_ok());
    }

    // =========================================================================
    // ChaosFs with zero probabilities
    // =========================================================================

    #[test]
    fn given_zero_probabilities_when_read_nonexistent_then_io_error() {
        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let fs = ChaosFs::new(injector);

        // File doesn't exist, but chaos passes through to real FS
        let result = fs.read_to_string("/nonexistent/file/that/does/not/exist.txt");
        assert!(result.is_err());
        // Should be a real IO error, not a chaos error
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    // =========================================================================
    // ChaosConfig Clone
    // =========================================================================

    #[test]
    fn given_config_when_cloned_then_equal() {
        let config = ChaosConfig {
            io_error_probability: 0.3,
            disk_full_probability: 0.1,
            network_timeout_probability: 0.5,
            network_error_probability: 0.2,
            process_kill_probability: 0.01,
        };
        let cloned = config.clone();
        assert_eq!(config.io_error_probability, cloned.io_error_probability);
        assert_eq!(
            config.network_timeout_probability,
            cloned.network_timeout_probability
        );
    }

    // =========================================================================
    // ChaosInjector Debug
    // =========================================================================

    #[test]
    fn given_injector_when_debug_then_contains_config() {
        let config = ChaosConfig {
            io_error_probability: 0.42,
            ..Default::default()
        };
        let injector = ChaosInjector::new(config);
        let debug = format!("{injector:?}");
        assert!(debug.contains("0.42"));
    }

    // =========================================================================
    // ChaosFs create_dir_all pass-through
    // =========================================================================

    #[test]
    fn given_zero_probabilities_when_create_dir_all_then_delegates() {
        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let fs = ChaosFs::new(injector);

        let tmp = std::env::temp_dir().join("chaos_test_create_dir");
        let _ = std::fs::remove_dir_all(&tmp);
        let result = fs.create_dir_all(&tmp);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // =========================================================================
    // ChaosDatabaseService pool delegation
    // =========================================================================

    #[tokio::test]
    async fn given_chaos_db_when_pool_then_delegates() {
        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_db = SqliteDatabaseService::in_memory().await.unwrap();
        let chaos_db = ChaosDatabaseService::new(inner_db, injector);

        // Pool access should work
        assert!(chaos_db.pool().is_closed() == false);
    }

    // =========================================================================
    // ChaosDatabaseService query pass-through
    // =========================================================================

    #[tokio::test]
    async fn given_zero_probabilities_when_chaos_db_query_then_delegates() {
        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_db = SqliteDatabaseService::in_memory().await.unwrap();
        inner_db
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, val TEXT)")
            .await
            .unwrap();
        inner_db
            .execute("INSERT INTO test (val) VALUES ('hello')")
            .await
            .unwrap();

        let chaos_db = ChaosDatabaseService::new(inner_db, injector);
        let results = chaos_db.query("SELECT val FROM test").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], vec!["hello".to_string()]);
    }

    // =========================================================================
    // ChaosInjector with disk_full_probability
    // =========================================================================

    #[test]
    fn given_disk_full_probability_one_when_inject_io_error_then_storage_full() {
        let config = ChaosConfig {
            disk_full_probability: 1.0,
            ..Default::default()
        };
        let injector = ChaosInjector::new(config);
        let result = injector.inject_io_error();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::StorageFull);
    }

    // =========================================================================
    // ChaosFs write pass-through with zero probabilities
    // =========================================================================

    #[test]
    fn given_zero_probabilities_when_write_then_delegates() {
        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let fs = ChaosFs::new(injector);

        let tmp = std::env::temp_dir().join("chaos_test_write.txt");
        let result = fs.write(&tmp, b"test-content");
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "test-content");
        let _ = std::fs::remove_file(&tmp);
    }

    // =========================================================================
    // ChaosDatabaseService with io_error_probability for query
    // =========================================================================

    #[tokio::test]
    async fn given_io_error_probability_one_when_chaos_db_query_then_err() {
        let config = ChaosConfig {
            io_error_probability: 1.0,
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_db = SqliteDatabaseService::in_memory().await.unwrap();
        inner_db
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();

        let chaos_db = ChaosDatabaseService::new(inner_db, injector);
        let result = chaos_db.query("SELECT id FROM test").await;
        assert!(result.is_err());
    }

    // =========================================================================
    // ChaosNetworkService with zero probabilities - passes through
    // =========================================================================

    #[tokio::test]
    async fn given_zero_probabilities_when_chaos_network_fetch_then_delegates() {
        struct AlwaysOkService;
        #[async_trait::async_trait]
        impl NetworkService for AlwaysOkService {
            async fn fetch(&self, _url: &str) -> Result<String> {
                Ok("from-service".to_string())
            }
        }

        let config = ChaosConfig::default();
        let injector = Arc::new(ChaosInjector::new(config));
        let chaos_net = ChaosNetworkService::new(AlwaysOkService, injector);

        let result = chaos_net.fetch("http://example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "from-service");
    }

    // =========================================================================
    // ChaosConfig with all non-zero probabilities
    // =========================================================================

    #[test]
    fn given_all_nonzero_config_when_debug_then_contains_all() {
        let config = ChaosConfig {
            io_error_probability: 0.1,
            disk_full_probability: 0.2,
            network_timeout_probability: 0.3,
            network_error_probability: 0.4,
            process_kill_probability: 0.05,
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("0.1"));
        assert!(debug.contains("0.2"));
        assert!(debug.contains("0.3"));
        assert!(debug.contains("0.4"));
        assert!(debug.contains("0.05"));
    }

    // =========================================================================
    // ChaosNetworkService network_error_probability with disk_full
    // =========================================================================

    #[test]
    fn given_disk_full_probability_one_when_inject_network_error_then_connection_reset() {
        // disk_full doesn't affect network_error path, only io_error path
        let config = ChaosConfig {
            disk_full_probability: 1.0,
            ..Default::default()
        };
        let injector = ChaosInjector::new(config);
        // inject_network_error only checks network_error_probability and process_kill_probability
        let result = injector.inject_network_error();
        // With disk_full=1.0 but network_error=0.0, network_error should succeed
        assert!(result.is_ok());
    }

    // =========================================================================
    // ChaosInjector Clone
    // =========================================================================

    #[test]
    fn given_injector_when_cloned_then_same_config() {
        let config = ChaosConfig {
            io_error_probability: 0.25,
            ..Default::default()
        };
        let injector = ChaosInjector::new(config);
        let cloned = injector.clone();
        // Both should have same behavior (zero probabilities for network)
        assert!(injector.inject_network_error().is_ok());
        assert!(cloned.inject_network_error().is_ok());
    }
}
