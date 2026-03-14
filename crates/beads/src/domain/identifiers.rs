use serde::{Deserialize, Serialize};

use crate::error::{BeadError, Result};

/// BeadId - unique identifier for a bead with validation
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

/// AgentId - unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
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
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if value.is_empty() {
            Err(BeadError::InvalidId("AgentId cannot be empty".into()))
        } else {
            Ok(Self(value))
        }
    }
}
