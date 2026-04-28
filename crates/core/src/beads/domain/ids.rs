//! Identifier newtypes for the beads domain.
//!
//! Semantic newtypes prevent primitive obsession and validate at construction.

use serde::{Deserialize, Serialize};

use super::errors::DomainError;

/// A validated issue identifier.
///
/// Must be non-empty and match typical ID patterns (alphanumeric, hyphens, underscores).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IssueId(String);

impl IssueId {
    /// Maximum length for issue IDs.
    pub const MAX_LENGTH: usize = 100;

    /// Create a new `IssueId`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::EmptyId` if the input is empty.
    /// Returns `DomainError::InvalidIdPattern` if the pattern doesn't match.
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();

        if id.is_empty() {
            return Err(DomainError::EmptyId);
        }

        if id.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidIdPattern(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }

        // Validate pattern: alphanumeric, hyphens, underscores only
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidIdPattern(
                "ID must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(Self(id))
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

impl std::fmt::Display for IssueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IssueId {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for IssueId {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// An assignee identifier (username or email).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Assignee(String);

impl Assignee {
    /// Maximum length for assignee.
    pub const MAX_LENGTH: usize = 100;

    /// Create a new `Assignee`, validating the input.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidIdPattern` if the pattern doesn't match.
    pub fn new(assignee: impl Into<String>) -> Result<Self, DomainError> {
        let assignee = assignee.into();

        if assignee.is_empty() {
            return Err(DomainError::InvalidIdPattern(
                "Assignee cannot be empty".to_string(),
            ));
        }

        if assignee.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidIdPattern(format!(
                "Assignee exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }

        Ok(Self(assignee))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Assignee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Assignee {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A parent issue identifier.
///
/// Type alias for semantic clarity - references another issue.
pub type ParentId = IssueId;
