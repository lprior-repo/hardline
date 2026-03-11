#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Domain identifiers - Newtype pattern with validation
//!
//! All identifiers are validated on construction and immutable.

use crate::domain::validation::{ValidationError, ValidationResult};

const SHELL_METACHARACTERS: &str = "$`|&<>\n\r\x00";

/// Unique queue entry identifier
///
/// Wrapper around a String that ensures non-empty values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    /// Create a new queue entry ID with validation.
    ///
    /// # Errors
    /// Returns `ValidationError::EmptyValue` if the ID is empty.
    pub fn new(id: impl Into<String>) -> ValidationResult<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            Err(ValidationError::EmptyValue("QueueEntryId".to_string()))
        } else {
            Ok(Self(id))
        }
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for QueueEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session name - a validated identifier for queue sessions
///
/// Wrapper that ensures:
/// - Non-empty after trimming
/// - No shell metacharacters (prevents command injection)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionName(String);

impl SessionName {
    /// Create a new session name with validation.
    ///
    /// # Errors
    /// Returns `ValidationError` if:
    /// - The name is empty after trimming
    /// - The name contains shell metacharacters
    pub fn new(name: impl Into<String>) -> ValidationResult<Self> {
        let name = name.into();
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::EmptyValue("SessionName".to_string()));
        }

        for c in SHELL_METACHARACTERS.chars() {
            if trimmed.contains(c) {
                return Err(ValidationError::InvalidCharacters {
                    field: "SessionName".to_string(),
                    found: c.to_string(),
                });
            }
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Validate a session name without consuming it.
    ///
    /// # Errors
    /// Returns `ValidationError` if the name is invalid.
    pub fn validate(name: &str) -> ValidationResult<()> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::EmptyValue("SessionName".to_string()));
        }

        for c in SHELL_METACHARACTERS.chars() {
            if trimmed.contains(c) {
                return Err(ValidationError::InvalidCharacters {
                    field: "SessionName".to_string(),
                    found: c.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get the session name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for SessionName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SessionName {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_entry_id_valid() {
        assert!(QueueEntryId::new("test-123").is_ok());
        assert!(QueueEntryId::new("  test-123  ").is_ok());
    }

    #[test]
    fn test_queue_entry_id_empty() {
        assert!(matches!(
            QueueEntryId::new(""),
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            QueueEntryId::new("   "),
            Err(ValidationError::EmptyValue(_))
        ));
    }

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::new("my-session").is_ok());
        assert!(SessionName::new("  my-session  ").is_ok());
        assert!(SessionName::new("session_123").is_ok());
        assert!(SessionName::new("session.with.dots").is_ok());
    }

    #[test]
    fn test_session_name_empty() {
        assert!(matches!(
            SessionName::new(""),
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            SessionName::new("   "),
            Err(ValidationError::EmptyValue(_))
        ));
    }

    #[test]
    fn test_session_name_rejects_shell_metacharacters() {
        let invalid_chars = ["$", "`", "|", "&", "<", ">", "\n", "\r", "\x00"];
        for c in invalid_chars {
            let test_name = format!("session{}name", c);
            assert!(
                matches!(
                    SessionName::new(&test_name),
                    Err(ValidationError::InvalidCharacters { .. })
                ),
                "Should reject character: {:?}",
                c
            );
        }
    }

    #[test]
    fn test_session_name_validate_works() {
        assert!(SessionName::validate("valid-name").is_ok());
        assert!(SessionName::validate("invalid$name").is_err());
    }

    #[test]
    fn test_session_name_try_from() {
        assert!(SessionName::try_from("valid".to_string()).is_ok());
        assert!(SessionName::try_from("valid").is_ok());
        assert!(SessionName::try_from("").is_err());
    }
}
