//! Chaos Testing Layer
//!
//! Provides wrappers for I/O, DB, and Network operations to inject random failures.
//! Useful for resilience testing.

use crate::error::{Error, Result};
use crate::infrastructure::database::DatabaseService;
use rand::Rng;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Chaos: random IO error",
            ));
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
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Chaos: database IO error",
            )));
        }
        if rng.random_bool(self.config.disk_full_probability) {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "Chaos: database disk full",
            )));
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

impl<T: DatabaseService> DatabaseService for ChaosDatabaseService<T> {
    fn execute(&self, query: &str) -> Result<()> {
        self.injector.inject_db_error()?;
        self.inner.execute(query)
    }

    fn query<D: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Vec<D>> {
        self.injector.inject_db_error()?;
        self.inner.query(query)
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
        self.injector.inject_network_error().map_err(Error::Io)?;
        self.inner.fetch(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::{DatabaseConfig, SqliteDatabaseService};

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

    #[test]
    fn test_chaos_db_resilience() {
        let config = ChaosConfig {
            io_error_probability: 1.0, // Always fail
            ..Default::default()
        };
        let injector = Arc::new(ChaosInjector::new(config));
        let inner_db = SqliteDatabaseService::new(DatabaseConfig::in_memory());
        let chaos_db = ChaosDatabaseService::new(inner_db, injector);

        let result = chaos_db.execute("SELECT 1");
        assert!(result.is_err());
        if let Err(Error::Io(e)) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::Other);
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
            assert_eq!(e.kind(), std::io::ErrorKind::ConnectionReset);
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
}
