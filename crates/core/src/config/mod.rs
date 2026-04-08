//! Configuration management for Source Control Plane.

pub mod command_types;
#[allow(clippy::module_inception)]
pub mod config;
pub mod config_core;
pub mod config_watcher;
pub mod partial;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod config_command_tests;

#[cfg(test)]
mod config_integration_tests;

#[cfg(test)]
mod config_proptests;

#[cfg(test)]
mod config_value_tests;

// Re-exports
pub use command_types::{
    clear_port, config_get, config_list, config_set, get_nested_value, parse_cli_value,
    set_nested_value, set_port, ConfigGetResult, ConfigKey, ConfigReadPort, ConfigSetResult,
    FileConfigReadPort, KNOWN_CONFIG_KEYS, KNOWN_SECTION_PREFIXES,
};
pub use config::{AgentConfig, ConflictResolutionConfig, HooksConfig, SessionConfig};
pub use config_core::{
    config_dir, get_repo_name, global_config, keys, substitute_placeholders, validate_key, Config,
    ConfigManager, ConfigScope, ConfigSource, ConfigValue, WatchConfig, ENV_PREFIX,
    VALID_CONFIG_KEYS,
};
pub use config_watcher::{validate_config_file, HotReloadConfigManager, MAX_CONFIG_FILE_SIZE};
pub use partial::{
    PartialAgentConfig, PartialConflictResolutionConfig, PartialHooksConfig, PartialSessionConfig,
};
pub use types::ConflictMode;
