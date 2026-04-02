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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct ConfigParser {
        #[command(subcommand)]
        command: ConfigCommands,
    }

    fn parse(args: &[&str]) -> ConfigCommands {
        let full: Vec<&str> = std::iter::once("scp")
            .chain(args.iter().copied())
            .collect();
        ConfigParser::parse_from(full).command
    }

    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), ConfigCommands::List));
    }

    #[test]
    fn get_requires_key() {
        let result = ConfigParser::try_parse_from(["scp", "get"]);
        assert!(result.is_err());
    }

    #[test]
    fn get_parses() {
        match parse(&["get", "core.editor"]) {
            ConfigCommands::Get { key } => assert_eq!(key, "core.editor"),
            other => panic!("Expected Get, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn set_requires_key_and_value() {
        let result = ConfigParser::try_parse_from(["scp", "set", "key-only"]);
        assert!(result.is_err());
    }

    #[test]
    fn set_parses() {
        match parse(&["set", "core.editor", "vim"]) {
            ConfigCommands::Set { key, value } => {
                assert_eq!(key, "core.editor");
                assert_eq!(value, "vim");
            }
            other => panic!("Expected Set, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
