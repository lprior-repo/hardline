//! Session name newtype with validation
//!
//! Session names must:
//! - Be 1-63 characters
//! - Start with a letter
//! - Contain only letters, numbers, dashes, and underscores

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    const MAX_LENGTH: usize = 63;

    pub fn parse(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(Error::InvalidState(
                "Session name cannot be empty".to_string(),
            ));
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(Error::InvalidState(format!(
                "Session name cannot exceed {} characters",
                Self::MAX_LENGTH
            )));
        }
        let first_char = name
            .chars()
            .next()
            .ok_or_else(|| Error::InvalidState("Session name cannot be empty".to_string()))?;
        if !first_char.is_ascii_alphabetic() {
            return Err(Error::InvalidState(
                "Session name must start with a letter".to_string(),
            ));
        }
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid_chars {
            return Err(Error::InvalidState(
                "Session name can only contain letters, numbers, dashes, and underscores"
                    .to_string(),
            ));
        }
        Ok(Self(name))
    }

    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::parse(name)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for SessionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
