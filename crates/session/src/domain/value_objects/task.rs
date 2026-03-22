//! Task-related value objects: AgentId, TaskId, Title, Description

use serde::{Deserialize, Serialize};

use crate::error::{SessionError, TaskIdError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "AgentId cannot be empty".into(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for AgentId {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn parse(id: impl Into<String>) -> Result<Self, TaskIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(TaskIdError::InvalidInput);
        }
        if !id.starts_with("bd-") {
            return Err(TaskIdError::InvalidPrefix);
        }
        let suffix = &id[3..];
        if suffix.is_empty() {
            return Err(TaskIdError::EmptySuffix);
        }
        if !suffix.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TaskIdError::InvalidHex);
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for TaskId {
    type Error = TaskIdError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for TaskId {
    type Error = TaskIdError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self, SessionError> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "Title cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
                "Title exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Title {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Description(String);

impl Description {
    pub const MAX_LENGTH: usize = 10000;

    pub fn new(desc: impl Into<String>) -> Result<Self, SessionError> {
        let desc = desc.into();
        if desc.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
                "Description exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(desc))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Description {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_valid_taskid_with_numeric_suffix() {
        let result = TaskId::parse("bd-123abc");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-123abc");
    }

    #[test]
    fn test_parses_valid_taskid_with_alphanumeric_hex() {
        let result = TaskId::parse("bd-deadbeef");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-deadbeef");
    }

    #[test]
    fn test_parses_valid_taskid_case_insensitive_hex() {
        let result = TaskId::parse("bd-ABCDEF");
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert_eq!(task_id.as_str(), "bd-ABCDEF");
    }

    #[test]
    fn test_try_from_str_trait_valid_input() {
        let result = TaskId::try_from("bd-f00ba7");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-f00ba7");
    }

    #[test]
    fn test_parse_empty_string_returns_invalid_input_error() {
        let result = TaskId::parse("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidInput));
    }

    #[test]
    fn test_parse_missing_prefix_returns_invalid_prefix_error() {
        let result = TaskId::parse("abc-123");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidPrefix));
    }

    #[test]
    fn test_parse_invalid_hex_returns_invalid_hex_error() {
        let result = TaskId::parse("bd-xyz");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidHex));
    }

    #[test]
    fn test_parse_empty_suffix_returns_empty_suffix_error() {
        let result = TaskId::parse("bd-");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::EmptySuffix));
    }

    #[test]
    fn test_try_from_str_trait_invalid_input() {
        let result = TaskId::try_from("invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskIdError::InvalidPrefix));
    }

    #[test]
    fn test_single_hex_digit_suffix() {
        let result = TaskId::parse("bd-a");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-a");
    }

    #[test]
    fn test_mixed_case_hex() {
        let result = TaskId::parse("bd-AbCdEf123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bd-AbCdEf123456");
    }

    #[test]
    fn test_to_string_always_starts_with_bd_prefix() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert!(task_id.to_string().starts_with("bd-"));
    }

    #[test]
    fn test_as_str_returns_valid_string_slice() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id.as_str(), "bd-abc123");
    }

    #[test]
    fn test_roundtrip_parse_to_string() {
        let original = TaskId::parse("bd-abc123").unwrap();
        let reparsed = TaskId::parse(original.to_string()).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn test_equality_based_on_value() {
        let task_id1 = TaskId::parse("bd-abc123").unwrap();
        let task_id2 = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id1, task_id2);
    }

    #[test]
    fn test_display_trait_outputs_correct_format() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        let formatted = format!("{}", task_id);
        assert_eq!(formatted, "bd-abc123");
    }

    #[test]
    fn test_into_inner_returns_original_string() {
        let task_id = TaskId::parse("bd-abc123").unwrap();
        assert_eq!(task_id.into_inner(), "bd-abc123");
    }
}
