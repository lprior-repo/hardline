//! KIRK Contracts for Config CLI operations.
//!
//! Config manages isolate configuration settings.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for getting a config value.
#[derive(Debug, Clone)]
pub struct GetConfigInput {
    /// Config key (e.g., "`session.max_count`")
    pub key: String,
}

/// Input for setting a config value.
#[derive(Debug, Clone)]
pub struct SetConfigInput {
    /// Config key
    pub key: String,
    /// Config value
    pub value: String,
    /// Scope (local, global, system)
    pub scope: Option<String>,
}

/// Input for listing config.
#[derive(Debug, Clone, Default)]
pub struct ListConfigInput {
    /// Filter by scope
    pub scope: Option<String>,
    /// Show only local config
    pub local: bool,
}

/// Input for editing config.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EditConfigInput {
    /// Scope to edit
    pub scope: Option<String>,
    /// Editor to use
    pub editor: Option<String>,
}

/// Result of config get.
#[derive(Debug, Clone)]
pub struct ConfigValue {
    /// Config key
    pub key: String,
    /// Config value
    pub value: String,
    /// Source (local, global, system, default)
    pub source: String,
}

/// Result of config list.
#[derive(Debug, Clone)]
pub struct ConfigListResult {
    /// Config entries
    pub entries: Vec<ConfigValue>,
    /// Config file paths
    pub config_files: Vec<PathBuf>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Config CLI operations.
pub struct ConfigContracts;

impl ConfigContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: config key is valid.
    pub const PRECOND_KEY_VALID: Precondition =
        Precondition::new("key_valid", "Config key must be a valid dotted path");

    /// Precondition: config key exists (for get).
    pub const PRECOND_KEY_EXISTS: Precondition =
        Precondition::new("key_exists", "Config key must exist");

    /// Precondition: config value is valid for key.
    pub const PRECOND_VALUE_VALID: Precondition =
        Precondition::new("value_valid", "Config value must be valid for the key type");

    /// Precondition: scope is valid.
    pub const PRECOND_SCOPE_VALID: Precondition =
        Precondition::new("scope_valid", "Scope must be one of: local, global, system");

    /// Precondition: config file is writable.
    pub const PRECOND_CONFIG_WRITABLE: Precondition =
        Precondition::new("config_writable", "Config file must be writable");

    /// Precondition: editor is available.
    pub const PRECOND_EDITOR_AVAILABLE: Precondition =
        Precondition::new("editor_available", "Editor must be available in PATH");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: config file is valid TOML/KDL.
    pub const INV_CONFIG_PARSEABLE: Invariant =
        Invariant::documented("config_parseable", "Config file must be valid TOML/KDL");

    /// Invariant: config values are typed correctly.
    pub const INV_TYPES_CORRECT: Invariant =
        Invariant::documented("types_correct", "Config values match expected types");

    /// Invariant: config precedence is respected.
    pub const INV_PRECEDENCE: Invariant = Invariant::documented(
        "precedence",
        "Local config overrides global which overrides system",
    );

    /// Invariant: required config has defaults.
    pub const INV_DEFAULTS_EXIST: Invariant =
        Invariant::documented("defaults_exist", "All required config keys have defaults");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: config was set.
    pub const POST_CONFIG_SET: Postcondition =
        Postcondition::new("config_set", "Config value was written to file");

    /// Postcondition: config was reloaded.
    pub const POST_CONFIG_RELOADED: Postcondition =
        Postcondition::new("config_reloaded", "Config was reloaded from disk");

    /// Postcondition: config file is still valid.
    pub const POST_CONFIG_VALID: Postcondition = Postcondition::new(
        "config_valid",
        "Config file is still valid after modification",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a config key.
    ///
    /// # Errors
    /// Returns `ContractError` if the key is invalid.
    pub fn validate_key(key: &str) -> Result<(), ContractError> {
        if key.is_empty() {
            return Err(ContractError::invalid_input("key", "cannot be empty"));
        }

        // Key should be a dotted path like "session.max_count"
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() < 2 {
            return Err(ContractError::invalid_input(
                "key",
                "must be a dotted path (e.g., 'section.key')",
            ));
        }

        for part in parts {
            if part.is_empty() {
                return Err(ContractError::invalid_input(
                    "key",
                    "cannot have empty segments",
                ));
            }
            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(ContractError::invalid_input(
                    "key",
                    "segments must contain only alphanumeric and underscore",
                ));
            }
        }

        Ok(())
    }

    /// Validate a config scope.
    ///
    /// # Errors
    /// Returns `ContractError` if the scope is invalid.
    pub fn validate_scope(scope: &str) -> Result<(), ContractError> {
        match scope {
            "local" | "global" | "system" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "scope",
                "must be one of: local, global, system",
            )),
        }
    }

    /// Known config keys with their validation rules.
    #[must_use]
    pub const fn known_keys() -> &'static [(&'static str, &'static str)] {
        &[
            ("session.max_count", "u32"),
            ("session.default_branch", "String"),
            ("session.workspace_root", "PathBuf"),
            ("hooks.pre_create", "String"),
            ("hooks.post_create", "String"),
            ("hooks.pre_remove", "String"),
            ("hooks.post_remove", "String"),
            ("queue.enabled", "bool"),
            ("queue.max_size", "u32"),
            ("stack.max_depth", "u32"),
            ("agent.default_timeout", "u64"),
            ("output.format", "String"),
        ]
    }

    /// Check if a key is a known config key.
    #[must_use]
    pub fn is_known_key(key: &str) -> bool {
        Self::known_keys().iter().any(|(k, _)| *k == key)
    }
}

impl Contract<GetConfigInput, ConfigValue> for ConfigContracts {
    fn preconditions(input: &GetConfigInput) -> Result<(), ContractError> {
        Self::validate_key(&input.key)
    }

    fn invariants(_input: &GetConfigInput) -> Vec<Invariant> {
        vec![Self::INV_CONFIG_PARSEABLE, Self::INV_PRECEDENCE]
    }

    fn postconditions(input: &GetConfigInput, result: &ConfigValue) -> Result<(), ContractError> {
        if result.key != input.key {
            return Err(ContractError::PostconditionFailed {
                name: "key_matches",
                description: "Result key must match requested key",
            });
        }
        Ok(())
    }
}

impl Contract<SetConfigInput, ()> for ConfigContracts {
    fn preconditions(input: &SetConfigInput) -> Result<(), ContractError> {
        Self::validate_key(&input.key)?;

        if input.value.is_empty() {
            return Err(ContractError::invalid_input("value", "cannot be empty"));
        }

        if let Some(ref scope) = input.scope {
            Self::validate_scope(scope)?;
        }

        Ok(())
    }

    fn invariants(_input: &SetConfigInput) -> Vec<Invariant> {
        vec![Self::INV_CONFIG_PARSEABLE, Self::INV_TYPES_CORRECT]
    }

    fn postconditions(_input: &SetConfigInput, _result: &()) -> Result<(), ContractError> {
        Ok(())
    }
}

impl Contract<ListConfigInput, ConfigListResult> for ConfigContracts {
    fn preconditions(input: &ListConfigInput) -> Result<(), ContractError> {
        if let Some(ref scope) = input.scope {
            Self::validate_scope(scope)?;
        }
        Ok(())
    }

    fn invariants(_input: &ListConfigInput) -> Vec<Invariant> {
        vec![Self::INV_CONFIG_PARSEABLE, Self::INV_PRECEDENCE]
    }

    fn postconditions(
        _input: &ListConfigInput,
        result: &ConfigListResult,
    ) -> Result<(), ContractError> {
        // Verify all entries have valid keys
        for entry in &result.entries {
            Self::validate_key(&entry.key)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key_valid() {
        assert!(ConfigContracts::validate_key("session.max_count").is_ok());
        assert!(ConfigContracts::validate_key("hooks.pre_create").is_ok());
        assert!(ConfigContracts::validate_key("a.b").is_ok());
    }

    #[test]
    fn test_validate_key_empty() {
        assert!(ConfigContracts::validate_key("").is_err());
    }

    #[test]
    fn test_validate_key_no_dot() {
        assert!(ConfigContracts::validate_key("session").is_err());
    }

    #[test]
    fn test_validate_key_empty_segment() {
        assert!(ConfigContracts::validate_key("session.").is_err());
        assert!(ConfigContracts::validate_key(".session").is_err());
        assert!(ConfigContracts::validate_key("session..max").is_err());
    }

    #[test]
    fn test_validate_key_invalid_chars() {
        assert!(ConfigContracts::validate_key("session.max-count").is_err());
        assert!(ConfigContracts::validate_key("session.max count").is_err());
    }

    #[test]
    fn test_validate_scope_valid() {
        assert!(ConfigContracts::validate_scope("local").is_ok());
        assert!(ConfigContracts::validate_scope("global").is_ok());
        assert!(ConfigContracts::validate_scope("system").is_ok());
    }

    #[test]
    fn test_validate_scope_invalid() {
        assert!(ConfigContracts::validate_scope("user").is_err());
        assert!(ConfigContracts::validate_scope("project").is_err());
    }

    #[test]
    fn test_is_known_key() {
        assert!(ConfigContracts::is_known_key("session.max_count"));
        assert!(ConfigContracts::is_known_key("hooks.pre_create"));
        assert!(!ConfigContracts::is_known_key("unknown.key"));
    }

    #[test]
    fn test_get_config_contract_preconditions() {
        let input = GetConfigInput {
            key: "session.max_count".to_string(),
        };
        assert!(ConfigContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_get_config_contract_postconditions() {
        let input = GetConfigInput {
            key: "session.max_count".to_string(),
        };
        let result = ConfigValue {
            key: "session.max_count".to_string(),
            value: "10".to_string(),
            source: "global".to_string(),
        };
        assert!(ConfigContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_get_config_contract_postconditions_key_mismatch() {
        let input = GetConfigInput {
            key: "session.max_count".to_string(),
        };
        let result = ConfigValue {
            key: "session.other".to_string(),
            value: "10".to_string(),
            source: "global".to_string(),
        };
        assert!(ConfigContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_set_config_contract_preconditions() {
        let input = SetConfigInput {
            key: "session.max_count".to_string(),
            value: "20".to_string(),
            scope: Some("local".to_string()),
        };
        assert!(ConfigContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_set_config_contract_preconditions_empty_value() {
        let input = SetConfigInput {
            key: "session.max_count".to_string(),
            value: String::new(),
            scope: None,
        };
        assert!(ConfigContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_list_config_contract_preconditions() {
        let input = ListConfigInput {
            scope: Some("global".to_string()),
            local: false,
        };
        assert!(ConfigContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_list_config_contract_postconditions() {
        let input = ListConfigInput::default();
        let result = ConfigListResult {
            entries: vec![ConfigValue {
                key: "session.max_count".to_string(),
                value: "10".to_string(),
                source: "global".to_string(),
            }],
            config_files: vec![PathBuf::from("/etc/isolate/config.toml")],
        };
        assert!(ConfigContracts::postconditions(&input, &result).is_ok());
    }
}
