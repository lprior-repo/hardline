use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(BeadError::InvalidId("ID cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidId(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(BeadError::InvalidId(
                "ID must contain only alphanumeric characters, hyphens, and underscores".into(),
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

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadId {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for BeadId {
    type Error = BeadError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadTitle(String);

impl BeadTitle {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(BeadError::InvalidTitle("Title cannot be empty".into()));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidTitle(format!(
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

impl std::fmt::Display for BeadTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadTitle {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadDescription(String);

impl BeadDescription {
    pub const MAX_LENGTH: usize = 10_000;

    pub fn new(description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        if description.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidTitle(format!(
                "Description exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(description))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for BeadDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadDescription {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
}

impl BeadState {
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    #[must_use]
    pub fn closed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(*closed_at),
            _ => None,
        }
    }

    pub fn transition_to(&self, new_state: Self) -> Result<Self> {
        if matches!(new_state, Self::Closed { .. }) && !matches!(self, Self::Closed { .. }) {
            return Ok(Self::Closed {
                closed_at: Utc::now(),
            });
        }
        Ok(new_state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    pub fn from_value(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            _ => Self::P4,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(pub Vec<String>);

impl Labels {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(mut self, label: impl Into<String>) -> Self {
        self.0.push(label.into());
        self
    }

    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::new()
    }
}
