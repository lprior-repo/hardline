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
