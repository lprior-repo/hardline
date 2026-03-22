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

mod tests;

// Re-exports
pub use config_core::{
    Config, ConfigManager, ConfigScope, ConfigSource, ConfigValue, WatchConfig, config_dir,
    global_config, keys,
};
pub use types::ConflictMode;
pub use config::ConflictResolutionConfig;
pub use partial::PartialConflictResolutionConfig;
