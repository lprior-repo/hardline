//! Text field newtypes for the beads domain.
//!
//! Validated text fields prevent empty strings and enforce length limits.

use super::errors::DomainError;
use serde::{Deserialize, Serialize};

/// A validated issue title.
///
/// Must be non-empty and within length limits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    /// Maximum length for titles.
    pub const MAX_LENGTH: usize = 200;

    /// Create a new `Title`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::EmptyTitle` if the input is empty.
    /// Returns `DomainError::TitleTooLong` if the input exceeds max length.
    pub fn new(title: impl Into<String>) -> Result<Self, DomainError> {
        let title = title.into();
        let trimmed = title.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyTitle);
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(DomainError::TitleTooLong {
                max: Self::MAX_LENGTH,
                got: trimmed.len(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Title {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Title {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A validated issue description.
///
/// Optional field with length limits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Description(String);

impl Description {
    /// Maximum length for descriptions.
    pub const MAX_LENGTH: usize = 10_000;

    /// Create a new `Description`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::DescriptionTooLong` if the input exceeds max length.
    pub fn new(description: impl Into<String>) -> Result<Self, DomainError> {
        let description = description.into();

        if description.len() > Self::MAX_LENGTH {
            return Err(DomainError::DescriptionTooLong {
                max: Self::MAX_LENGTH,
            });
        }

        Ok(Self(description))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Description {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Description {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
