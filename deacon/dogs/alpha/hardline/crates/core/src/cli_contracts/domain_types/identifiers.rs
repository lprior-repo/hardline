//! Identifier newtypes for CLI contracts.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt::Display;

use crate::cli_contracts::ContractError;

pub use crate::domain::AgentId;
pub use crate::domain::SessionName;

impl AgentId {
    /// Parses an agent ID from a string, returning a contract error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a valid agent ID.
    pub fn try_parse_contract(s: impl Into<String>) -> Result<Self, ContractError> {
        let s = s.into();
        Self::parse(s).map_err(|e| ContractError::invalid_input("agent_id", e.to_string()))
    }
}

impl SessionName {
    /// Parses a session name from a string, returning a contract error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a valid session name.
    pub fn try_parse_contract(s: impl Into<String>) -> Result<Self, ContractError> {
        let s = s.into();
        Self::parse(s).map_err(|e| ContractError::invalid_input("name", e.to_string()))
    }
}

/// A validated task identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for TaskId {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(ContractError::invalid_input("task_id", "cannot be empty"));
        }
        Ok(Self(value.to_string()))
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
