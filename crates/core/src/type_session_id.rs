//! Session ID newtype with validation
//!
//! Session IDs must contain only alphanumeric characters and hyphens.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn parse(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::invalid_state(
                "Session ID cannot be empty".to_string(),
            ));
        }
        let valid_chars = id.chars().all(|c| c.is_alphanumeric() || c == '-');
        if !valid_chars {
            return Err(Error::invalid_state(
                "Session ID can only contain alphanumeric characters and hyphens".to_string(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
