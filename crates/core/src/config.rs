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

use crate::error::{Error, Result};

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
    #[serde(skip)]
    sources: Vec<ConfigSource>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
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
        self.sources.sort_by(|a, b| b.priority.cmp(&a.priority));
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
            .ok_or_else(|| Error::ConfigNotFound("Could not determine config directory".into()))?
            .config_dir()
            .join("config.toml");

        Ok(Self {
            global_path,
            project_path: None,
            env_prefix: "SCP_".to_string(),
        })
    }

    /// Create with explicit paths
    pub fn with_paths(global: PathBuf, project: Option<PathBuf>) -> Self {
        Self {
            global_path: global,
            project_path: project,
            env_prefix: "SCP_".to_string(),
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
        if self.global_path.exists() {
            let global = self.load_file(&self.global_path)?;
            for (k, v) in global.iter() {
                config.values.insert(k.clone(), v.clone());
            }
            config.add_source(self.global_path.clone(), ConfigScope::Global, 1);
        }

        // 2. Load project config (overrides global)
        if let Some(project_path) = &self.project_path {
            if project_path.exists() {
                let project = self.load_file(project_path)?;
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

    /// Load from a TOML file
    fn load_file(&self, path: &Path) -> Result<HashMap<String, String>> {
        let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
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
            ConfigScope::Project => self
                .project_path
                .as_ref()
                .ok_or_else(|| Error::ConfigNotFound("No project config path set".into()))?,
            ConfigScope::Env => {
                return Err(Error::ConfigInvalid(
                    "Cannot save to environment scope".into(),
                ))
            }
        };

        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
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

        std::fs::write(path, content).map_err(Error::Io)?;
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
            if vcs != "jj" && vcs != "git" {
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

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create config manager")
    }
}

/// Global config instance
pub fn global_config() -> ConfigManager {
    ConfigManager::new().expect("Failed to create config manager")
}

/// Get config directory
pub fn config_dir() -> Result<PathBuf> {
    directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::ConfigNotFound("Could not determine config directory".into()))
        .map(|dirs| dirs.config_dir().to_path_buf())
}

/// Configuration for file watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Whether file watching is enabled
    pub enabled: bool,
    /// Debounce duration in milliseconds (10-5000)
    pub debounce_ms: u32,
    /// Paths to watch (relative to workspace)
    pub paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
