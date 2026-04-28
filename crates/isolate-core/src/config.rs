//! Configuration loading and management
//!
//! # Hierarchy
//!
//! Configuration is loaded in this order (later overrides earlier):
//! 1. Built-in defaults
//! 2. Global config: ~/.config/isolate/config.toml
//! 3. Project config: .isolate/config.toml
//! 4. Environment variables: Isolate_*
//! 5. CLI flags (command-specific)
//!
//! # Hot-Reload
//!
//! For long-running commands that need hot-reload, use [`ConfigManager`]
//! to get automatic config reloading when files change.
//!
//! # Example Config
//!
//! ```toml
//! workspace_dir = "../{repo}__workspaces"
//! main_branch = "main"
//!
//! [watch]
//! enabled = true
//! debounce_ms = 100
//!
//! [hooks]
//! post_create = ["br sync", "npm install"]
//!
//! [conflict_resolution]
//! mode = "hybrid"
//! autonomy = 60
//! security_keywords = ["password", "token", "secret"]
//! log_resolutions = true
//! ```

use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

// Import notify for Watcher trait
use notify::Watcher;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use crate::{Error, Result};

// Conflict resolution configuration
pub mod conflict_resolution;
pub use conflict_resolution::{
    ConflictMode, ConflictResolutionConfig, PartialConflictResolutionConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryPolicy {
    Silent,
    #[default]
    Warn,
    FailFast,
}

impl<'de> Deserialize<'de> for RecoveryPolicy {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for RecoveryPolicy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "silent" => Ok(Self::Silent),
            "warn" => Ok(Self::Warn),
            "fail-fast" | "failfast" | "fail" => Ok(Self::FailFast),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid recovery policy: {s}. Must be one of: silent, warn, fail-fast"
            ))),
        }
    }
}

impl std::fmt::Display for RecoveryPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Silent => write!(f, "silent"),
            Self::Warn => write!(f, "warn"),
            Self::FailFast => write!(f, "fail-fast"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATED BOOLEAN TYPE - rejects string values for boolean fields
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(from = "bool", into = "bool")]
pub struct ValidatedBool(bool);

impl ValidatedBool {
    /// Get the underlying boolean value.
    #[must_use]
    #[inline]
    pub const fn as_bool(self) -> bool {
        self.0
    }
}

impl From<bool> for ValidatedBool {
    fn from(b: bool) -> Self {
        Self(b)
    }
}

impl From<ValidatedBool> for bool {
    fn from(v: ValidatedBool) -> Self {
        v.0
    }
}

impl<'de> serde::Deserialize<'de> for ValidatedBool {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BoolVisitor;

        impl serde::de::Visitor<'_> for BoolVisitor {
            type Value = ValidatedBool;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean value (true or false)")
            }

            fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ValidatedBool(v))
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Str(v),
                    &self,
                ))
            }

            fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Str(&v),
                    &self,
                ))
            }
        }

        deserializer.deserialize_bool(BoolVisitor)
    }
}

impl FromStr for ValidatedBool {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "true" | "1" => Ok(Self(true)),
            "false" | "0" => Ok(Self(false)),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid boolean value: '{s}'. Must be 'true' or 'false'"
            ))),
        }
    }
}

impl std::ops::Not for ValidatedBool {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.0
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub workspace_dir: String,
    pub main_branch: String,
    pub state_db: String,
    pub watch: WatchConfig,
    pub hooks: HooksConfig,
    pub agent: AgentConfig,
    pub session: SessionConfig,
    pub recovery: RecoveryConfig,
    pub conflict_resolution: ConflictResolutionConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RecoveryConfig {
    #[serde(default = "default_recovery_policy")]
    pub policy: RecoveryPolicy,
    #[serde(default = "default_true")]
    pub log_recovered: ValidatedBool,
    #[serde(default = "default_true")]
    pub auto_recover_corrupted_wal: ValidatedBool,
    #[serde(default = "default_false")]
    pub delete_corrupted_database: ValidatedBool,
}

const fn default_recovery_policy() -> RecoveryPolicy {
    RecoveryPolicy::Warn
}

const fn default_true() -> ValidatedBool {
    ValidatedBool(true)
}

const fn default_false() -> ValidatedBool {
    ValidatedBool(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchConfig {
    pub enabled: ValidatedBool,
    pub debounce_ms: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HooksConfig {
    pub post_create: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_merge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub command: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    pub auto_commit: ValidatedBool,
    pub commit_prefix: String,
    pub max_sessions: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_dir: "../{repo}__workspaces".to_string(),
            main_branch: String::new(),
            state_db: ".isolate/state.db".to_string(),
            watch: WatchConfig::default(),
            hooks: HooksConfig::default(),
            agent: AgentConfig::default(),
            session: SessionConfig::default(),
            recovery: RecoveryConfig::default(),
            conflict_resolution: ConflictResolutionConfig::default(),
        }
    }
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            enabled: ValidatedBool(true),
            debounce_ms: 100,
            paths: vec![".beads/beads.db".to_string()],
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            post_create: Vec::new(),
            pre_remove: Vec::new(),
            post_merge: Vec::new(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            env: HashMap::new(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_commit: ValidatedBool(false),
            commit_prefix: "wip:".to_string(),
            max_sessions: 100,
        }
    }
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            policy: RecoveryPolicy::Warn,
            log_recovered: ValidatedBool(true),
            auto_recover_corrupted_wal: ValidatedBool(true),
            delete_corrupted_database: ValidatedBool(false),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARTIAL CONFIG STRUCTURES (explicit-key merge semantics)
// ═══════════════════════════════════════════════════════════════════════════

/// Partial configuration with `Option<T>` fields for explicit-key merge semantics.
///
/// When loading a config file, only fields explicitly present in the TOML
/// will be `Some(value)`. Missing fields remain `None` and won't override
/// lower-precedence config values during merge.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialConfig {
    #[serde(default)]
    pub workspace_dir: Option<String>,
    #[serde(default)]
    pub main_branch: Option<String>,
    #[serde(default)]
    pub state_db: Option<String>,
    #[serde(default)]
    pub watch: Option<PartialWatchConfig>,
    #[serde(default)]
    pub hooks: Option<PartialHooksConfig>,
    #[serde(default)]
    pub agent: Option<PartialAgentConfig>,
    #[serde(default)]
    pub session: Option<PartialSessionConfig>,
    #[serde(default)]
    pub recovery: Option<PartialRecoveryConfig>,
    #[serde(default)]
    pub conflict_resolution: Option<PartialConflictResolutionConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialWatchConfig {
    #[serde(default)]
    pub enabled: Option<ValidatedBool>,
    #[serde(default)]
    pub debounce_ms: Option<u32>,
    #[serde(default)]
    pub paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialHooksConfig {
    #[serde(default)]
    pub post_create: Option<Vec<String>>,
    #[serde(default)]
    pub pre_remove: Option<Vec<String>>,
    #[serde(default)]
    pub post_merge: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialAgentConfig {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialSessionConfig {
    #[serde(default)]
    pub auto_commit: Option<ValidatedBool>,
    #[serde(default)]
    pub commit_prefix: Option<String>,
    #[serde(default)]
    pub max_sessions: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialRecoveryConfig {
    #[serde(default)]
    pub policy: Option<RecoveryPolicy>,
    #[serde(default)]
    pub log_recovered: Option<ValidatedBool>,
    #[serde(default)]
    pub auto_recover_corrupted_wal: Option<ValidatedBool>,
    #[serde(default)]
    pub delete_corrupted_database: Option<ValidatedBool>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG MANAGER (HOT-RELOAD)
// ═══════════════════════════════════════════════════════════════════════════

/// Manages configuration with hot-reload capability
///
/// This type provides thread-safe, reloadable configuration for long-running
/// processes. It watches config files and automatically reloads when they change.
///
/// # Example
///
/// ```rust,no_run
/// use isolate_core::config::ConfigManager;
///
/// # async fn example() -> isolate_core::Result<()> {
/// let manager = ConfigManager::new().await?;
///
/// // Get current config (fast, non-blocking read)
/// let config = manager.get().await;
/// println!("Workspace dir: {}", config.workspace_dir);
///
/// // Config auto-reloads when files change
/// tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
/// let updated_config = manager.get().await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ConfigManager {
    inner: Arc<RwLock<ConfigManagerInner>>,
}

struct ConfigManagerInner {
    config: Config,
}

impl ConfigManager {
    /// Create a new `ConfigManager` with hot-reload enabled
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Initial config load fails
    /// - Unable to spawn reload task
    pub async fn new() -> Result<Self> {
        let config = load_config().await?;

        let manager = Self {
            inner: Arc::new(RwLock::new(ConfigManagerInner { config })),
        };

        // Spawn config watcher task
        let inner = manager.inner.clone();
        let mut file_watcher_rx = Self::watch_config_files();

        tokio::spawn(async move {
            // Loop: watch for file changes and reload config
            loop {
                tokio::select! {
                    // File changed - reload config
                    Some(()) = file_watcher_rx.recv() => {
                        // Debounce: small delay before reload
                        tokio::time::sleep(Duration::from_millis(150)).await;

                        // Reload config
                        match load_config().await {
                            Ok(new_config) => {
                                {
                                    let mut inner_write = inner.write().await;
                                    inner_write.config = new_config;
                                } // Drop guard before logging
                                tracing::info!("Config reloaded successfully");
                            }
                            Err(e) => {
                                // Log error but keep running with last known good config
                                tracing::warn!("Config reload failed: {e}, using previous config");
                            }
                        }
                    }
                    // Channel closed - exit task
                    else => break,
                }
            }
        });

        Ok(manager)
    }

    /// Get the current configuration
    ///
    /// This is a fast, non-blocking read that returns the most recent
    /// successfully loaded configuration (including hot-reloaded changes).
    pub async fn get(&self) -> Config {
        let inner = self.inner.read().await;
        inner.config.clone()
    }

    /// Create a config file watcher channel
    ///
    /// Returns a receiver that gets events when config files change.
    fn watch_config_files() -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel::<()>(4);

        tokio::spawn(async move {
            let paths_to_watch = Self::get_config_paths();

            if paths_to_watch.is_empty() {
                return;
            }

            // Use notify to watch config files
            let result = notify::recommended_watcher(
                move |res: std::result::Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        if event.kind.is_modify() || event.kind.is_create() {
                            let _ = tx.blocking_send(());
                        }
                    }
                },
            );

            let Ok(mut watcher) = result else { return };

            // Watch each config path
            for path in paths_to_watch {
                // Try watching the file directly first, fall back to parent on error
                let watch_result = watcher.watch(&path, notify::RecursiveMode::NonRecursive);

                // If file doesn't exist, watch parent directory for creation
                if watch_result.is_err() {
                    if let Some(parent) = path.parent() {
                        let _ = watcher.watch(parent, notify::RecursiveMode::NonRecursive);
                    }
                }
            }

            // Keep the watcher task alive
            tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
        });

        rx
    }

    /// Get paths to config files that should be watched
    fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Global config
        if let Ok(global) = global_config_path() {
            paths.push(global);
        }

        // Project config
        if let Ok(project) = project_config_path() {
            paths.push(project);
        }

        paths
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG KEY VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

/// All valid configuration keys in dot-notation
///
/// This list defines the complete schema of supported configuration keys.
/// Any key not in this list will be rejected with a helpful error message.
const VALID_KEYS: &[&str] = &[
    "workspace_dir",
    "main_branch",
    "state_db",
    "watch",
    "hooks",
    "agent",
    "session",
    "recovery",
    "conflict_resolution",
    "watch.enabled",
    "watch.debounce_ms",
    "watch.paths",
    "hooks.post_create",
    "hooks.pre_remove",
    "hooks.post_merge",
    "agent.command",
    "agent.env",
    "session.auto_commit",
    "session.commit_prefix",
    "recovery.policy",
    "recovery.log_recovered",
    "recovery.auto_recover_corrupted_wal",
    "recovery.delete_corrupted_database",
    "conflict_resolution.mode",
    "conflict_resolution.autonomy",
    "conflict_resolution.security_keywords",
    "conflict_resolution.log_resolutions",
];

/// Validate a configuration key
///
/// Checks if the given key is in the list of valid configuration keys.
/// Returns an error if the key is not recognized.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the key is not valid.
/// The error message includes a list of valid keys to help the user.
///
/// # Examples
///
/// ```rust
/// use isolate_core::config::validate_key;
///
/// assert!(validate_key("workspace_dir").is_ok());
/// assert!(validate_key("watch.enabled").is_ok());
/// assert!(validate_key("invalid_key").is_err());
/// ```
pub fn validate_key(key: &str) -> Result<()> {
    // Check if the key exactly matches a valid key or is a parent of a valid key
    // For example:
    // - "watch" is valid (parent of watch.enabled, watch.debounce_ms, etc.)
    // - "watch.enabled" is valid (exact match)
    // - "watch.invalid" is invalid (not in list)
    let is_valid = VALID_KEYS
        .iter()
        .any(|valid_key| key == *valid_key || valid_key.starts_with(&format!("{key}.")));

    if is_valid {
        Ok(())
    } else {
        // Build a helpful error message with valid keys grouped by category
        let mut error_msg = format!("Unknown configuration key: '{key}'\n\n");

        error_msg.push_str("  workspace_dir, main_branch, state_db\n");
        error_msg.push_str("  watch.enabled, watch.debounce_ms, watch.paths\n");
        error_msg.push_str("  hooks.post_create, hooks.pre_remove, hooks.post_merge\n");
        error_msg.push_str("  agent.command, agent.env\n");
        error_msg.push_str("  session.auto_commit, session.commit_prefix, session.max_sessions\n");
        error_msg.push_str("  recovery.policy, recovery.log_recovered, recovery.auto_recover_corrupted_wal, recovery.delete_corrupted_database\n");
        error_msg.push_str("  conflict_resolution.mode, conflict_resolution.autonomy, conflict_resolution.security_keywords, conflict_resolution.log_resolutions\n");
        error_msg.push_str("\nUse 'isolate config' to see current configuration.");

        Err(Error::ValidationError {
            message: error_msg,
            field: None,
            value: None,
            constraints: Vec::new(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Load configuration from all sources with hierarchy
///
/// For long-running processes that need hot-reload, use [`ConfigManager`] instead.
///
/// # Errors
///
/// Returns error if:
/// - Config file is malformed TOML
/// - Config values fail validation
/// - Unable to determine repository name for placeholder substitution
pub async fn load_config() -> Result<Config> {
    // 1. Start with built-in defaults
    let mut config = Config::default();

    // 2. Load global config if exists (try-load pattern eliminates TOCTTOU race)
    // Use partial config merge to preserve precedence - only explicitly set
    // fields in config files will override defaults
    if let Ok(global_path) = global_config_path() {
        match load_partial_toml_file(&global_path).await {
            Ok(global) => config.merge_partial(global),
            Err(Error::IoError(_)) => {
                // Config file doesn't exist - skip silently
            }
            Err(e) => return Err(e),
        }
    }

    // 3. Load project config if exists (try-load pattern eliminates TOCTTOU race)
    // Use partial config merge to preserve precedence - only explicitly set
    // fields in project config will override global/defaults
    if let Ok(project_path) = project_config_path() {
        match load_partial_toml_file(&project_path).await {
            Ok(project) => config.merge_partial(project),
            Err(Error::IoError(_)) => {
                // Config file doesn't exist - skip silently
            }
            Err(e) => return Err(e),
        }
    }

    // 4. Apply environment variable overrides
    config.apply_env_vars()?;

    // 5. Validate and substitute placeholders
    config.validate()?;

    // Only attempt placeholder substitution if we're in a proper directory structure
    // This prevents failures in test environments where current_dir might not be valid
    if get_repo_name().is_ok() {
        config.substitute_placeholders()?;
    }
    // In test environments, we might not have a proper repo name
    // Just use defaults without placeholder substitution

    Ok(config)
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to project config file
///
/// # Errors
///
/// Returns error if current directory cannot be determined
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".isolate/config.toml"))
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
}

/// Get path to global config file
fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "isolate")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| Error::IoError("Failed to determine global config directory".to_string()))
}

/// Get repository name from current directory
///
/// # Errors
///
/// Returns error if:
/// - Current directory cannot be determined
/// - Directory name cannot be extracted
fn get_repo_name() -> Result<String> {
    let dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))?;

    dir.file_name()
        .and_then(|name| name.to_str())
        .map(String::from)
        .ok_or_else(|| Error::Unknown("Failed to determine repository name".to_string()))
}

/// Load a TOML file into a partial Config
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - TOML is malformed
#[allow(dead_code)]
async fn load_toml_file(path: &std::path::Path) -> Result<Config> {
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    toml::from_str(&content)
        .map_err(|e| Error::ParseError(format!("Failed to parse config: {}: {e}", path.display())))
}

/// Load a TOML file into a `PartialConfig` for explicit-key merge semantics.
///
/// Only fields explicitly present in the TOML will be `Some(value)`.
/// Missing fields remain `None` and won't override lower-precedence config values.
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - TOML is malformed
/// - Unknown configuration keys are present (typos will be rejected)
const MAX_CONFIG_FILE_SIZE: usize = 1_048_576; // 1 MB

pub async fn load_partial_toml_file(path: &std::path::Path) -> Result<PartialConfig> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file metadata {}: {e}",
            path.display()
        ))
    })?;

    if metadata.is_symlink() {
        return Err(Error::ValidationError {
            message: format!(
                "Config file {} is a symbolic link - refusing to follow for security",
                path.display()
            ),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }

    if metadata.len() as usize > MAX_CONFIG_FILE_SIZE {
        return Err(Error::ValidationError {
            message: format!(
                "Config file {} exceeds maximum size of {} bytes",
                path.display(),
                MAX_CONFIG_FILE_SIZE
            ),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }

    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    let value: toml::Value = toml::from_str(&content).map_err(|e| {
        Error::ParseError(format!(
            "Failed to parse config file {}: {e}",
            path.display()
        ))
    })?;

    validate_toml_keys(&value, "")?;

    toml::from_str(&content)
        .map_err(|e| Error::ParseError(format!("Failed to parse config: {}: {e}", path.display())))
}

/// Extract all keys from a TOML value in dot-notation format
///
/// For example, `{ watch = { enabled = true } }` produces `["watch", "watch.enabled"]`
fn extract_keys(value: &toml::Value, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();

    if let toml::Value::Table(table) = value {
        for (key, val) in table {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };

            keys.push(full_key.clone());

            if let toml::Value::Table(_) = val {
                keys.extend(extract_keys(val, &full_key));
            }
        }
    }

    keys
}

/// Validate all keys in a TOML document against known configuration keys
///
/// # Errors
///
/// Returns `Error::ValidationError` if any unknown key is found
fn validate_toml_keys(value: &toml::Value, _prefix: &str) -> Result<()> {
    let keys = extract_keys(value, "");

    for key in keys {
        validate_key(&key)?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// MERGE TRAIT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl WatchConfig {
    #[allow(dead_code)]
    fn merge(&mut self, other: Self) {
        self.enabled = other.enabled;
        self.debounce_ms = other.debounce_ms;
        self.paths = other.paths;
    }
}

impl HooksConfig {
    #[allow(dead_code)]
    fn merge(&mut self, other: Self) {
        self.post_create = other.post_create;
        self.pre_remove = other.pre_remove;
        self.post_merge = other.post_merge;
    }
}

impl AgentConfig {
    #[allow(dead_code)]
    fn merge(&mut self, other: Self) {
        self.command = other.command;
        self.env = other.env;
    }
}

impl SessionConfig {
    #[allow(dead_code)]
    fn merge(&mut self, other: Self) {
        self.auto_commit = other.auto_commit;
        self.commit_prefix = other.commit_prefix;
        self.max_sessions = other.max_sessions;
    }
}

impl RecoveryConfig {
    #[allow(dead_code)]
    #[allow(clippy::missing_const_for_fn)]
    fn merge(&mut self, other: Self) {
        self.policy = other.policy;
        self.log_recovered = other.log_recovered;
        self.auto_recover_corrupted_wal = other.auto_recover_corrupted_wal;
        self.delete_corrupted_database = other.delete_corrupted_database;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARTIAL MERGE IMPLEMENTATIONS (explicit-key merge semantics)
// ═══════════════════════════════════════════════════════════════════════════

impl WatchConfig {
    /// Merge partial config, only updating fields that are `Some(value)`.
    fn merge_partial(&mut self, partial: PartialWatchConfig) {
        if let Some(enabled) = partial.enabled {
            self.enabled = enabled;
        }
        if let Some(debounce_ms) = partial.debounce_ms {
            self.debounce_ms = debounce_ms;
        }
        if let Some(paths) = partial.paths {
            self.paths = paths;
        }
    }
}

impl HooksConfig {
    fn merge_partial(&mut self, partial: PartialHooksConfig) {
        if let Some(post_create) = partial.post_create {
            self.post_create = post_create;
        }
        if let Some(pre_remove) = partial.pre_remove {
            self.pre_remove = pre_remove;
        }
        if let Some(post_merge) = partial.post_merge {
            self.post_merge = post_merge;
        }
    }
}

impl AgentConfig {
    /// Merge partial config, only updating fields that are `Some(value)`.
    fn merge_partial(&mut self, partial: PartialAgentConfig) {
        if let Some(command) = partial.command {
            self.command = command;
        }
        if let Some(env) = partial.env {
            self.env = env;
        }
    }
}

impl SessionConfig {
    /// Merge partial config, only updating fields that are `Some(value)`.
    fn merge_partial(&mut self, partial: PartialSessionConfig) {
        if let Some(auto_commit) = partial.auto_commit {
            self.auto_commit = auto_commit;
        }
        if let Some(commit_prefix) = partial.commit_prefix {
            self.commit_prefix = commit_prefix;
        }
        if let Some(max_sessions) = partial.max_sessions {
            self.max_sessions = max_sessions;
        }
    }
}

impl RecoveryConfig {
    /// Merge partial config, only updating fields that are `Some(value)`.
    #[allow(clippy::missing_const_for_fn)]
    #[allow(clippy::needless_pass_by_value)]
    fn merge_partial(&mut self, partial: PartialRecoveryConfig) {
        if let Some(policy) = partial.policy {
            self.policy = policy;
        }
        if let Some(log_recovered) = partial.log_recovered {
            self.log_recovered = log_recovered;
        }
        if let Some(auto_recover_corrupted_wal) = partial.auto_recover_corrupted_wal {
            self.auto_recover_corrupted_wal = auto_recover_corrupted_wal;
        }
        if let Some(delete_corrupted_database) = partial.delete_corrupted_database {
            self.delete_corrupted_database = delete_corrupted_database;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG METHODS
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Merge another config into this one (other takes precedence)
    ///
    /// Note: This performs a deep replacement merge, not append.
    /// For example, if `hooks.post_create` is `["a","b"]` in self and `["c"]` in other,
    /// result will be `["c"]`, not `["a","b","c"]`.
    #[allow(dead_code)]
    fn merge(&mut self, other: Self) {
        // Top-level string fields - replace always (other takes precedence)
        // We don't check against defaults here because an explicit project config
        // should override a global config even if the project config matches the default.
        self.workspace_dir = other.workspace_dir;
        self.main_branch = other.main_branch;
        self.state_db = other.state_db;

        // Merge nested configs
        self.watch.merge(other.watch);
        self.hooks.merge(other.hooks);
        self.agent.merge(other.agent);
        self.session.merge(other.session);
        self.recovery.merge(other.recovery);
    }

    /// Merge partial config into this one using explicit-key semantics.
    ///
    /// Only fields that are `Some(value)` in the partial config will override
    /// the corresponding fields in self. Fields that are `None` in the partial
    /// config will NOT reset the values in self.
    ///
    /// This is the key to proper config layering: a partial config file that
    /// only specifies `main_branch = "develop"` will NOT reset `workspace_dir`
    /// or any other fields to their defaults.
    pub fn merge_partial(&mut self, partial: PartialConfig) {
        // Top-level string fields - only override if explicitly set
        if let Some(workspace_dir) = partial.workspace_dir {
            self.workspace_dir = workspace_dir;
        }
        if let Some(main_branch) = partial.main_branch {
            self.main_branch = main_branch;
        }
        if let Some(state_db) = partial.state_db {
            self.state_db = state_db;
        }

        // Merge nested configs only if present
        if let Some(watch) = partial.watch {
            self.watch.merge_partial(watch);
        }
        if let Some(hooks) = partial.hooks {
            self.hooks.merge_partial(hooks);
        }
        if let Some(agent) = partial.agent {
            self.agent.merge_partial(agent);
        }
        if let Some(session) = partial.session {
            self.session.merge_partial(session);
        }
        if let Some(recovery) = partial.recovery {
            self.recovery.merge_partial(recovery);
        }
        if let Some(conflict_resolution) = partial.conflict_resolution {
            self.conflict_resolution.merge_partial(conflict_resolution);
        }
    }

    /// Apply environment variable overrides
    ///
    /// # Errors
    ///
    /// Returns error if environment variable values are invalid
    fn apply_env_vars(&mut self) -> Result<()> {
        // Isolate_WORKSPACE_DIR
        if let Ok(value) = std::env::var("Isolate_WORKSPACE_DIR") {
            self.workspace_dir = value;
        }

        // Isolate_MAIN_BRANCH
        if let Ok(value) = std::env::var("Isolate_MAIN_BRANCH") {
            self.main_branch = value;
        }

        // Isolate_WATCH_ENABLED
        if let Ok(value) = std::env::var("Isolate_WATCH_ENABLED") {
            self.watch.enabled = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid Isolate_WATCH_ENABLED value: {e}"))
            })?;
        }

        // Isolate_WATCH_DEBOUNCE_MS
        if let Ok(value) = std::env::var("Isolate_WATCH_DEBOUNCE_MS") {
            self.watch.debounce_ms = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid Isolate_WATCH_DEBOUNCE_MS value: {e}"))
            })?;
        }

        // Isolate_AGENT_COMMAND
        if let Ok(value) = std::env::var("Isolate_AGENT_COMMAND") {
            self.agent.command = value;
        }

        // Isolate_RECOVERY_POLICY
        if let Ok(value) = std::env::var("Isolate_RECOVERY_POLICY") {
            self.recovery.policy = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid Isolate_RECOVERY_POLICY value: {e}"))
            })?;
        }

        // Isolate_RECOVERY_LOG
        if let Ok(value) = std::env::var("Isolate_RECOVERY_LOG") {
            self.recovery.log_recovered = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid Isolate_RECOVERY_LOG value: {e}"))
            })?;
        }

        // Isolate_CONFLICT_RESOLUTION_MODE
        if let Ok(value) = std::env::var("Isolate_CONFLICT_RESOLUTION_MODE") {
            self.conflict_resolution.mode = value.parse().map_err(|e| {
                Error::InvalidConfig(format!(
                    "Invalid Isolate_CONFLICT_RESOLUTION_MODE value: {e}"
                ))
            })?;
        }

        // Isolate_CONFLICT_RESOLUTION_AUTONOMY
        if let Ok(value) = std::env::var("Isolate_CONFLICT_RESOLUTION_AUTONOMY") {
            self.conflict_resolution.autonomy = value.parse().map_err(|e| {
                Error::InvalidConfig(format!(
                    "Invalid Isolate_CONFLICT_RESOLUTION_AUTONOMY value: {e}"
                ))
            })?;
        }

        // Isolate_CONFLICT_RESOLUTION_LOG_RESOLUTIONS
        if let Ok(value) = std::env::var("Isolate_CONFLICT_RESOLUTION_LOG_RESOLUTIONS") {
            self.conflict_resolution.log_resolutions = value.parse().map_err(|e| {
                Error::InvalidConfig(format!(
                    "Invalid Isolate_CONFLICT_RESOLUTION_LOG_RESOLUTIONS value: {e}"
                ))
            })?;
        }

        // Isolate_CONFLICT_RESOLUTION_SECURITY_KEYWORDS (comma-separated)
        if let Ok(value) = std::env::var("Isolate_CONFLICT_RESOLUTION_SECURITY_KEYWORDS") {
            self.conflict_resolution.security_keywords = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        Ok(())
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns error if any values are out of range or invalid
    fn validate(&self) -> Result<()> {
        // Validate debounce_ms range [10-5000]
        if self.watch.debounce_ms < 10 || self.watch.debounce_ms > 5000 {
            return Err(Error::ValidationError {
                message: "debounce_ms must be 10-5000".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        // Validate conflict resolution config
        self.conflict_resolution.validate()?;

        Ok(())
    }

    /// Substitute placeholders like {repo} in config values
    ///
    /// # Errors
    ///
    /// Returns error if unable to determine values for placeholders
    fn substitute_placeholders(&mut self) -> Result<()> {
        let repo_name = get_repo_name()?;
        self.workspace_dir = self.workspace_dir.replace("{repo}", &repo_name);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: No config files - Returns default config
    #[tokio::test]
    async fn test_no_config_files_returns_defaults() {
        // This test works in the normal repo context where no .isolate/config.toml exists
        // Note: Global config may exist and override defaults
        let result = load_config().await;
        assert!(
            result.is_ok(),
            "load_config should succeed even without config files"
        );

        #[allow(clippy::unnecessary_result_map_or_else)]
        let config = result.map_or_else(|_| Config::default(), |c| c);
        // Check that we got a valid config (global config may override workspace_dir)
        assert!(!config.workspace_dir.is_empty());
        // state_db may be overridden by global config
        assert!(!config.state_db.is_empty());
    }

    // Test 2: Global only - Loads global, merges with defaults
    #[test]
    fn test_global_only_merges_with_defaults() {
        // For this test, we're testing the merge logic directly, not the file loading
        let mut base = Config::default();
        let override_config = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.workspace_dir, "../custom");
    }

    // Test 3: Project only - Loads project, merges with defaults
    #[test]
    fn test_project_only_merges_with_defaults() {
        let mut base = Config::default();
        let override_config = Config {
            main_branch: "develop".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.main_branch, "develop");
        assert_eq!(base.workspace_dir, "../{repo}__workspaces"); // Should still have default
    }

    // Test 4: Both - Project overrides global overrides defaults
    #[test]
    fn test_project_overrides_global() {
        let mut base = Config::default();

        // First merge global
        let global_config = Config {
            workspace_dir: "../global".to_string(),
            ..Default::default()
        };
        base.merge(global_config);
        assert_eq!(base.workspace_dir, "../global");

        // Then merge project (should override)
        let project_config = Config {
            workspace_dir: "../project".to_string(),
            ..Default::default()
        };
        base.merge(project_config);

        assert_eq!(base.workspace_dir, "../project");
    }

    // Test 5: Env override - Isolate_WORKSPACE_DIR=../custom → config.workspace_dir
    #[test]
    fn test_env_var_overrides_config() {
        // Set env var
        std::env::set_var("Isolate_WORKSPACE_DIR", "../env");

        let mut config = Config {
            workspace_dir: "../original".to_string(),
            ..Default::default()
        };

        let result = config.apply_env_vars();
        assert!(result.is_ok());

        assert_eq!(config.workspace_dir, "../env");

        // Cleanup
        std::env::remove_var("Isolate_WORKSPACE_DIR");
    }

    // Test 6: Placeholder substitution
    #[test]
    fn test_placeholder_substitution() {
        let mut config = Config {
            workspace_dir: "../{repo}__ws".to_string(),
            ..Default::default()
        };

        let result = config.substitute_placeholders();
        assert!(result.is_ok());

        // The repo name will be "isolate" since we're in the isolate directory
        assert!(config.workspace_dir.contains("__ws"));
        assert!(!config.workspace_dir.contains("{repo}"));
    }

    // Test 7: Invalid debounce - debounce_ms = 5 → Error
    #[test]
    fn test_invalid_debounce_ms_too_low() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("10-5000"));
        }
    }

    // Test 9: Missing global config - No error, uses defaults
    #[tokio::test]
    async fn test_missing_global_config_no_error() {
        // This tests that load_config doesn't fail when global config doesn't exist
        // (which is the normal case for most users)
        let result = load_config().await;
        assert!(result.is_ok());
    }

    // Test 10: Malformed TOML - Clear error with line number
    #[tokio::test]
    async fn test_malformed_toml_returns_parse_error() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_config.toml");

        // Use async file operations
        tokio::fs::write(&config_path, b"workspace_dir = \n invalid toml [[[")
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, Error::ParseError(_)));
        }
        Ok(())
    }

    // Test: Unknown keys should be rejected
    #[tokio::test]
    async fn test_unknown_keys_rejected() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("unknown_key_config.toml");

        let content = r#"workspace_dir = "../test"
typo_key = "invalid""#;
        tokio::fs::write(&config_path, content.as_bytes())
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_partial_toml_file(&config_path).await;
        assert!(result.is_err(), "Unknown keys should be rejected");

        if let Err(e) = &result {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("Unknown configuration key") || err_str.contains("typo_key"),
                "Error should mention unknown key: {err_str}"
            );
        }
        Ok(())
    }

    // Test: Unknown nested keys should be rejected
    #[tokio::test]
    async fn test_unknown_nested_keys_rejected() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("unknown_nested_config.toml");

        let content = r#"[watch]
enabled = true
typo_nested = "invalid""#;
        tokio::fs::write(&config_path, content.as_bytes())
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_partial_toml_file(&config_path).await;
        assert!(result.is_err(), "Unknown nested keys should be rejected");
        Ok(())
    }

    // Test 10b: Invalid boolean string - Config should reject non-boolean values
    #[tokio::test]
    async fn test_invalid_boolean_string_rejected() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_bool_config.toml");

        tokio::fs::write(&config_path, b"[watch]\nenabled = \"not_a_bool\"")
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path).await;
        assert!(
            result.is_err(),
            "String value for boolean field should be rejected"
        );
        Ok(())
    }

    // Test 10b-2: bd-31c - Reject arbitrary string values like "invalid-value-not-bool"
    #[tokio::test]
    async fn test_invalid_boolean_string_exact_match_rejected() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_bool_exact.toml");

        // Write a config with the exact string from the bead requirement
        tokio::fs::write(
            &config_path,
            b"[watch]\nenabled = \"invalid-value-not-bool\"",
        )
        .await
        .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path).await;
        assert!(
            result.is_err(),
            "String value 'invalid-value-not-bool' for boolean field should be rejected"
        );

        // Verify it's a parse error with type validation message
        if let Err(Error::ParseError(msg)) = result {
            assert!(
                msg.contains("expected")
                    || msg.contains("boolean")
                    || msg.contains("true or false"),
                "Error message should indicate type mismatch: {msg}"
            );
        }
        Ok(())
    }

    // Test 10b-3: bd-31c - Verify valid booleans are accepted
    #[tokio::test]
    async fn test_valid_boolean_values_accepted() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("valid_bool.toml");

        for bool_val in [true, false] {
            let bool_str = if bool_val { "true" } else { "false" };
            let content = format!(
                r#"
workspace_dir = "../test"
main_branch = "main"
state_db = ".isolate/state.db"

[watch]
enabled = {}
debounce_ms = 100
paths = [".beads/beads.db"]

[hooks]
post_create = []
pre_remove = []
post_merge = []

[agent]
command = "claude"
env = {{}}

[session]
auto_commit = {}
commit_prefix = "wip:"
max_sessions = 100

[recovery]
policy = "warn"
log_recovered = true
auto_recover_corrupted_wal = true
delete_corrupted_database = false

[conflict_resolution]
mode = "hybrid"
autonomy = 80
security_keywords = ["api-key", "secret", "password"]
log_resolutions = true
"#,
                bool_str, bool_str
            );
            tokio::fs::write(&config_path, &content)
                .await
                .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

            let result = load_toml_file(&config_path).await;
            let err = result.as_ref().err();
            assert!(
                result.is_ok(),
                "Valid boolean value {bool_val} should be accepted, but got error: {:?}",
                err
            );
        }
        Ok(())
    }

    // Test 10c: Valid boolean values in Config should work
    #[tokio::test]
    async fn test_valid_config_with_booleans() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("valid_bool_config.toml");

        let toml_content = r#"
workspace_dir = "../test"
main_branch = "main"
state_db = ".isolate/state.db"

[watch]
enabled = true
debounce_ms = 100
paths = [".beads/beads.db"]

[hooks]
post_create = []
pre_remove = []
post_merge = []

[agent]
command = "claude"
env = {}

[session]
auto_commit = false
commit_prefix = "wip:"
max_sessions = 100

[recovery]
policy = "warn"
log_recovered = true
auto_recover_corrupted_wal = true
delete_corrupted_database = false

[conflict_resolution]
mode = "hybrid"
autonomy = 80
security_keywords = ["api-key", "secret", "password"]
log_resolutions = true
"#;
        tokio::fs::write(&config_path, toml_content)
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path).await;
        assert!(
            result.is_ok(),
            "Valid boolean values should be accepted: {:?}",
            result
        );
        Ok(())
    }

    // Test 11: Partial config - Unspecified values use defaults
    #[test]
    fn test_partial_config_uses_defaults() {
        let mut base = Config::default();
        let partial = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };
        // All other fields remain default

        base.merge(partial);

        assert_eq!(base.workspace_dir, "../custom");
        assert!(base.watch.enabled); // Still default
    }

    // Test 12: Deep merge - hooks.post_create in global + project → project replaces
    #[test]
    fn test_deep_merge_replaces_not_appends() {
        let mut base = Config::default();
        base.hooks.post_create = vec!["a".to_string(), "b".to_string()];

        let mut override_config = Config::default();
        override_config.hooks.post_create = vec!["c".to_string()];

        base.merge(override_config);

        assert_eq!(base.hooks.post_create, vec!["c".to_string()]);
        assert_ne!(
            base.hooks.post_create,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_project_config_path() {
        let result = project_config_path();
        assert!(result.is_ok());

        let path = result.map_or_else(
            |_e| {
                // Return empty path on error - the assert!(result.is_ok()) above
                // will fail with better context
                String::new()
            },
            |p| p.to_string_lossy().to_string(),
        );

        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.workspace_dir, "../{repo}__workspaces");
        assert_eq!(config.main_branch, "");
        assert_eq!(config.state_db, ".isolate/state.db");
        assert!(config.watch.enabled.as_bool());
        assert_eq!(config.watch.debounce_ms, 100);
    }

    #[test]
    fn test_env_var_parsing_bool() {
        std::env::set_var("Isolate_WATCH_ENABLED", "false");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert!(!config.watch.enabled);

        std::env::remove_var("Isolate_WATCH_ENABLED");
    }

    #[test]
    fn test_env_var_parsing_int() {
        std::env::set_var("Isolate_WATCH_DEBOUNCE_MS", "200");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert_eq!(config.watch.debounce_ms, 200);

        std::env::remove_var("Isolate_WATCH_DEBOUNCE_MS");
    }

    #[test]
    fn test_validation_debounce_ms_valid() {
        let mut config = Config::default();
        config.watch.debounce_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_min() {
        let mut config = Config::default();
        config.watch.debounce_ms = 10;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_max() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5000;
        assert!(config.validate().is_ok());
    }

    // Test: Valid top-level keys pass validation
    #[test]
    fn test_validate_key_valid_top_level() {
        let valid_keys = [
            "workspace_dir",
            "main_branch",
            "state_db",
            "watch",
            "hooks",
            "agent",
            "session",
            "recovery",
        ];

        for key in valid_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    // Test: Valid nested keys pass validation
    #[test]
    fn test_validate_key_valid_nested() {
        let valid_keys = [
            "watch.enabled",
            "watch.debounce_ms",
            "watch.paths",
            "hooks.post_create",
            "hooks.pre_remove",
            "hooks.post_merge",
            "agent.command",
            "agent.env",
            "session.auto_commit",
            "session.commit_prefix",
            "recovery.policy",
            "recovery.log_recovered",
        ];

        for key in valid_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    // Test: Invalid keys return error
    #[test]
    fn test_validate_key_invalid_returns_error() {
        let invalid_keys = [
            "nonexistent",
            "typo_key",
            "isolate_agant_id",
            "invalid.nested",
            "watch.invalid_field",
        ];

        for key in invalid_keys {
            let result = validate_key(key);
            assert!(
                result.is_err(),
                "Key '{key}' should be invalid but passed validation"
            );

            if let Err(e) = result {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Unknown configuration key"),
                    "Error should mention unknown key for '{key}': {error_msg}"
                );
            }
        }
    }

    // Test: Empty key returns error
    #[test]
    fn test_validate_key_empty_returns_error() {
        let result = validate_key("");
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Unknown configuration key"),
                "Error should mention unknown key for empty string"
            );
        }
    }

    // Test 13: ConfigManager creation and retrieval
    #[tokio::test]
    async fn test_config_manager_basic() {
        let result = ConfigManager::new().await;

        // Use assert on the Result directly - no unwrap needed
        assert!(result.is_ok(), "ConfigManager::new should succeed");

        let manager = match result {
            Ok(manager) => manager,
            Err(e) => panic!("ConfigManager::new should succeed: {e}"),
        };

        let test_config = manager.get().await;

        // Verify we got a valid config
        assert!(!test_config.workspace_dir.is_empty());
        assert!(test_config.watch.enabled);
    }

    // Test 14: ConfigManager is thread-safe (can clone)
    #[tokio::test]
    async fn test_config_manager_clone() {
        let result = ConfigManager::new().await;

        // Use assert on the Result directly - no unwrap needed
        assert!(result.is_ok(), "ConfigManager::new should succeed");

        let manager1 = result.expect("ConfigManager::new failed");
        let manager2 = manager1.clone();

        // Both managers should provide the same config
        let config1 = manager1.get().await;
        let config2 = manager2.get().await;

        assert_eq!(config1.workspace_dir, config2.workspace_dir);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PARTIAL CONFIG MERGE TESTS (bd-3bg: explicit-key merge semantics)
    // ═══════════════════════════════════════════════════════════════════════════

    // Test: Partial TOML with only one top-level field should not reset others
    #[tokio::test]
    async fn test_partial_config_single_field_preserves_defaults() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("partial_config.toml");

        // Only set main_branch, nothing else
        tokio::fs::write(&config_path, b"main_branch = \"develop\"")
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let partial = load_partial_toml_file(&config_path).await?;

        // Verify that only main_branch is set
        assert!(
            partial.main_branch.is_some(),
            "main_branch should be set from TOML"
        );
        assert_eq!(partial.main_branch, Some("develop".to_string()));

        // workspace_dir should NOT be set (it was not in the TOML)
        assert!(
            partial.workspace_dir.is_none(),
            "workspace_dir should be None since it wasn't in the TOML"
        );

        Ok(())
    }

    // Test: Merging partial config should only override explicitly set fields
    #[test]
    fn test_merge_partial_only_overrides_set_fields() {
        let mut base = Config::default();
        let original_workspace_dir = base.workspace_dir.clone();

        // Create a partial that only sets main_branch
        let partial = PartialConfig {
            main_branch: Some("develop".to_string()),
            ..Default::default()
        };

        base.merge_partial(partial);

        // main_branch should be updated
        assert_eq!(base.main_branch, "develop");

        // Other fields should remain unchanged
        assert_eq!(
            base.workspace_dir, original_workspace_dir,
            "workspace_dir should not be changed"
        );
    }

    // Test: Partial nested config should only override set fields
    #[test]
    fn test_merge_partial_nested_only_overrides_set_fields() {
        let mut base = Config::default();
        let original_enabled = base.watch.enabled;
        let original_debounce_ms = base.watch.debounce_ms;

        let partial = PartialConfig {
            watch: Some(PartialWatchConfig {
                paths: Some(vec!["custom.db".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        base.merge_partial(partial);

        assert_eq!(base.watch.paths, vec!["custom.db".to_string()]);

        assert_eq!(
            base.watch.enabled, original_enabled,
            "enabled should not be changed"
        );
        assert_eq!(
            base.watch.debounce_ms, original_debounce_ms,
            "debounce_ms should not be changed"
        );
    }

    // Test: Multi-layer precedence with partial configs
    #[test]
    fn test_multi_layer_precedence_preserves_lower_layers() {
        // Start with defaults
        let mut config = Config::default();

        // Apply "global" partial that only sets workspace_dir
        let global_partial = PartialConfig {
            workspace_dir: Some("../global_workspaces".to_string()),
            ..Default::default()
        };
        config.merge_partial(global_partial);

        assert_eq!(config.workspace_dir, "../global_workspaces");
        assert_eq!(config.main_branch, ""); // Default should be preserved

        // Apply "project" partial that only sets main_branch
        let project_partial = PartialConfig {
            main_branch: Some("develop".to_string()),
            ..Default::default()
        };
        config.merge_partial(project_partial);

        // workspace_dir from "global" should be preserved
        assert_eq!(
            config.workspace_dir, "../global_workspaces",
            "workspace_dir from global should NOT be reset by project partial"
        );
        // main_branch from "project" should be applied
        assert_eq!(config.main_branch, "develop");
    }

    // Test: Partial with nested section that only sets some fields
    #[test]
    fn test_partial_nested_section_preserves_other_fields() {
        let mut base = Config::default();
        base.watch.debounce_ms = 500; // Set a custom value

        // Create partial that only sets watch.enabled
        let partial = PartialConfig {
            watch: Some(PartialWatchConfig {
                enabled: Some(ValidatedBool(false)),
                debounce_ms: None,
                paths: None,
            }),
            ..Default::default()
        };

        base.merge_partial(partial);

        // enabled should be updated
        assert!(!base.watch.enabled);

        // debounce_ms should be preserved
        assert_eq!(
            base.watch.debounce_ms, 500,
            "debounce_ms should be preserved from base"
        );
    }
}
