use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    #[error("identifier cannot be empty")]
    Empty,
    #[error("identifier too long: {0} characters (max {1})")]
    TooLong(usize, usize),
    #[error("identifier contains invalid characters: {0}")]
    InvalidCharacters(String),
    #[error("identifier must start with a letter")]
    InvalidStart,
    #[error("identifier must be ASCII only")]
    NotAscii,
}

pub type SessionNameError = IdentifierError;
pub type WorkspaceIdError = IdentifierError;
pub type BeadIdError = IdentifierError;

fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if s.len() > 63 {
        return Err(IdentifierError::TooLong(s.len(), 63));
    }
    if !s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(IdentifierError::InvalidStart);
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IdentifierError::InvalidCharacters(
            "must contain only letters, numbers, hyphens, or underscores".into(),
        ));
    }
    Ok(())
}

fn validate_hex_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if !s.starts_with("bd-") {
        return Err(IdentifierError::InvalidCharacters(
            "must start with 'bd-'".into(),
        ));
    }
    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentifierError::InvalidCharacters(
            "must be valid hex after 'bd-'".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    pub const MAX_LENGTH: usize = 63;

    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        if s.is_empty() {
            return Err(IdentifierError::Empty);
        }
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        Self(format!("ws-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_hex_id(&s)?;
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        let hex = format!("{:x}", uuid::Uuid::new_v4());
        Self(format!("bd-{}", &hex[..12]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(name: impl Into<String>) -> Result<Self, SessionError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "WorkspaceName cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidIdentifier(format!(
                "WorkspaceName exceeds maximum length of {}",
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

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for WorkspaceName {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// =============================================================================
// New domain types for scp-31h
// =============================================================================

use crate::error::SessionError;

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
    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "TaskId cannot be empty".into(),
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

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for TaskId {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbsolutePath(String);

impl AbsolutePath {
    pub fn new(path: impl Into<String>) -> Result<Self, SessionError> {
        let path = path.into();
        if path.is_empty() {
            return Err(SessionError::InvalidPath("Path cannot be empty".into()));
        }
        if !path.starts_with('/') {
            return Err(SessionError::InvalidPath(
                "Path must be absolute (must start with /)".into(),
            ));
        }
        Ok(Self(path))
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

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for AbsolutePath {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(Vec<String>);

impl Labels {
    pub const MAX_LABELS: usize = 50;

    pub fn new(labels: Vec<String>) -> Result<Self, SessionError> {
        if labels.len() > Self::MAX_LABELS {
            return Err(SessionError::InvalidIdentifier(format!(
                "Too many labels (max {})",
                Self::MAX_LABELS
            )));
        }
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        if unique.len() != labels.len() {
            return Err(SessionError::InvalidIdentifier(
                "Labels contain duplicates".into(),
            ));
        }
        Ok(Self(labels))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl std::fmt::Display for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependsOn(String);

impl DependsOn {
    pub fn new(bead_id: impl Into<String>) -> Result<Self, SessionError> {
        let bead_id = bead_id.into();
        if bead_id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn cannot be empty".into(),
            ));
        }
        Ok(Self(bead_id))
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

impl std::fmt::Display for DependsOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for DependsOn {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(priority: u8) -> Result<Self, SessionError> {
        if priority > 4 {
            return Err(SessionError::InvalidPriority(format!(
                "Priority must be 0-4, got {}",
                priority
            )));
        }
        Ok(Self(priority))
    }

    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    #[must_use]
    pub fn into_inner(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u8> for Priority {
    type Error = SessionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueType(String);

impl IssueType {
    pub fn new(issue_type: impl Into<String>) -> Result<Self, SessionError> {
        let issue_type = issue_type.into();
        let valid_types = ["bug", "feature", "task", "epic", "chore"];
        if !valid_types.contains(&issue_type.as_str()) {
            return Err(SessionError::InvalidIssueType(format!(
                "Invalid issue type: {}. Must be one of: {}",
                issue_type,
                valid_types.join(", ")
            )));
        }
        Ok(Self(issue_type))
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

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IssueType {
    type Error = SessionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_session_value_object_name_adversarial(s in ".*") {
            let res = SessionName::parse(s.clone());
            let trimmed = s.trim();

            if let Ok(name) = res {
                let name_str = name.as_str();
                prop_assert!(!name_str.is_empty(), "Empty string allowed");
                prop_assert!(name_str.len() <= SessionName::MAX_LENGTH, "Max length exceeded: {} > {}", name_str.len(), SessionName::MAX_LENGTH);

                let first_char = name_str.chars().next().unwrap();
                prop_assert!(first_char.is_ascii_alphabetic(), "First char not ascii alphabetic: {}", first_char);

                let valid_chars = name_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                prop_assert!(valid_chars, "Contains invalid chars: {}", name_str);
            } else {
                // If it failed, it must violate one of the rules.
                let violates_rules = trimmed.is_empty()
                    || trimmed.len() > SessionName::MAX_LENGTH
                    || !trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                    || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

                prop_assert!(violates_rules, "Valid string was rejected: {:?}", s);
            }
        }
    }

    #[test]
    fn test_valid_session_name() {
        assert!(SessionName::parse("my-session").is_ok());
        assert!(SessionName::parse("test_123").is_ok());
    }

    #[test]
    fn test_invalid_session_name_empty() {
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn test_valid_bead_id() {
        assert!(BeadId::parse("bd-abc123").is_ok());
    }

    #[test]
    fn test_invalid_bead_id_no_prefix() {
        assert!(BeadId::parse("abc123").is_err());
    }
}
