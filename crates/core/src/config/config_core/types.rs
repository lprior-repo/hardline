//! Core configuration types: Config, ConfigScope, ConfigValue, ConfigSource, WatchConfig.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    pub conflict: crate::config::ConflictResolutionConfig,
    pub session: crate::config::SessionConfig,
    pub hooks: crate::config::HooksConfig,
    pub agent: crate::config::AgentConfig,
    pub vcs: crate::config::VcsConfig,
    pub auth: crate::config::AuthConfig,
    #[serde(skip)]
    pub(crate) sources: Vec<ConfigSource>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            conflict: crate::config::ConflictResolutionConfig::default(),
            session: crate::config::SessionConfig::default(),
            hooks: crate::config::HooksConfig::default(),
            agent: crate::config::AgentConfig::default(),
            vcs: crate::config::VcsConfig::default(),
            auth: crate::config::AuthConfig::default(),
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

/// Configuration for file watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Whether file watching is enabled
    pub enabled: crate::config::types::ValidatedBool,
    /// Debounce duration in milliseconds (10-5000)
    pub debounce_ms: u32,
    /// Paths to watch (relative to workspace)
    pub paths: Vec<String>,
}
