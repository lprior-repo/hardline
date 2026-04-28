//! Validated session ID
//!
//! A semantic newtype for session identifiers that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::{error::IdentifierError, validation::validate_session_id};

/// A validated session ID
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use scp_core::SessionId;
///
/// let id = SessionId::parse("session-abc123")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - ASCII only
/// - Suitable for use as unique identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SessionId(String);

impl SessionId {
    /// Parse and validate a session ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_session_id(&s)?;
        Ok(Self(s))
    }

    /// Get the session ID as a string slice
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

impl TryFrom<String> for SessionId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for SessionId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse() --

    #[test]
    fn parse_valid_id() {
        let id = SessionId::parse("session-abc123").expect("valid id");
        assert_eq!(id.as_str(), "session-abc123");
    }

    #[test]
    fn parse_empty_rejects() {
        let result = SessionId::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_non_ascii_rejects() {
        let result = SessionId::parse("session-cafe\u{301}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_spaces_is_valid() {
        // Session IDs allow any printable ASCII, including spaces
        let id = SessionId::parse("session abc").expect("valid id");
        assert_eq!(id.as_str(), "session abc");
    }

    #[test]
    fn parse_with_dots_and_hyphens() {
        let id = SessionId::parse("session-abc.123").expect("valid id");
        assert_eq!(id.as_str(), "session-abc.123");
    }

    // -- as_str / into_string --

    #[test]
    fn as_str_returns_inner() {
        let id = SessionId::parse("my-session").expect("ok");
        assert_eq!(id.as_str(), "my-session");
    }

    #[test]
    fn into_string_returns_ownership() {
        let id = SessionId::parse("my-session").expect("ok");
        let s = id.into_string();
        assert_eq!(s, "my-session");
    }

    // -- Display --

    #[test]
    fn display_shows_inner() {
        let id = SessionId::parse("test-session").expect("ok");
        assert_eq!(format!("{id}"), "test-session");
    }

    // -- AsRef --

    #[test]
    fn as_ref_str() {
        let id = SessionId::parse("test-id").expect("ok");
        assert_eq!(id.as_ref(), "test-id");
    }

    // -- TryFrom --

    #[test]
    fn try_from_string() {
        let id = SessionId::try_from("test-id".to_string()).expect("ok");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn try_from_str() {
        let id = SessionId::try_from("test-id").expect("ok");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn try_from_empty_fails() {
        assert!(SessionId::try_from("".to_string()).is_err());
    }

    // -- Eq / Hash --

    #[test]
    fn equality() {
        let a = SessionId::parse("same").expect("ok");
        let b = SessionId::parse("same").expect("ok");
        let c = SessionId::parse("different").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
