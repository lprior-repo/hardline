use serde::{Deserialize, Serialize};

use crate::error::{BeadError, Result};

/// BeadTitle - title value object with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// BeadDescription - description value object with validation
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
