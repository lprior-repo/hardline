//! Configuration types for CLI contracts.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt::Display;

use crate::cli_contracts::ContractError;

/// A validated configuration key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey(String);

impl ConfigKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(key: &str) -> Result<(), ContractError> {
        if key.is_empty() {
            return Err(ContractError::invalid_input("key", "cannot be empty"));
        }
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
}

impl TryFrom<&str> for ConfigKey {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }
}

impl Display for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValue(String);

impl ConfigValue {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(value: &str) -> Result<(), ContractError> {
        if value.is_empty() {
            return Err(ContractError::invalid_input("value", "cannot be empty"));
        }
        Ok(())
    }
}

impl TryFrom<&str> for ConfigValue {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
