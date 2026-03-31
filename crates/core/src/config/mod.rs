//! Configuration management for Source Control Plane.
//!
//! Provides persistent configuration storage with support for:
//! - Global config (user-level)
//! - Project config (repo-level)
//! - Environment variable overrides
//! - Config validation
//!
//! Also includes conflict resolution configuration with mode selection
//! (auto/manual/hybrid), autonomy levels, and security keyword detection.

pub mod config_core;

pub mod types;
pub mod config;
pub mod partial;
pub mod command_types;

mod tests;

#[cfg(test)]
mod config_command_tests;

#[cfg(test)]
mod config_integration_tests;

#[cfg(test)]
mod config_proptests;

// Re-exports
pub use config_core::{
    Config, ConfigManager, ConfigScope, ConfigSource, ConfigValue, WatchConfig, config_dir,
    global_config, keys,
};
pub use types::ConflictMode;
pub use config::{ConflictResolutionConfig, SessionConfig};
pub use partial::{PartialConflictResolutionConfig, PartialSessionConfig};
pub use command_types::{
    ConfigKey, ConfigGetResult, ConfigSetResult, ConfigReadPort,
    parse_cli_value, get_nested_value, set_nested_value,
    config_get, config_set, config_list,
    KNOWN_CONFIG_KEYS, KNOWN_SECTION_PREFIXES,
};
