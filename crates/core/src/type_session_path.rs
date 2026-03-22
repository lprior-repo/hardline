//! Absolute path newtype with validation
//!
//! Ensures paths are absolute (not relative).

use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    pub fn parse(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(Error::InvalidState("Path must be absolute".to_string()));
        }
        Ok(Self(path))
    }

    #[must_use]
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap_or("")
    }
}

impl From<String> for AbsolutePath {
    fn from(s: String) -> Self {
        Self(PathBuf::from(s))
    }
}

impl FromStr for AbsolutePath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}
