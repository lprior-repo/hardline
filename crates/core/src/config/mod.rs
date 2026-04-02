//! Configuration management for Source Control Plane.

pub mod config_core;
pub mod types;
pub mod config;
pub mod partial;
pub mod command_types;
pub mod config_watcher;

#[cfg(test)]
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
    get_repo_name, global_config, keys, substitute_placeholders, validate_key,
    VALID_CONFIG_KEYS, ENV_PREFIX,
};
pub use config_watcher::{HotReloadConfigManager, MAX_CONFIG_FILE_SIZE, validate_config_file};
pub use types::ConflictMode;
pub use config::{AgentConfig, ConflictResolutionConfig, HooksConfig, SessionConfig};
pub use partial::{
    PartialAgentConfig, PartialConflictResolutionConfig, PartialHooksConfig,
    PartialSessionConfig,
};
pub use command_types::{
    ConfigKey, ConfigGetResult, ConfigSetResult, ConfigReadPort,
    FileConfigReadPort,
    parse_cli_value, get_nested_value, set_nested_value,
    config_get, config_set, config_list,
    set_port, clear_port,
    KNOWN_CONFIG_KEYS, KNOWN_SECTION_PREFIXES,
};
