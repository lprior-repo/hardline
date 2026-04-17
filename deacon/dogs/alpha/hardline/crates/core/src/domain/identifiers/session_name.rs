//! Validated session name
//!
//! A semantic newtype for session names that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::validation::validate_session_name;

/// A validated session name
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::SessionName;
///
/// // Parse and validate
/// let name = SessionName::parse("my-session")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with a letter
/// - Contains only alphanumeric, hyphen, underscore
/// - 1-63 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SessionName(String);

impl SessionName {
    /// Maximum allowed length for a session name
    pub const MAX_LENGTH: usize = 63;

    /// Parse and validate a session name (trims whitespace first)
    ///
    /// This follows the "parse at boundaries" DDD principle:
    /// - Trims whitespace from input
    /// - Validates once at construction
    /// - Cannot represent invalid states
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the name is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    /// Get the session name as a string slice
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

impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SessionName {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for SessionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<SessionName> for String {
    #[allow(clippy::use_self)] // Self refers to String, not SessionName
    fn from(name: SessionName) -> String {
        name.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_name() {
        let name = SessionName::parse("my-session").expect("valid");
        assert_eq!(name.as_str(), "my-session");
    }

    #[test]
    fn parse_with_numbers() {
        let name = SessionName::parse("session-123").expect("valid");
        assert_eq!(name.as_str(), "session-123");
    }

    #[test]
    fn parse_trims_whitespace() {
        let name = SessionName::parse("  my-session  ").expect("valid");
        assert_eq!(name.as_str(), "my-session");
    }

    #[test]
    fn parse_empty_rejects() {
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn parse_whitespace_only_rejects() {
        assert!(SessionName::parse("   ").is_err());
    }

    #[test]
    fn parse_must_start_with_letter() {
        assert!(SessionName::parse("1session").is_err());
        assert!(SessionName::parse("-session").is_err());
        assert!(SessionName::parse("_session").is_err());
    }

    #[test]
    fn parse_special_chars_reject() {
        assert!(SessionName::parse("session$test").is_err());
        assert!(SessionName::parse("session test").is_err());
        assert!(SessionName::parse("session@test").is_err());
    }

    #[test]
    fn parse_too_long_rejects() {
        let long_name = "a".repeat(64);
        assert!(SessionName::parse(long_name).is_err());
    }

    #[test]
    fn parse_max_length_is_ok() {
        let name = "a".repeat(63);
        let result = SessionName::parse(name);
        assert!(result.is_ok());
    }

    #[test]
    fn display_shows_inner() {
        let name = SessionName::parse("test-session").expect("ok");
        assert_eq!(format!("{name}"), "test-session");
    }

    #[test]
    fn from_into_string() {
        let name = SessionName::parse("test").expect("ok");
        let s: String = name.into();
        assert_eq!(s, "test");
    }

    #[test]
    fn try_from_string() {
        let name = SessionName::try_from("valid-session".to_string()).expect("ok");
        assert_eq!(name.as_str(), "valid-session");
    }

    #[test]
    fn try_from_str() {
        let name = SessionName::try_from("valid-session").expect("ok");
        assert_eq!(name.as_str(), "valid-session");
    }

    #[test]
    fn equality() {
        let a = SessionName::parse("same").expect("ok");
        let b = SessionName::parse("same").expect("ok");
        let c = SessionName::parse("different").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
