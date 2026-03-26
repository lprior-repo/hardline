//! Config command definitions
//!
//! Subcommand enum for configuration management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get config value
    Get {
        /// Key
        key: String,
    },

    /// Set config value
    Set {
        /// Key
        key: String,

        /// Value
        value: String,
    },

    /// List all config
    List,
}
