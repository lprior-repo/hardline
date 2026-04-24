//! Config file loading, parsing, saving, and environment variable overrides.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Result;

use super::types::{Config, ConfigScope};
use super::validation::{keys, ENV_PREFIX};

use crate::config::config_watcher::validate_config_file;

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
            crate::error_config::ConfigErrorKind::NotFound(
                "Could not determine config directory".into(),
            )
            .into()
        },
    )?;
    Ok(dirs.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use proptest::prop_assert_eq;

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
        let hooks = crate::config::HooksConfig::with_values(
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

    proptest::proptest! {
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
}
