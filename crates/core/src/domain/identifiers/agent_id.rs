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
}
