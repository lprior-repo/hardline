//! Configuration management for Source Control Plane.
//!
//! Provides persistent configuration storage with support for:
//! - Global config (user-level)
//! - Project config (repo-level)
//! - Environment variable overrides
//! - Config validation

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config_watcher::validate_config_file;
use super::types::ValidatedBool;
use crate::error::Result;
use crate::error_config::ConfigErrorKind;

/// Configuration key names
pub mod keys {
    pub const VCS_TYPE: &str = "vcs.type";
    pub const DEFAULT_BRANCH: &str = "vcs.default_branch";
    pub const WORKSPACE_DIR: &str = "workspace.directory";
    pub const QUEUE_NAME: &str = "queue.default";
    pub const LOG_LEVEL: &str = "logging.level";
    pub const EDITOR: &str = "editor";
    pub const REMOTE_PUSH: &str = "remote.push";
    pub const REMOTE_FETCH: &str = "remote.fetch";
    pub const AUTO_REBASE: &str = "workspace.auto_rebase";
    pub const AUTO_PUSH: &str = "workspace.auto_push";
}

/// Validated list of all known configuration keys.
///
/// This is used by [`validate_key`] to reject unknown keys at runtime
/// with a helpful error message listing valid keys grouped by category.
pub const VALID_CONFIG_KEYS: &[&str] = &[
    // Top-level (section-only) keys
    "watch",
    "conflict_resolution",
    "session",
    "hooks",
    "agent",
    "vcs",
    "workspace",
    "queue",
    "logging",
    "remote",
    "editor",
    // Watch section
    "watch.enabled",
    "watch.debounce_ms",
    "watch.paths",
    // Conflict resolution section
    "conflict_resolution.mode",
    "conflict_resolution.autonomy",
    "conflict_resolution.security_keywords",
    "conflict_resolution.log_resolutions",
    // Session section
    "session.auto_commit",
    "session.commit_prefix",
    "session.max_sessions",
    // Hooks section
    "hooks.post_create",
    "hooks.pre_remove",
    "hooks.post_merge",
    // Agent section
    "agent.command",
    // VCS section
    "vcs.type",
    "vcs.default_branch",
    // Workspace section
    "workspace.directory",
    "workspace.auto_rebase",
    "workspace.auto_push",
    // Queue section
    "queue.default",
    // Logging section
    "logging.level",
    // Remote section
    "remote.push",
    "remote.fetch",
];

/// Environment variable prefix for SCP config overrides.
/// For example, `SCP_VCS_TYPE` maps to `vcs.type`.
pub const ENV_PREFIX: &str = "SCP_";

/// Validate a configuration key.
///
/// Checks if the given key is either:
/// - An exact match in [`VALID_CONFIG_KEYS`], or
/// - A parent prefix of a valid key (e.g. `"watch"` is valid because
///   `"watch.enabled"` etc. start with `"watch."`)
///
/// # Errors
///
/// Returns `ConfigErrorKind::ConfigParseError` if the key is not recognized.
/// The error message includes a list of valid keys grouped by category.
pub fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(ConfigErrorKind::ConfigParseError("empty config key".to_string()).into());
    }

    let is_valid = VALID_CONFIG_KEYS
        .iter()
        .any(|valid_key| key == *valid_key || valid_key.starts_with(&format!("{key}.")));

    if is_valid {
        Ok(())
    } else {
        let mut msg = format!("Unknown configuration key: '{key}'\n\n");
        msg.push_str("  watch.enabled, watch.debounce_ms, watch.paths\n");
        msg.push_str("  conflict_resolution.mode, conflict_resolution.autonomy,\n");
        msg.push_str(
            "    conflict_resolution.security_keywords, conflict_resolution.log_resolutions\n",
        );
        msg.push_str("  session.auto_commit, session.commit_prefix, session.max_sessions\n");
        msg.push_str("  hooks.post_create, hooks.pre_remove, hooks.post_merge\n");
        msg.push_str("  agent.command\n");
        msg.push_str("  vcs.type, vcs.default_branch\n");
        msg.push_str("  workspace.directory, workspace.auto_rebase, workspace.auto_push\n");
        msg.push_str("  queue.default\n");
        msg.push_str("  logging.level\n");
        msg.push_str("  remote.push, remote.fetch\n");
        msg.push_str("  editor\n");
        msg.push_str("\nUse 'scp config list' to see current configuration.");
        Err(ConfigErrorKind::ConfigParseError(msg).into())
    }
}

/// Get repository name from current working directory.
///
/// # Errors
///
/// Returns error if the current directory cannot be determined or
/// the directory name cannot be extracted.
pub fn get_repo_name() -> Result<String> {
    let dir = std::env::current_dir().map_err(|e| {
        crate::error::Error::io_error(format!("Failed to get current directory: {e}"))
    })?;
    dir.file_name()
        .ok_or_else(|| {
            crate::error::Error::io_error(
                "Failed to determine repository name from current directory".to_string(),
            )
        })
        .map(|name| name.to_string_lossy().to_string())
}

/// Substitute `{repo}` placeholders in all config values with the current
/// repository name.
///
/// Only string values containing `{repo}` are modified; all other values
/// are left unchanged.
///
/// # Errors
///
/// Returns error if the repository name cannot be determined.
pub fn substitute_placeholders(config: &mut Config) -> Result<()> {
    let repo_name = get_repo_name()?;
    let updated: std::collections::HashMap<String, String> = config
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.replace("{repo}", &repo_name)))
        .collect();
    config.values = updated;
    Ok(())
}

/// Configuration scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConfigScope {
    /// User-level configuration (~/.config/scp/)
    #[default]
    Global,
    /// Project-level configuration (.scp/config in repo)
    Project,
    /// Environment variables override everything
    Env,
}

impl fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigScope::Global => write!(f, "Global"),
            ConfigScope::Project => write!(f, "Project"),
            ConfigScope::Env => write!(f, "Env"),
        }
    }
}

/// A configuration value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue {
    pub key: String,
    pub value: String,
    pub scope: ConfigScope,
    pub source: PathBuf,
}

impl ConfigValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>, scope: ConfigScope) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            scope,
            source: PathBuf::new(),
        }
    }

    pub fn with_source(
        key: impl Into<String>,
        value: impl Into<String>,
        scope: ConfigScope,
        source: impl Into<PathBuf>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            scope,
            source: source.into(),
        }
    }
}

/// Configuration source with priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSource {
    pub path: PathBuf,
    pub scope: ConfigScope,
    pub priority: u8,
}

/// Main configuration container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub values: HashMap<String, String>,
    pub conflict: super::config::ConflictResolutionConfig,
    pub session: super::config::SessionConfig,
    pub hooks: super::config::HooksConfig,
    pub agent: super::config::AgentConfig,
    #[serde(skip)]
    sources: Vec<ConfigSource>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            conflict: super::config::ConflictResolutionConfig::default(),
            session: super::config::SessionConfig::default(),
            hooks: super::config::HooksConfig::default(),
            agent: super::config::AgentConfig::default(),
            sources: Vec::new(),
        }
    }

    /// Get a config value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    /// Set a config value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Remove a config value
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.values.remove(key)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Get all key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.values.iter()
    }

    /// Add a source
    pub fn add_source(&mut self, path: PathBuf, scope: ConfigScope, priority: u8) {
        self.sources.push(ConfigSource {
            path,
            scope,
            priority,
        });
        self.sources.sort_by_key(|b| std::cmp::Reverse(b.priority));
    }

    /// Get all sources
    pub fn sources(&self) -> &[ConfigSource] {
        &self.sources
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Configuration:")?;
        for (key, value) in self.iter() {
            writeln!(f, "  {} = {}", key, value)?;
        }
        Ok(())
    }
}

/// Check whether a path exists on disk, including as a dead symlink.
///
/// Unlike `Path::exists()` which follows symlinks (returning `false` for dead
/// symlinks), this uses `symlink_metadata` which returns `Ok` for any filesystem
/// entry, even if the symlink target is missing.
fn path_exists_on_disk(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Config file manager
pub struct ConfigManager {
    global_path: PathBuf,
    project_path: Option<PathBuf>,
    env_prefix: String,
}

impl ConfigManager {
    /// Create a new config manager
    pub fn new() -> Result<Self> {
        let global_path = directories::ProjectDirs::from("com", "scp", "scp")
            .ok_or_else(|| {
                crate::error::Error::config_not_found("Could not determine config directory")
            })?
            .config_dir()
            .join("config.toml");

        Ok(Self {
            global_path,
            project_path: None,
            env_prefix: ENV_PREFIX.to_string(),
        })
    }

    /// Create with explicit paths
    pub fn with_paths(global: PathBuf, project: Option<PathBuf>) -> Self {
        Self {
            global_path: global,
            project_path: project,
            env_prefix: ENV_PREFIX.to_string(),
        }
    }

    /// Get global config path
    pub fn global_path(&self) -> &Path {
        &self.global_path
    }

    /// Get project config path
    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    /// Load configuration from all sources with proper precedence:
    /// 1. Environment variables (highest)
    /// 2. Project config (.scp/config)
    /// 3. Global config (~/.config/scp/config.toml)
    pub fn load(&self) -> Result<Config> {
        let mut config = Config::new();

        // 1. Load global config
        if path_exists_on_disk(&self.global_path) {
            validate_config_file(&self.global_path)?;
            let global = self.load_file_contents(&self.global_path)?;
            for (k, v) in global.iter() {
                config.values.insert(k.clone(), v.clone());
            }
            config.add_source(self.global_path.clone(), ConfigScope::Global, 1);
        }

        // 2. Load project config (overrides global)
        if let Some(project_path) = &self.project_path {
            if path_exists_on_disk(project_path) {
                validate_config_file(project_path)?;
                let project = self.load_file_contents(project_path)?;
                for (k, v) in project.iter() {
                    config.values.insert(k.clone(), v.clone());
                }
                config.add_source(project_path.clone(), ConfigScope::Project, 2);
            }
        }

        // 3. Apply environment variable overrides
        self.load_env(&mut config);
        config.add_source(PathBuf::from("environment"), ConfigScope::Env, 3);

        Ok(config)
    }

    /// Load from a TOML file (assumes validation has already been performed).
    fn load_file_contents(&self, path: &Path) -> Result<HashMap<String, String>> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        self.parse_toml(&contents)
    }

    /// Parse TOML content
    pub(crate) fn parse_toml(&self, content: &str) -> Result<HashMap<String, String>> {
        let mut values = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                // Remove quotes if present
                let value = if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    &value[1..value.len() - 1]
                } else {
                    value
                };

                values.insert(key.to_string(), value.to_string());
            }
        }

        Ok(values)
    }

    /// Load environment variable overrides
    fn load_env(&self, config: &mut Config) {
        for (key, value) in std::env::vars() {
            if key.starts_with(&self.env_prefix) {
                let config_key = key[self.env_prefix.len()..]
                    .to_lowercase()
                    .replace('_', ".");
                config.values.insert(config_key, value);
            }
        }
    }

    /// Save configuration to a file
    pub fn save(&self, config: &Config, scope: ConfigScope) -> Result<()> {
        let path = match scope {
            ConfigScope::Global => &self.global_path,
            ConfigScope::Project => self.project_path.as_ref().ok_or_else(|| {
                crate::error::Error::config_not_found("No project config path set")
            })?,
            ConfigScope::Env => {
                return Err(crate::error::Error::config_invalid(
                    "Cannot save to environment scope",
                ))
            }
        };

        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        }

        // Write config
        let mut content = String::new();
        content.push_str("# SCP Configuration\n");
        content.push_str(&format!("# Generated: {}\n\n", chrono::Utc::now()));

        let mut keys: Vec<_> = config.values.keys().collect();
        keys.sort();

        for key in keys {
            if let Some(value) = config.values.get(key) {
                content.push_str(&format!("{} = \"{}\"\n", key, value));
            }
        }

        std::fs::write(path, content).map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        Ok(())
    }

    /// Get a single value with environment override support
    pub fn get_value(&self, key: &str) -> Option<String> {
        // Check env first
        let env_key = format!(
            "{}{}",
            self.env_prefix,
            key.to_uppercase().replace('.', "_")
        );
        if let Ok(value) = std::env::var(&env_key) {
            return Some(value);
        }

        // Load and check config
        if let Ok(config) = self.load() {
            config.get(key).cloned()
        } else {
            None
        }
    }

    /// Validate configuration
    pub fn validate(config: &Config) -> Vec<String> {
        let mut errors = Vec::new();

        // Validate VCS type
        if let Some(vcs) = config.get(keys::VCS_TYPE) {
            if vcs != "git" {
                errors.push(format!("Invalid VCS type: {}", vcs));
            }
        }

        // Validate logging level
        if let Some(level) = config.get(keys::LOG_LEVEL) {
            let valid = ["trace", "debug", "info", "warn", "error"];
            if !valid.contains(&level.as_str()) {
                errors.push(format!("Invalid log level: {}", level));
            }
        }

        errors
    }
}

#[allow(clippy::expect_used)]
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create config manager")
    }
}

/// Global config instance
#[allow(clippy::expect_used)]
pub fn global_config() -> ConfigManager {
    ConfigManager::new().expect("Failed to create config manager")
}

/// Get config directory
pub fn config_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "scp", "scp").ok_or_else(
        || -> crate::error::Error {
            ConfigErrorKind::NotFound("Could not determine config directory".into()).into()
        },
    )?;
    Ok(dirs.config_dir().to_path_buf())
}

/// Configuration for file watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Whether file watching is enabled
    pub enabled: ValidatedBool,
    /// Debounce duration in milliseconds (10-5000)
    pub debounce_ms: u32,
    /// Paths to watch (relative to workspace)
    pub paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assert, prop_assert_eq};

    #[test]
    fn test_config_basic() {
        let mut config = Config::new();
        assert!(config.get("test").is_none());

        config.set("test", "value");
        assert_eq!(config.get("test"), Some(&"value".to_string()));

        config.remove("test");
        assert!(config.get("test").is_none());
    }

    #[test]
    fn test_parse_toml() {
        let manager = ConfigManager::new().unwrap();
        let toml = r#"
            # Comment
            key1 = "value1"
            key2 = "value 2"
            key3 = 'single quotes'
        "#;

        let parsed = manager.parse_toml(toml).unwrap();
        assert_eq!(parsed.get("key1"), Some(&"value1".to_string()));
        assert_eq!(parsed.get("key2"), Some(&"value 2".to_string()));
        assert_eq!(parsed.get("key3"), Some(&"single quotes".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::new();
        config.set(keys::VCS_TYPE, "invalid");
        config.set(keys::LOG_LEVEL, "trace");

        let errors = ConfigManager::validate(&config);
        assert!(!errors.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // validate_key tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn validate_key_accepts_exact_leaf_keys() {
        let leaf_keys = [
            "watch.enabled",
            "watch.debounce_ms",
            "watch.paths",
            "conflict_resolution.mode",
            "conflict_resolution.autonomy",
            "conflict_resolution.security_keywords",
            "conflict_resolution.log_resolutions",
            "session.auto_commit",
            "session.commit_prefix",
            "session.max_sessions",
            "hooks.post_create",
            "hooks.pre_remove",
            "hooks.post_merge",
            "agent.command",
            "vcs.type",
            "vcs.default_branch",
            "workspace.directory",
            "workspace.auto_rebase",
            "workspace.auto_push",
            "queue.default",
            "logging.level",
            "remote.push",
            "remote.fetch",
            "editor",
        ];
        for key in &leaf_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    #[test]
    fn validate_key_accepts_section_prefixes() {
        let section_keys = [
            "watch",
            "conflict_resolution",
            "session",
            "hooks",
            "agent",
            "vcs",
            "workspace",
            "queue",
            "logging",
            "remote",
        ];
        for key in &section_keys {
            assert!(validate_key(key).is_ok(), "Section '{key}' should be valid");
        }
    }

    #[test]
    fn validate_key_rejects_unknown_keys() {
        let invalid_keys = [
            "foo.bar",
            "unknown_key",
            "watch.nonexistent",
            "session.invalid_field",
            "nope",
            "vcs.svn",
        ];
        for key in &invalid_keys {
            let result = validate_key(key);
            assert!(result.is_err(), "Key '{key}' should be rejected");
            let err_msg = format!("{result:?}");
            assert!(
                err_msg.contains("Unknown configuration key"),
                "Error for '{key}' should mention unknown key, got: {err_msg}"
            );
        }
    }

    #[test]
    fn validate_key_rejects_empty_string() {
        let result = validate_key("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_key_error_lists_valid_keys() {
        let result = validate_key("bad_key");
        assert!(result.is_err());
        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.contains("watch.enabled"),
            "Should list watch.enabled"
        );
        assert!(
            err_str.contains("conflict_resolution.mode"),
            "Should list conflict_resolution.mode"
        );
        assert!(
            err_str.contains("session.auto_commit"),
            "Should list session.auto_commit"
        );
        assert!(
            err_str.contains("hooks.post_create"),
            "Should list hooks.post_create"
        );
        assert!(err_str.contains("vcs.type"), "Should list vcs.type");
        assert!(
            err_str.contains("workspace.directory"),
            "Should list workspace.directory"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // substitute_placeholders tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn substitute_placeholders_replaces_repo_placeholder() {
        let mut config = Config::new();
        config.set("workspace.directory", "../{repo}__workspaces");
        config.set("logging.level", "info");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        let workspace_dir = config.get("workspace.directory").expect("key should exist");
        assert!(
            !workspace_dir.contains("{repo}"),
            "Placeholder should be replaced, got: {workspace_dir}"
        );
        assert!(
            workspace_dir.contains("__workspaces"),
            "Non-placeholder content should be preserved, got: {workspace_dir}"
        );
        // Unrelated value should be unchanged
        assert_eq!(config.get("logging.level"), Some(&"info".to_string()));
    }

    #[test]
    fn substitute_placeholders_handles_multiple_placeholders() {
        let mut config = Config::new();
        config.set("remote.push", "https://github.com/{repo}/{repo}.git");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        let push = config.get("remote.push").expect("key should exist");
        assert!(
            !push.contains("{repo}"),
            "All placeholders should be replaced, got: {push}"
        );
    }

    #[test]
    fn substitute_placeholders_no_error_when_no_values() {
        let mut config = Config::new();
        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn substitute_placeholders_preserves_non_placeholder_values() {
        let mut config = Config::new();
        config.set("vcs.type", "git");
        config.set("logging.level", "debug");

        let result = substitute_placeholders(&mut config);
        assert!(result.is_ok());

        assert_eq!(config.get("vcs.type"), Some(&"git".to_string()));
        assert_eq!(config.get("logging.level"), Some(&"debug".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // get_repo_name tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn get_repo_name_returns_current_dir_name() {
        let result = get_repo_name();
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(!name.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ENV_PREFIX constant tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn env_prefix_is_scp() {
        assert_eq!(ENV_PREFIX, "SCP_");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VALID_CONFIG_KEYS completeness tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn valid_config_keys_is_nonempty() {
        assert!(!VALID_CONFIG_KEYS.is_empty());
    }

    #[test]
    fn valid_config_keys_covers_all_known_keys() {
        // Every key in command_types::KNOWN_CONFIG_KEYS should be in VALID_CONFIG_KEYS
        let known_from_command = crate::config::command_types::KNOWN_CONFIG_KEYS;
        for key in known_from_command {
            assert!(
                VALID_CONFIG_KEYS.contains(key),
                "KNOWN_CONFIG_KEYS entry '{key}' should be in VALID_CONFIG_KEYS"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // HooksConfig integration in Config
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_new_has_default_hooks() {
        let config = Config::new();
        assert!(config.hooks.post_create.is_empty());
        assert!(config.hooks.pre_remove.is_empty());
        assert!(config.hooks.post_merge.is_empty());
        assert!(!config.hooks.has_hooks());
    }

    #[test]
    fn config_hooks_serialization_roundtrip() {
        let hooks = super::super::config::HooksConfig::with_values(
            vec!["echo 'created'".to_string()],
            vec!["echo 'removing'".to_string()],
            vec!["echo 'merged'".to_string()],
        );
        let config = Config {
            hooks,
            ..Config::new()
        };
        let json = serde_json::to_string(&config).expect("should serialize");
        let deserialized: Config = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(config.hooks, deserialized.hooks);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigManager::load validation integration tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn load_returns_error_for_dead_symlink() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().expect("tempdir should succeed");
            let dead_target = dir.path().join("does_not_exist.toml");
            let link = dir.path().join("config.toml");

            std::os::unix::fs::symlink(&dead_target, &link)
                .expect("symlink creation should succeed");

            let manager = ConfigManager::with_paths(link.clone(), None);
            let result = manager.load();

            assert!(
                result.is_err(),
                "ConfigManager::load should error on dead symlink, not silently use defaults"
            );
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

    #[test]
    fn load_returns_defaults_when_no_config_file_exists() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let missing_path = dir.path().join("nonexistent.toml");

        let manager = ConfigManager::with_paths(missing_path, None);
        let result = manager.load();

        assert!(
            result.is_ok(),
            "ConfigManager::load should succeed when config file does not exist"
        );
        let config = result.expect("load should succeed");
        assert!(
            config.values.is_empty(),
            "Config should be empty defaults when no file exists"
        );
    }

    #[test]
    fn load_returns_error_for_live_symlink() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().expect("tempdir should succeed");
            let target = dir.path().join("real.toml");
            let link = dir.path().join("config.toml");

            std::fs::write(&target, "logging.level = \"debug\"").expect("write should succeed");
            std::os::unix::fs::symlink(&target, &link).expect("symlink creation should succeed");

            let manager = ConfigManager::with_paths(link.clone(), None);
            let result = manager.load();

            assert!(
                result.is_err(),
                "ConfigManager::load should reject symlink config file"
            );
            let err_msg = format!("{result:?}");
            assert!(
                err_msg.contains("symbolic link"),
                "Error should mention symbolic link, got: {err_msg}"
            );
        }
        #[cfg(not(unix))]
        {
            // On non-Unix platforms, symlink_file requires admin privileges.
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Proptests
    // ═══════════════════════════════════════════════════════════════════════

    proptest::proptest! {
        /// Any key drawn from VALID_CONFIG_KEYS should pass validation.
        #[test]
        fn proptest_validate_key_accepts_all_valid_keys(idx in 0..VALID_CONFIG_KEYS.len()) {
            let key = VALID_CONFIG_KEYS[idx];
            prop_assert!(validate_key(key).is_ok(), "VALID_CONFIG_KEYS entry '{key}' was rejected");
        }
    }

    proptest::proptest! {
        /// Keys containing empty segments (consecutive dots, leading/trailing dots)
        /// must be rejected by validate_key.
        #[test]
        fn proptest_validate_key_rejects_empty_segments(
            prefix in "[a-z]{1,10}",
            suffix in "[a-z]{1,10}"
        ) {
            // Consecutive dots create an empty segment: "prefix..suffix"
            let double_dot = format!("{prefix}..{suffix}");
            prop_assert!(validate_key(&double_dot).is_err(),
                "Key with empty segment (double dot) should be rejected: {double_dot}");

            // Leading dot: ".suffix"
            let leading = format!(".{suffix}");
            prop_assert!(validate_key(&leading).is_err(),
                "Key with leading dot should be rejected: {leading}");

            // Trailing dot: "prefix."
            let trailing = format!("{prefix}.");
            prop_assert!(validate_key(&trailing).is_err(),
                "Key with trailing dot should be rejected: {trailing}");
        }
    }

    proptest::proptest! {
        /// Setting a value and then getting it must always return the original value.
        #[test]
        fn proptest_config_set_get_roundtrip(
            key in "[a-zA-Z_][a-zA-Z0-9_]{0,63}",
            value in ".{0,512}"
        ) {
            let mut config = Config::new();
            config.set(&key, &value);
            prop_assert_eq!(config.get(&key), Some(&value));
        }
    }

    proptest::proptest! {
        /// substitute_placeholders must replace {repo} and leave unknown
        /// placeholders like {unknown} untouched.
        #[test]
        fn proptest_substitute_placeholders(
            before in ".{0,64}",
            after in ".{0,64}",
            unknown_tag in "[a-z_]{1,20}"
        ) {
            let mut config = Config::new();
            // Build a value with known {repo}, unknown {unknown_tag}, and literal text.
            let raw = format!("{before}{{repo}}{after}{{{unknown_tag}}}");
            config.set("workspace.directory", &raw);
            config.set("logging.level", "info"); // unrelated value must not change

            let result = substitute_placeholders(&mut config);
            prop_assert!(result.is_ok());

            let _repo_name = get_repo_name().expect("get_repo_name should succeed");
            let replaced = config.get("workspace.directory").expect("key should exist");

            // {repo} must have been replaced
            prop_assert!(!replaced.contains("{repo}"),
                "Expected {{repo}} to be substituted, got: {replaced}");

            // The unknown placeholder must survive unchanged
            let expected_unknown = format!("{{{unknown_tag}}}");
            prop_assert!(replaced.contains(&expected_unknown),
                "Expected unknown placeholder '{expected_unknown}' to be preserved, got: {replaced}");

            // Surrounding literals must survive
            prop_assert!(replaced.starts_with(&before),
                "Value should start with '{before}', got: {replaced}");
            prop_assert!(replaced.ends_with(&expected_unknown),
                "Value should end with '{expected_unknown}', got: {replaced}");

            // Unrelated value must be untouched
            prop_assert_eq!(config.get("logging.level"), Some(&"info".to_string()));
        }
    }
}
