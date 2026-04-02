//! Validated workspace name
//!
//! A semantic newtype for workspace names that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::validation::validate_workspace_name;

/// A validated workspace name
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::WorkspaceName;
///
/// let workspace = WorkspaceName::parse("my-workspace")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - No path separators or null bytes
/// - 1-255 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct WorkspaceName(String);

impl WorkspaceName {
    /// Parse and validate a workspace name
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the name is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_workspace_name(&s)?;
        Ok(Self(s))
    }

    /// Get the workspace name as a string slice
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

impl TryFrom<String> for WorkspaceName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for WorkspaceName {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for WorkspaceName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_name() {
        let name = WorkspaceName::parse("my-workspace").expect("valid");
        assert_eq!(name.as_str(), "my-workspace");
    }

    #[test]
    fn parse_with_underscores() {
        let name = WorkspaceName::parse("my_workspace_01").expect("valid");
        assert_eq!(name.as_str(), "my_workspace_01");
    }

    #[test]
    fn parse_with_dots() {
        let name = WorkspaceName::parse("workspace.v2").expect("valid");
        assert_eq!(name.as_str(), "workspace.v2");
    }

    #[test]
    fn parse_empty_rejects() {
        assert!(WorkspaceName::parse("").is_err());
    }

    #[test]
    fn parse_forward_slash_rejects() {
        assert!(WorkspaceName::parse("path/name").is_err());
    }

    #[test]
    fn parse_backslash_rejects() {
        assert!(WorkspaceName::parse("path\\name").is_err());
    }

    #[test]
    fn parse_null_byte_rejects() {
        assert!(WorkspaceName::parse("path\0name").is_err());
    }

    #[test]
    fn parse_too_long_rejects() {
        let long_name = "a".repeat(256);
        assert!(WorkspaceName::parse(long_name).is_err());
    }

    #[test]
    fn parse_max_length_is_ok() {
        let name = "a".repeat(255);
        let result = WorkspaceName::parse(name);
        assert!(result.is_ok());
    }

    #[test]
    fn display_shows_inner() {
        let name = WorkspaceName::parse("test-ws").expect("ok");
        assert_eq!(format!("{name}"), "test-ws");
    }

    #[test]
    fn try_from_string() {
        let name = WorkspaceName::try_from("test-ws".to_string()).expect("ok");
        assert_eq!(name.as_str(), "test-ws");
    }

    #[test]
    fn try_from_str() {
        let name = WorkspaceName::try_from("test-ws").expect("ok");
        assert_eq!(name.as_str(), "test-ws");
    }

    #[test]
    fn equality() {
        let a = WorkspaceName::parse("same").expect("ok");
        let b = WorkspaceName::parse("same").expect("ok");
        let c = WorkspaceName::parse("different").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
