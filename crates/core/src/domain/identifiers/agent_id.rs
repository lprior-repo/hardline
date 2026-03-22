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
/// use isolate_core::domain::AgentId;
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
