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
            return Err(Error::invalid_state("Path must be absolute".to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- Valid paths --

    #[test]
    fn test_valid_root() {
        assert!(AbsolutePath::parse("/").is_ok());
    }

    #[test]
    fn test_valid_home() {
        assert!(AbsolutePath::parse("/home/user").is_ok());
    }

    #[test]
    fn test_valid_tmp() {
        assert!(AbsolutePath::parse("/tmp").is_ok());
    }

    #[test]
    fn test_valid_deep_path() {
        assert!(AbsolutePath::parse("/a/b/c/d/e/f").is_ok());
    }

    #[test]
    fn test_valid_trailing_slash() {
        assert!(AbsolutePath::parse("/home/user/").is_ok());
    }

    #[test]
    fn test_valid_with_dots() {
        assert!(AbsolutePath::parse("/home/user/./docs/../workspace").is_ok());
    }

    #[test]
    fn test_valid_from_pathbuf() {
        let pb = PathBuf::from("/home/user");
        assert!(AbsolutePath::parse(pb).is_ok());
    }

    // -- Invalid paths --

    #[test]
    fn test_reject_relative() {
        assert!(AbsolutePath::parse("relative/path").is_err());
    }

    #[test]
    fn test_reject_empty() {
        assert!(AbsolutePath::parse("").is_err());
    }

    #[test]
    fn test_reject_dot() {
        assert!(AbsolutePath::parse(".").is_err());
    }

    #[test]
    fn test_reject_dotdot() {
        assert!(AbsolutePath::parse("..").is_err());
    }

    #[test]
    fn test_reject_tilde() {
        assert!(AbsolutePath::parse("~/workspace").is_err());
    }

    // -- Display --

    #[test]
    fn test_display_roundtrip() {
        let path = AbsolutePath::parse("/home/user/workspace").expect("valid");
        assert_eq!(format!("{path}"), "/home/user/workspace");
    }

    #[test]
    fn test_display_root() {
        let path = AbsolutePath::parse("/").expect("valid");
        assert_eq!(format!("{path}"), "/");
    }

    // -- as_path / as_str --

    #[test]
    fn test_as_path_returns_inner() {
        let path = AbsolutePath::parse("/home/user").expect("valid");
        assert_eq!(path.as_path(), &PathBuf::from("/home/user"));
    }

    #[test]
    fn test_as_str() {
        let path = AbsolutePath::parse("/tmp/test").expect("valid");
        assert_eq!(path.as_str(), "/tmp/test");
    }

    // -- PartialEq --

    #[test]
    fn test_equality() {
        let a = AbsolutePath::parse("/home/user").expect("valid");
        let b = AbsolutePath::parse("/home/user").expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn test_inequality() {
        let a = AbsolutePath::parse("/home/user").expect("valid");
        let b = AbsolutePath::parse("/home/other").expect("valid");
        assert_ne!(a, b);
    }

    // -- FromStr --

    #[test]
    fn test_from_str_valid() {
        use std::str::FromStr;
        let path = AbsolutePath::from_str("/home/user");
        assert!(path.is_ok());
    }

    #[test]
    fn test_from_str_invalid() {
        use std::str::FromStr;
        let path = AbsolutePath::from_str("relative");
        assert!(path.is_err());
    }

    // -- From<String> bypasses validation --

    #[test]
    fn test_from_string_bypasses_validation() {
        let _path = AbsolutePath::from(String::from("relative"));
    }

    // -- Serde roundtrip --

    #[test]
    fn test_serde_roundtrip() {
        let path = AbsolutePath::parse("/home/user/workspace").expect("valid");
        let json = serde_json::to_string(&path).expect("serialize ok");
        let deserialized: AbsolutePath = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(path, deserialized);
        assert_eq!(deserialized.as_str(), "/home/user/workspace");
    }

    #[test]
    fn test_serde_roundtrip_root() {
        let path = AbsolutePath::parse("/").expect("valid");
        let json = serde_json::to_string(&path).expect("serialize ok");
        let deserialized: AbsolutePath = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(path, deserialized);
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_absolute_path_with_leading_slash_always_valid(path in "/[a-zA-Z0-9_/]+") {
            assert!(AbsolutePath::parse(&path).is_ok(), "path: {path}");
        }

        #[test]
        fn prop_relative_path_always_invalid(path in "[a-zA-Z0-9_/]+") {
            // Only accept paths that don't start with /
            if !path.starts_with('/') {
                assert!(AbsolutePath::parse(&path).is_err(), "path: {path}");
            }
        }

        #[test]
        fn prop_absolute_path_roundtrips_through_serde(path in "/[a-zA-Z0-9_/]+") {
            let parsed = AbsolutePath::parse(&path).expect("valid");
            let json = serde_json::to_string(&parsed).expect("serialize ok");
            let deserialized: AbsolutePath =
                serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(parsed, deserialized);
        }
    }
}
