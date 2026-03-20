//! CLI Alias Handler - Backward compatibility aliases with deprecation warnings
//!
//! This module provides deprecated command aliases that map old command names
//! to the new object-based command structure:
//!
//! ## Mappings:
//! - `add` -> `session add`
//! - `list` -> `session list`
//! - `claim` -> `task claim`
//! - `yield` -> `task yield`
//! - `sync` -> `session sync`
//! - `submit` -> `session submit`
//! - `done` -> `session done`

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Write};

/// Represents a deprecated command alias with its replacement
#[derive(Debug, Clone)]
pub struct DeprecatedAlias {
    /// Old command name
    pub old_command: &'static str,
    /// New command path (e.g., "session add")
    pub new_command: &'static str,
    /// Deprecation message
    pub message: &'static str,
}

impl DeprecatedAlias {
    /// Create a new deprecated alias
    pub const fn new(old_command: &'static str, new_command: &'static str) -> Self {
        Self {
            old_command,
            new_command,
            message: "will be removed in a future version",
        }
    }

    /// Display deprecation warning to stderr
    pub fn warn(&self) {
        let warning = format!(
            "warning: '{}' is deprecated, use '{}' instead. {}\n",
            self.old_command, self.new_command, self.message
        );
        // Use eprintln alternative that doesn't panic
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(warning.as_bytes());
        let _ = stderr.flush();
    }

    /// Display deprecation warning with custom message
    pub fn warn_with_message(&self, additional_info: &str) {
        let warning = format!(
            "warning: '{}' is deprecated, use '{}' instead. {} {}\n",
            self.old_command, self.new_command, self.message, additional_info
        );
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(warning.as_bytes());
        let _ = stderr.flush();
    }
}

// ============================================================================
// DEPRECATED ALIASES REGISTRY
// ============================================================================

/// Alias: `add` -> `session add`
pub const ALIAS_ADD: DeprecatedAlias = DeprecatedAlias::new("add", "session add");

/// Alias: `list` -> `session list`
pub const ALIAS_LIST: DeprecatedAlias = DeprecatedAlias::new("list", "session list");

/// Alias: `sync` -> `session sync`
pub const ALIAS_SYNC: DeprecatedAlias = DeprecatedAlias::new("sync", "session sync");

/// Alias: `submit` -> `session submit`
pub const ALIAS_SUBMIT: DeprecatedAlias = DeprecatedAlias::new("submit", "session submit");

/// Alias: `done` -> `session done`
pub const ALIAS_DONE: DeprecatedAlias = DeprecatedAlias::new("done", "session done");

/// All deprecated aliases
pub const ALL_ALIASES: &[DeprecatedAlias] =
    &[ALIAS_ADD, ALIAS_LIST, ALIAS_SYNC, ALIAS_SUBMIT, ALIAS_DONE];

/// Check if a command is a deprecated alias and return its info
pub fn get_alias(command: &str) -> Option<&'static DeprecatedAlias> {
    ALL_ALIASES
        .iter()
        .find(|alias| alias.old_command == command)
}

/// Emit deprecation warning for a command if it's an alias
pub fn warn_if_deprecated(command: &str) {
    if let Some(alias) = get_alias(command) {
        alias.warn();
    }
}

/// Emit deprecation warning for a command with additional context
pub fn warn_if_deprecated_with_info(command: &str, info: &str) {
    if let Some(alias) = get_alias(command) {
        alias.warn_with_message(info);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_aliases_registered() {
        assert_eq!(ALL_ALIASES.len(), 5);
    }

    #[test]
    fn test_alias_add() {
        assert_eq!(ALIAS_ADD.old_command, "add");
        assert_eq!(ALIAS_ADD.new_command, "session add");
    }

    #[test]
    fn test_alias_list() {
        assert_eq!(ALIAS_LIST.old_command, "list");
        assert_eq!(ALIAS_LIST.new_command, "session list");
    }

    #[test]
    fn test_alias_sync() {
        assert_eq!(ALIAS_SYNC.old_command, "sync");
        assert_eq!(ALIAS_SYNC.new_command, "session sync");
    }

    #[test]
    fn test_alias_submit() {
        assert_eq!(ALIAS_SUBMIT.old_command, "submit");
        assert_eq!(ALIAS_SUBMIT.new_command, "session submit");
    }

    #[test]
    fn test_alias_done() {
        assert_eq!(ALIAS_DONE.old_command, "done");
        assert_eq!(ALIAS_DONE.new_command, "session done");
    }

    #[test]
    fn test_get_alias_found() {
        let alias = get_alias("add");
        assert!(alias.is_some());
        assert_eq!(alias.map(|a| a.new_command), Some("session add"));
    }

    #[test]
    fn test_get_alias_not_found() {
        let alias = get_alias("nonexistent");
        assert!(alias.is_none());
    }

    #[test]
    fn test_deprecated_alias_new() {
        let alias = DeprecatedAlias::new("old", "new path");
        assert_eq!(alias.old_command, "old");
        assert_eq!(alias.new_command, "new path");
    }
}
