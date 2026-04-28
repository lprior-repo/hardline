//! Validated task ID and bead ID
//!
//! Semantic newtypes for task and bead identifiers that guarantee valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::{error::IdentifierError, validation::validate_task_id};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TaskId(String);

impl TaskId {
    /// Parses a task ID from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a valid task ID.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_task_id(&s)?;
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for TaskId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- TaskId --

    #[test]
    fn parse_valid_task_id() {
        let id = TaskId::parse("bd-1a2b3c").expect("valid");
        assert_eq!(id.as_str(), "bd-1a2b3c");
    }

    #[test]
    fn parse_empty_rejects() {
        assert!(TaskId::parse("").is_err());
    }

    #[test]
    fn parse_missing_prefix_rejects() {
        assert!(TaskId::parse("1a2b3c").is_err());
        assert!(TaskId::parse("task-1a2b").is_err());
    }

    #[test]
    fn parse_empty_hex_rejects() {
        assert!(TaskId::parse("bd-").is_err());
    }

    #[test]
    fn parse_non_hex_characters_rejects() {
        assert!(TaskId::parse("bd-xyz").is_err());
        assert!(TaskId::parse("bd-1g2h").is_err());
    }

    #[test]
    fn parse_all_hex_digits() {
        let id = TaskId::parse("bd-0123456789abcdefABCDEF").expect("valid");
        assert_eq!(id.as_str(), "bd-0123456789abcdefABCDEF");
    }

    #[test]
    fn task_id_display() {
        let id = TaskId::parse("bd-ab").expect("ok");
        assert_eq!(format!("{id}"), "bd-ab");
    }

    #[test]
    fn task_id_try_from_string() {
        let id = TaskId::try_from("bd-ff".to_string()).expect("ok");
        assert_eq!(id.as_str(), "bd-ff");
    }

    #[test]
    fn task_id_equality() {
        let a = TaskId::parse("bd-1").expect("ok");
        let b = TaskId::parse("bd-1").expect("ok");
        let c = TaskId::parse("bd-2").expect("ok");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // -- BeadId (same validation rules) --

    #[test]
    fn parse_valid_bead_id() {
        let id = BeadId::parse("bd-ff00aa").expect("valid");
        assert_eq!(id.as_str(), "bd-ff00aa");
    }

    #[test]
    fn bead_id_missing_prefix_rejects() {
        assert!(BeadId::parse("abc").is_err());
    }

    #[test]
    fn bead_id_display() {
        let id = BeadId::parse("bd-01").expect("ok");
        assert_eq!(format!("{id}"), "bd-01");
    }

    #[test]
    fn bead_id_equality() {
        let a = BeadId::parse("bd-1").expect("ok");
        let b = BeadId::parse("bd-1").expect("ok");
        assert_eq!(a, b);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct BeadId(String);

impl BeadId {
    /// Parses a bead ID from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a valid bead ID.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_task_id(&s)?;
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for BeadId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for BeadId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for BeadId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
