//! Validated queue entry ID
//!
//! A semantic newtype for queue entry IDs that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::validation::{ValidationError, ValidationResult};

/// A validated queue entry ID
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use scp_core::domain::identifiers::QueueEntryId;
///
/// // Parse and validate
/// let id = QueueEntryId::parse("entry-123")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Contains only alphanumeric, hyphen, underscore
/// - 1-63 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct QueueEntryId(String);

impl QueueEntryId {
    /// Maximum allowed length for a queue entry ID
    pub const MAX_LENGTH: usize = 63;

    /// Parse and validate a queue entry ID (trims whitespace first)
    ///
    /// This follows the "parse at boundaries" DDD principle:
    /// - Trims whitespace from input
    /// - Validates once at construction
    /// - Cannot represent invalid states
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> ValidationResult<Self> {
        let s = s.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::EmptyValue("queue entry id".to_string()));
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(ValidationError::ExceedsMaximum {
                field: "queue_entry_id".to_string(),
                value: u32::try_from(trimmed.len()).unwrap_or(u32::MAX),
                max: u32::try_from(Self::MAX_LENGTH).unwrap_or(u32::MAX),
            });
        }

        // Check for valid characters (alphanumeric, hyphen, underscore)
        for c in trimmed.chars() {
            if !c.is_alphanumeric() && c != '-' && c != '_' {
                return Err(ValidationError::InvalidCharacters {
                    field: "queue_entry_id".to_string(),
                    found: c.to_string(),
                });
            }
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Create a new queue entry ID (alias for parse)
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the ID is invalid.
    pub fn new(s: impl Into<String>) -> ValidationResult<Self> {
        Self::parse(s)
    }

    /// Get the queue entry ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for QueueEntryId {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for QueueEntryId {
    type Error = ValidationError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for QueueEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for QueueEntryId {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for QueueEntryId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<QueueEntryId> for String {
    #[allow(clippy::use_self)] // Self refers to String, not QueueEntryId
    fn from(id: QueueEntryId) -> String {
        id.0
    }
}
