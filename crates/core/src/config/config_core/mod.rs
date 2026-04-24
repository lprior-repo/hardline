//! Configuration management for Source Control Plane.
//!
//! Provides persistent configuration storage with support for:
//! - Global config (user-level)
//! - Project config (repo-level)
//! - Environment variable overrides
//! - Config validation

mod loader;
mod merger;
mod types;
pub(crate) mod validation;

pub use loader::{config_dir, global_config, ConfigManager};
pub use merger::{get_repo_name, substitute_placeholders};
pub use types::{Config, ConfigScope, ConfigSource, ConfigValue, WatchConfig};
pub use validation::{keys, validate_key, ENV_PREFIX, VALID_CONFIG_KEYS};
