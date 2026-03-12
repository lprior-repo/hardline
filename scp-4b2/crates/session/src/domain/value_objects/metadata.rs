//! Metadata value objects: Labels, DependsOn, Priority, IssueType, WorkspaceName

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

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
