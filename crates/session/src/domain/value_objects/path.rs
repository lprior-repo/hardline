//! AbsolutePath value object.
//!
//! This module is split into multiple files to keep each under 300 lines:
//! - path_errors.rs: Error types
//! - path_validation.rs: Pure validation functions
//! - path.rs: AbsolutePath type and TryFrom implementations
//! - path_tests.rs: All tests

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod path_errors;
pub mod path_validation;

// Re-export for tests
pub use path_errors::{AbsolutePathError, PathValidationError, ShellMetacharacterError};
pub use path_validation::find_first_metacharacter;
use path_validation::{validate_is_absolute, validate_no_metacharacters, validate_utf8};

/// An owned, validated filesystem path that is guaranteed to be absolute and shell-safe.
///
/// # Invariants
/// - `self.path.is_absolute() == true`
/// - Path contains no shell metacharacters: `$`, `` ` ``, `;`, `|`, `&`
/// - Path is valid UTF-8
///
/// # Construction
/// Use `TryFrom<&str>` or `TryFrom<PathBuf>` to construct:
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    /// Create an AbsolutePath after validating all preconditions.
    fn validate_and_create(path: PathBuf) -> Result<Self, AbsolutePathError> {
        let path_str = path.to_string_lossy();
        validate_utf8(&path)?;
        validate_is_absolute(&path)?;
        validate_no_metacharacters(&path_str)?;
        Ok(Self(path))
    }

    /// Get the underlying Path reference.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Convert to PathBuf (cloned).
    #[must_use]
    pub fn to_path_buf(&self) -> PathBuf {
        self.0.clone()
    }
}

impl TryFrom<PathBuf> for AbsolutePath {
    type Error = AbsolutePathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::validate_and_create(path)
    }
}

impl TryFrom<&str> for AbsolutePath {
    type Error = AbsolutePathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::validate_and_create(PathBuf::from(path))
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_path().display())
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[cfg(test)]
mod path_tests;

#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn absolute_path_serde_roundtrip() {
        let path = AbsolutePath::try_from("/usr/local/bin").expect("valid");
        let json = serde_json::to_string(&path).expect("serialize");
        let parsed: AbsolutePath = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(path, parsed);
    }

    #[test]
    fn absolute_path_serde_json_output() {
        let path = AbsolutePath::try_from("/etc/config").expect("valid");
        let json = serde_json::to_string(&path).expect("serialize");
        assert!(json.contains("/etc/config"));
    }

    #[test]
    fn absolute_path_serde_with_unicode() {
        let path = AbsolutePath::try_from("/home/user/data").expect("valid");
        let json = serde_json::to_string(&path).expect("serialize");
        let parsed: AbsolutePath = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(path.to_path_buf(), parsed.to_path_buf());
    }
}
