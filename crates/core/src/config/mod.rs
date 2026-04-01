//! Configuration management for Source Control Plane.

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
    FileConfigReadPort,
    parse_cli_value, get_nested_value, set_nested_value,
    config_get, config_set, config_list,
    set_port, clear_port,
    KNOWN_CONFIG_KEYS, KNOWN_SECTION_PREFIXES,
};
