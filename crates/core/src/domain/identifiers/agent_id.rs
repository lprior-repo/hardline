//! Validated agent ID
//!
//! A semantic newtype for agent identifiers that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::validation::validate_agent_id;

/// A validated agent ID
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use scp_core::domain::AgentId;
///
/// let agent = AgentId::parse("agent-123")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Contains only alphanumeric, hyphen, underscore, dot, colon
/// - 1-128 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct AgentId(String);

impl AgentId {
    /// Parse and validate an agent ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_agent_id(&s)?;
        Ok(Self(s))
    }

    /// Get the agent ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Generate a default agent ID from process ID
    #[must_use]
    pub fn from_process() -> Self {
        Self(format!("pid-{}", std::process::id()))
    }
}

impl TryFrom<String> for AgentId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for AgentId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse() --

    #[test]
    fn parse_valid_agent_id() {
        let id = AgentId::parse("agent-123").expect("valid");
        assert_eq!(id.as_str(), "agent-123");
    }

    #[test]
    fn parse_with_underscore() {
        let id = AgentId::parse("my_agent_01").expect("valid");
        assert_eq!(id.as_str(), "my_agent_01");
    }

    #[test]
    fn parse_with_dots() {
        let id = AgentId::parse("agent.v2.beta").expect("valid");
        assert_eq!(id.as_str(), "agent.v2.beta");
    }

    #[test]
    fn parse_with_colons() {
        let id = AgentId::parse("agent:cli:001").expect("valid");
        assert_eq!(id.as_str(), "agent:cli:001");
    }

    #[test]
    fn parse_empty_rejects() {
        assert!(AgentId::parse("").is_err());
    }

    #[test]
    fn parse_too_long_rejects() {
        let long_id = "a".repeat(129);
        let result = AgentId::parse(long_id);
        assert!(result.is_err());
    }

    #[test]
    fn parse_max_length_is_ok() {
        let id = "a".repeat(128);
        let result = AgentId::parse(id);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_invalid_characters_rejects() {
        assert!(AgentId::parse("agent$123").is_err());
        assert!(AgentId::parse("agent 123").is_err());
        assert!(AgentId::parse("agent/123").is_err());
        assert!(AgentId::parse("agent@host").is_err());
    }

    #[test]
    fn parse_non_ascii_rejects() {
        assert!(AgentId::parse("agent-\u{00e9}").is_err());
    }

    // -- as_str / into_string --

    #[test]
    fn as_str_returns_inner() {
        let id = AgentId::parse("test-agent").expect("ok");
        assert_eq!(id.as_str(), "test-agent");
    }

    #[test]
    fn into_string_returns_ownership() {
        let id = AgentId::parse("test-agent").expect("ok");
        let s = id.into_string();
        assert_eq!(s, "test-agent");
    }

    // -- from_process() --

    #[test]
    fn from_process_generates_valid_id() {
        let id = AgentId::from_process();
        assert!(id.as_str().starts_with("pid-"));
        assert!(id.as_str().len() > 4);
    }

    // -- Display --

    #[test]
    fn display_shows_inner() {
        let id = AgentId::parse("cli-agent").expect("ok");
        assert_eq!(format!("{id}"), "cli-agent");
    }

    // -- AsRef --

    #[test]
    fn as_ref_str() {
        let id = AgentId::parse("test-id").expect("ok");
        assert_eq!(id.as_ref(), "test-id");
    }

    // -- TryFrom --

    #[test]
    fn try_from_string() {
        let id = AgentId::try_from("test-agent".to_string()).expect("ok");
        assert_eq!(id.as_str(), "test-agent");
    }

    #[test]
    fn try_from_str() {
        let id = AgentId::try_from("test-agent").expect("ok");
        assert_eq!(id.as_str(), "test-agent");
    }

    #[test]
    fn try_from_empty_fails() {
        assert!(AgentId::try_from("".to_string()).is_err());
        assert!(AgentId::try_from("").is_err());
    }

    // -- Eq / Hash --

    #[test]
    fn equality() {
        let a = AgentId::parse("same").expect("ok");
        let b = AgentId::parse("same").expect("ok");
        let c = AgentId::parse("different").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // -- Serde roundtrip --

    #[test]
    fn serde_serialize_roundtrip() {
        let id = AgentId::parse("agent-42").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"agent-42\"");
        let deserialized: AgentId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, id);
    }

    #[test]
    fn serde_deserialize_valid() {
        let json = "\"my.agent:01\"";
        let id: AgentId = serde_json::from_str(json).expect("deserialize valid");
        assert_eq!(id.as_str(), "my.agent:01");
    }

    #[test]
    fn serde_deserialize_invalid_rejects() {
        // Empty string should fail deserialization (try_from validates)
        let json = "\"\"";
        let result = serde_json::from_str::<AgentId>(json);
        assert!(result.is_err());
    }

    #[test]
    fn serde_deserialize_invalid_chars_rejects() {
        let json = "\"agent@host\"";
        let result = serde_json::from_str::<AgentId>(json);
        assert!(result.is_err());
    }

    #[test]
    fn serde_deserialize_too_long_rejects() {
        let long_val = "a".repeat(129);
        let json = format!("\"{long_val}\"");
        let result = serde_json::from_str::<AgentId>(&json);
        assert!(result.is_err());
    }

    // -- Error variant matching --

    #[test]
    fn parse_empty_returns_empty_error() {
        let err = AgentId::parse("").unwrap_err();
        assert!(matches!(err, IdentifierError::Empty));
    }

    #[test]
    fn parse_too_long_returns_correct_bounds() {
        let long = "a".repeat(129);
        let err = AgentId::parse(&long).unwrap_err();
        assert!(matches!(
            err,
            IdentifierError::TooLong { max: 128, actual: 129 }
        ));
    }

    #[test]
    fn parse_invalid_chars_returns_invalid_characters() {
        let err = AgentId::parse("bad!id").unwrap_err();
        assert!(matches!(err, IdentifierError::InvalidCharacters { .. }));
    }

    // -- Edge cases --

    #[test]
    fn parse_single_char() {
        let id = AgentId::parse("a").expect("single char valid");
        assert_eq!(id.as_str(), "a");
    }

    #[test]
    fn parse_all_valid_chars_combined() {
        let id = AgentId::parse("aB3-_.:xY9").expect("all valid chars");
        assert_eq!(id.as_str(), "aB3-_.:xY9");
    }

    #[test]
    fn parse_129_chars_fails() {
        let too_long = "x".repeat(129);
        assert!(AgentId::parse(&too_long).is_err());
    }

    #[test]
    fn parse_128_chars_succeeds() {
        let max = "x".repeat(128);
        assert!(AgentId::parse(&max).is_ok());
    }

    #[test]
    fn parse_rejects_null_byte() {
        assert!(AgentId::parse("agent\0id").is_err());
    }

    #[test]
    fn parse_rejects_newline() {
        assert!(AgentId::parse("agent\nid").is_err());
    }

    #[test]
    fn parse_rejects_tab() {
        assert!(AgentId::parse("agent\tid").is_err());
    }

    #[test]
    fn parse_rejects_unicode_emoji() {
        assert!(AgentId::parse("agent\u{1F600}").is_err());
    }
}
