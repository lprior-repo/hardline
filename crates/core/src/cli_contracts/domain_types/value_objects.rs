//! Value object types for CLI contracts.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt::Display;

use crate::cli_contracts::ContractError;

/// A non-empty string that has been trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validates a non-empty string value.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty or whitespace-only.
    pub fn validate(s: &str) -> Result<(), ContractError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ContractError::invalid_input(
                "string",
                "cannot be empty or whitespace",
            ));
        }
        Ok(())
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = ContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.trim().to_string()))
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated limit value (1..=1000).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Limit(u16);

impl Limit {
    #[must_use]
    pub const fn value(self) -> usize {
        self.0 as usize
    }

    /// Validates a limit value.
    ///
    /// # Errors
    ///
    /// Returns an error if the limit is zero or exceeds 1000.
    pub fn validate(limit: usize) -> Result<(), ContractError> {
        if limit == 0 {
            return Err(ContractError::invalid_input("limit", "must be at least 1"));
        }
        if limit > 1000 {
            return Err(ContractError::invalid_input("limit", "cannot exceed 1000"));
        }
        Ok(())
    }
}

impl TryFrom<usize> for Limit {
    type Error = ContractError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(u16::try_from(value).map_err(|_| {
            ContractError::invalid_input("limit", "value too large for internal representation")
        })?))
    }
}

/// A validated priority value (0..=1000, where 0 is highest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Priority(u16);

impl Priority {
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0 as u32
    }

    /// Validates a priority value.
    ///
    /// # Errors
    ///
    /// Returns an error if the priority exceeds 1000.
    pub fn validate(priority: u32) -> Result<(), ContractError> {
        if priority > 1000 {
            return Err(ContractError::invalid_input(
                "priority",
                "must be between 0 and 1000",
            ));
        }
        Ok(())
    }
}

impl TryFrom<u32> for Priority {
    type Error = ContractError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(u16::try_from(value).map_err(|_| {
            ContractError::invalid_input("priority", "value too large for internal representation")
        })?))
    }
}

/// A validated timeout value in seconds (1..=86400, i.e., 24 hours).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeoutSeconds(u64);

impl TimeoutSeconds {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    const MIN: u64 = 1;
    const MAX: u64 = 24 * 60 * 60; // 24 hours

    /// Validates a timeout value in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeout is less than 1 or exceeds 24 hours.
    pub fn validate(timeout: u64) -> Result<(), ContractError> {
        if timeout < Self::MIN {
            return Err(ContractError::invalid_input(
                "timeout",
                "must be at least 1 second",
            ));
        }
        if timeout > Self::MAX {
            return Err(ContractError::invalid_input(
                "timeout",
                "cannot exceed 24 hours",
            ));
        }
        Ok(())
    }
}

impl TryFrom<u64> for TimeoutSeconds {
    type Error = ContractError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value))
    }
}
