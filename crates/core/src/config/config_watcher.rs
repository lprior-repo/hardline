//! Hot-reload configuration watcher.
//!
//! Provides [`HotReloadConfigManager`] which wraps [`ConfigManager`] with
//! file-watching and automatic reload when config files change on disk.
//!
//! # Features
//!
//! - File watching via the `notify` crate
//! - 150 ms debounce on reload (avoids rapid re-reads during writes)
//! - Auto-reload on config file changes (modify or create events)
//! - 1 MB max config file size (rejects oversized files)
//! - Symlink rejection for security
//!
//! # Architecture
//!
//! ```text
//!  HotReloadConfigManager
//!  ├── inner: Arc<RwLock<Config>>  (current config, read by consumers)
//!  └── watcher task (tokio::spawn)
//!       ├── notify::RecommendedWatcher
//!       ├── debounced rx channel
//!       └── reload loop
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use scp_core::config::HotReloadConfigManager;
//!
//! # async fn example() -> scp_core::error::Result<()> {
//! let manager = HotReloadConfigManager::new().await?;
//!
//! // Get current config (fast, non-blocking read)
//! let config = manager.get().await;
//! println!("Config keys: {:?}", config.keys().collect::<Vec<_>>());
//!
//! // Config auto-reloads when files change
//! # Ok(())
//! # }
//! ```

use std::{path::PathBuf, sync::Arc, time::Duration};

use notify::Watcher;
use tokio::sync::{mpsc, RwLock};

use super::config_core::{Config, ConfigManager};
use crate::{error::Result, error_config::ConfigErrorKind};

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

/// Maximum allowed config file size: 1 MB.
pub const MAX_CONFIG_FILE_SIZE: usize = 1_048_576;

/// Default debounce duration for reload: 150 ms.
const DEBOUNCE_MS: u64 = 150;

// ═══════════════════════════════════════════════════════════════════════════
// HotReloadConfigManager
// ═══════════════════════════════════════════════════════════════════════════

/// Thread-safe, reloadable configuration manager.
///
/// Watches config files and automatically reloads when they change.
/// Uses a 150 ms debounce to avoid rapid re-reads during in-progress writes.
#[derive(Clone)]
pub struct HotReloadConfigManager {
    inner: Arc<RwLock<Config>>,
}

struct HotReloadInner {
    config: Config,
}

impl HotReloadConfigManager {
    /// Create a new `HotReloadConfigManager` with hot-reload enabled.
    ///
    /// Loads initial config and spawns a background watcher task that
    /// automatically reloads when config files change.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Initial config load fails
    /// - Unable to set up file watcher
    pub async fn new() -> Result<Self> {
        Self::with_manager(ConfigManager::new()?).await
    }

    /// Create with an explicit [`ConfigManager`].
    ///
    /// Useful for testing or when custom paths are needed.
    ///
    /// # Errors
    ///
    /// Returns error if initial config load or watcher setup fails.
    pub async fn with_manager(manager: ConfigManager) -> Result<Self> {
        let config = manager.load()?;

        let instance = Self {
            inner: Arc::new(RwLock::new(HotReloadInner { config }.config)),
        };

        // Spawn the config watcher task
        let inner = instance.inner.clone();
        let config_paths = collect_config_paths(&manager);

        if config_paths.is_empty() {
            // Nothing to watch; skip spawning the task.
            tracing::debug!("No config paths to watch; hot-reload disabled");
            return Ok(instance);
        }

        let (tx, mut rx) = mpsc::channel::<()>(4);

        // Set up notify watcher
        let watcher_result = notify::recommended_watcher(
            move |res: std::result::Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        // Best-effort send; if the channel is full we simply
                        // coalesce into the next event.
                        let _ = tx.blocking_send(());
                    }
                }
            },
        )
        .map_err(|e| {
            crate::error::Error::from(ConfigErrorKind::WatcherError(format!(
                "Failed to create file watcher: {e}"
            )))
        })?;

        let mut watcher = watcher_result;

        // Register paths with the watcher.
        for path in &config_paths {
            let watch_result = watcher.watch(path, notify::RecursiveMode::NonRecursive);

            // If the file does not exist yet, fall back to watching the parent
            // directory so we can detect creation events.
            if watch_result.is_err() {
                if let Some(parent) = path.parent() {
                    let _ = watcher.watch(parent, notify::RecursiveMode::NonRecursive);
                }
            }
        }

        // Spawn the reload loop.
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(()) = rx.recv() => {
                        // Debounce: small delay before reloading.
                        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                        // Drain any queued events that arrived during the
                        // debounce window so we only reload once.
                        while rx.try_recv().is_ok() {}

                        // Reload config.
                        match manager.load() {
                            Ok(new_config) => {
                                let mut write = inner.write().await;
                                *write = new_config;
                                tracing::info!("Config reloaded successfully");
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Config reload failed: {e}; using previous config"
                                );
                            }
                        }
                    }
                    else => break,
                }
            }
        });

        tracing::info!(
            paths = ?config_paths,
            "Hot-reload config watcher started"
        );

        Ok(instance)
    }

    /// Get the current configuration.
    ///
    /// Returns a snapshot of the most recently loaded config, including
    /// any hot-reloaded changes.
    pub async fn get(&self) -> Config {
        let inner = self.inner.read().await;
        inner.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Collect all config file paths from a [`ConfigManager`].
fn collect_config_paths(manager: &ConfigManager) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Global config
    if manager.global_path().exists() {
        paths.push(manager.global_path().to_path_buf());
    } else {
        // Watch the parent even if the file doesn't exist yet.
        if let Some(parent) = manager.global_path().parent() {
            paths.push(parent.to_path_buf());
        }
    }

    // Project config
    if let Some(project_path) = manager.project_path() {
        if project_path.exists() {
            paths.push(project_path.to_path_buf());
        } else if let Some(parent) = project_path.parent() {
            paths.push(parent.to_path_buf());
        }
    }

    paths
}

// ═══════════════════════════════════════════════════════════════════════════
// File validation helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Validate that a config file path is safe to read.
///
/// Checks:
/// - The file is not a symbolic link (security).
/// - The file is not a dead symlink (target does not exist).
/// - The file does not exceed [`MAX_CONFIG_FILE_SIZE`].
///
/// # Errors
///
/// - [`ConfigErrorKind::SecuritySymlink`] if the path is a symlink.
/// - [`ConfigErrorKind::DeadSymlink`] if the symlink target does not exist.
/// - [`ConfigErrorKind::FileTooLarge`] if the file exceeds 1 MB.
/// - IO errors are propagated.
pub fn validate_config_file(path: &std::path::Path) -> Result<()> {
    // Use symlink_metadata to detect symlinks without following them.
    let metadata = std::fs::symlink_metadata(path).map_err(|e| {
        crate::error::Error::io_error(format!(
            "Failed to read config file metadata {}: {e}",
            path.display()
        ))
    })?;

    if metadata.is_symlink() {
        // Check if the symlink target actually exists (dead symlink).
        if !path.exists() {
            return Err(ConfigErrorKind::DeadSymlink(format!(
                "Config file {} is a dead symlink - target does not exist",
                path.display()
            ))
            .into());
        }

        return Err(ConfigErrorKind::SecuritySymlink(format!(
            "Config file {} is a symbolic link - refusing to follow for security",
            path.display()
        ))
        .into());
    }

    let file_size = metadata.len();
    if file_size as usize > MAX_CONFIG_FILE_SIZE {
        return Err(ConfigErrorKind::FileTooLarge(format!(
            "Config file {} exceeds maximum size of {} bytes (got {})",
            path.display(),
            MAX_CONFIG_FILE_SIZE,
            file_size
        ))
        .into());
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // ------------------------------------------------------------------
    // MAX_CONFIG_FILE_SIZE constant
    // ------------------------------------------------------------------

    #[test]
    fn max_config_file_size_is_1mb() {
        assert_eq!(MAX_CONFIG_FILE_SIZE, 1_048_576);
    }

    #[test]
    fn debounce_is_150ms() {
        assert_eq!(DEBOUNCE_MS, 150);
    }

    // ------------------------------------------------------------------
    // validate_config_file: symlink rejection
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn validate_config_file_rejects_symlink() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let target = dir.path().join("real.toml");
        let link = dir.path().join("link.toml");

        fs::write(&target, "key = \"value\"").expect("write should succeed");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("symlink should succeed");

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, std::os::windows::fs::symlink_file requires
            // admin privileges, so we skip this test.
            return;
        }

        let result = validate_config_file(&link);
        assert!(result.is_err(), "symlink should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("symbolic link"),
            "Error should mention symbolic link, got: {err_msg}"
        );
    }

    // ------------------------------------------------------------------
    // validate_config_file: max size check
    // ------------------------------------------------------------------

    #[test]
    fn validate_config_file_rejects_oversized_file() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let file_path = dir.path().join("big.toml");

        // Write slightly over 1 MB
        let big_content = "x".repeat(MAX_CONFIG_FILE_SIZE + 1);
        fs::write(&file_path, big_content).expect("write should succeed");

        let result = validate_config_file(&file_path);
        assert!(result.is_err(), "oversized file should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("exceeds maximum size"),
            "Error should mention file size, got: {err_msg}"
        );
        assert!(
            err_msg.contains(&MAX_CONFIG_FILE_SIZE.to_string()),
            "Error should include max size value, got: {err_msg}"
        );
    }

    #[test]
    fn validate_config_file_accepts_normal_file() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let file_path = dir.path().join("normal.toml");

        fs::write(file_path, "key = \"value\"").expect("write should succeed");

        // Re-read the path to get a fresh reference
        let file_path = dir.path().join("normal.toml");
        let result = validate_config_file(&file_path);
        assert!(result.is_ok(), "normal file should be accepted");
    }

    #[test]
    fn validate_config_file_accepts_file_at_exact_max_size() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let file_path = dir.path().join("exact.toml");

        // Write exactly 1 MB
        let content = "x".repeat(MAX_CONFIG_FILE_SIZE);
        fs::write(&file_path, content).expect("write should succeed");

        let file_path = dir.path().join("exact.toml");
        let result = validate_config_file(&file_path);
        assert!(
            result.is_ok(),
            "file at exactly max size should be accepted"
        );
    }

    // ------------------------------------------------------------------
    // validate_config_file: missing file
    // ------------------------------------------------------------------

    #[test]
    fn validate_config_file_errors_on_missing_file() {
        let result = validate_config_file(std::path::Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err(), "missing file should return error");
    }

    // ------------------------------------------------------------------
    // validate_config_file: dead symlink
    // ------------------------------------------------------------------

    #[test]
    fn validate_config_file_errors_on_dead_symlink() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().expect("tempdir should succeed");
            let dead_target = dir.path().join("does_not_exist.toml");
            let link = dir.path().join("dead.toml");

            std::os::unix::fs::symlink(&dead_target, &link)
                .expect("symlink creation should succeed");

            let result = validate_config_file(&link);
            assert!(result.is_err(), "dead symlink should return error");
            let err_msg = format!("{result:?}");
            assert!(
                err_msg.contains("dead symlink"),
                "Error should mention dead symlink, got: {err_msg}"
            );
        }
        #[cfg(not(unix))]
        {
            // On non-Unix platforms, symlink_file requires admin privileges.
        }
    }

    // ------------------------------------------------------------------
    // DeadSymlink error kind exit code
    // ------------------------------------------------------------------

    #[test]
    fn dead_symlink_error_kind_has_correct_exit_code() {
        use crate::error_config::ConfigErrorKind;
        let err: crate::error_config::ConfigError =
            ConfigErrorKind::DeadSymlink("test".to_string()).into();
        assert_eq!(err.exit_code(), 98);
    }

    // ------------------------------------------------------------------
    // collect_config_paths
    // ------------------------------------------------------------------

    #[test]
    fn collect_config_paths_includes_existing_global() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let global_path = dir.path().join("config.toml");
        fs::write(&global_path, "key = \"value\"").expect("write should succeed");

        let manager = ConfigManager::with_paths(global_path.clone(), None);
        let paths = collect_config_paths(&manager);

        assert!(
            paths.contains(&global_path),
            "should include existing global path"
        );
    }

    #[test]
    fn collect_config_paths_includes_project_when_set() {
        let global_dir = tempfile::tempdir().expect("tempdir should succeed");
        let project_dir = tempfile::tempdir().expect("tempdir should succeed");
        let global_path = global_dir.path().join("config.toml");
        let project_path = project_dir.path().join(".scp").join("config");

        fs::create_dir_all(project_path.parent().expect("parent should exist"))
            .expect("mkdir should succeed");
        fs::write(&global_path, "key = \"value\"").expect("write should succeed");
        fs::write(&project_path, "key2 = \"value2\"").expect("write should succeed");

        let manager = ConfigManager::with_paths(global_path, Some(project_path.clone()));
        let paths = collect_config_paths(&manager);

        assert!(
            paths.contains(&project_path),
            "should include existing project path"
        );
    }

    // ------------------------------------------------------------------
    // HotReloadConfigManager: creation and get
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn hot_reload_config_manager_creates_and_gets() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let global_path = dir.path().join("config.toml");
        fs::write(&global_path, "logging.level = \"debug\"").expect("write should succeed");

        let manager = ConfigManager::with_paths(global_path, None);
        let hot = HotReloadConfigManager::with_manager(manager)
            .await
            .expect("HotReloadConfigManager should be created");

        let config = hot.get().await;
        assert_eq!(
            config.get("logging.level"),
            Some(&"debug".to_string()),
            "should load initial config"
        );
    }

    #[tokio::test]
    async fn hot_reload_config_manager_is_clone() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let global_path = dir.path().join("config.toml");
        fs::write(&global_path, "key = \"value\"").expect("write should succeed");

        let manager = ConfigManager::with_paths(global_path, None);
        let hot = HotReloadConfigManager::with_manager(manager)
            .await
            .expect("HotReloadConfigManager should be created");

        let _hot2 = hot.clone();
        // Both clones should point to the same inner data
        let config = hot.get().await;
        assert_eq!(config.get("key"), Some(&"value".to_string()));
    }

    // ------------------------------------------------------------------
    // Security error kinds
    // ------------------------------------------------------------------

    #[test]
    fn security_symlink_error_kind_has_correct_exit_code() {
        use crate::error_config::ConfigErrorKind;
        let err: crate::error_config::ConfigError =
            ConfigErrorKind::SecuritySymlink("test".to_string()).into();
        assert_eq!(err.exit_code(), 95);
    }

    #[test]
    fn file_too_large_error_kind_has_correct_exit_code() {
        use crate::error_config::ConfigErrorKind;
        let err: crate::error_config::ConfigError =
            ConfigErrorKind::FileTooLarge("test".to_string()).into();
        assert_eq!(err.exit_code(), 96);
    }

    #[test]
    fn watcher_error_kind_has_correct_exit_code() {
        use crate::error_config::ConfigErrorKind;
        let err: crate::error_config::ConfigError =
            ConfigErrorKind::WatcherError("test".to_string()).into();
        assert_eq!(err.exit_code(), 97);
    }
}
