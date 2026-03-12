//! Path-related value objects: AbsolutePath

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

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
