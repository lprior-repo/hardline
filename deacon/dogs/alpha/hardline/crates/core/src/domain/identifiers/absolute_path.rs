//! Validated absolute path
//!
//! A semantic newtype for absolute filesystem paths that guarantees valid states.

use serde::{Deserialize, Serialize};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::validation::validate_absolute_path;

/// A validated absolute path
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::AbsolutePath;
///
/// let path = AbsolutePath::parse("/home/user/workspace")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Always absolute (starts with `/` on Unix)
/// - No null bytes
/// - Suitable for filesystem operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct AbsolutePath(String);

impl AbsolutePath {
    /// Parse and validate an absolute path
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the path is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_absolute_path(&s)?;
        Ok(Self(s))
    }

    /// Get the path as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Convert to `std::path::PathBuf`
    #[must_use]
    pub fn to_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.0)
    }

    /// Display the path (for error messages)
    #[must_use]
    pub fn display(&self) -> impl std::fmt::Display + '_ {
        struct DisplayPath<'a>(&'a str);

        impl std::fmt::Display for DisplayPath<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        DisplayPath(&self.0)
    }

    /// Check if the path exists on the filesystem
    #[must_use]
    pub fn exists(&self) -> bool {
        self.to_path_buf().exists()
    }
}

impl TryFrom<String> for AbsolutePath {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for AbsolutePath {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AbsolutePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
