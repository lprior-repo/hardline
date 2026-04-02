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
    pub const MAX_LENGTH: usize = 63;

    pub fn parse(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(Error::invalid_state(
                "Session name cannot be empty".to_string(),
            ));
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(Error::invalid_state(format!(
                "Session name cannot exceed {} characters",
                Self::MAX_LENGTH
            )));
        }
        let first_char = name
            .chars()
            .next()
            .ok_or_else(|| Error::invalid_state("Session name cannot be empty".to_string()))?;
        if !first_char.is_ascii_alphabetic() {
            return Err(Error::invalid_state(
                "Session name must start with a letter".to_string(),
            ));
        }
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid_chars {
            return Err(Error::invalid_state(
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

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::proptest;

    // ── Valid names ──────────────────────────────────────────────────────────

    #[test]
    fn test_valid_simple() {
        assert!(SessionName::parse("test-session").is_ok());
    }

    #[test]
    fn test_valid_single_letter() {
        assert!(SessionName::parse("a").is_ok());
    }

    #[test]
    fn test_valid_with_underscore() {
        assert!(SessionName::parse("test_session").is_ok());
    }

    #[test]
    fn test_valid_with_numbers() {
        assert!(SessionName::parse("session123").is_ok());
    }

    #[test]
    fn test_valid_mixed() {
        assert!(SessionName::parse("my-session_v2").is_ok());
    }

    #[test]
    fn test_valid_at_max_length() {
        let name = "a".repeat(63);
        assert!(SessionName::parse(&name).is_ok());
    }

    #[test]
    fn test_valid_whitespace_trimmed() {
        // Leading/trailing whitespace is trimmed
        let result = SessionName::parse("  test-session  ");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid").as_str(), "test-session");
    }

    #[test]
    fn test_new_aliases_parse() {
        let via_parse = SessionName::parse("test").expect("ok");
        let via_new = SessionName::new("test").expect("ok");
        assert_eq!(via_parse, via_new);
    }

    #[test]
    fn test_from_string_bypasses_validation() {
        // From<String> impl does not validate
        let _name = SessionName::from(String::from("123-invalid-start"));
    }

    // ── Invalid names ────────────────────────────────────────────────────────

    #[test]
    fn test_reject_empty() {
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn test_reject_whitespace_only() {
        assert!(SessionName::parse("   ").is_err());
    }

    #[test]
    fn test_reject_starts_with_number() {
        assert!(SessionName::parse("123session").is_err());
    }

    #[test]
    fn test_reject_starts_with_hyphen() {
        assert!(SessionName::parse("-session").is_err());
    }

    #[test]
    fn test_reject_starts_with_underscore() {
        assert!(SessionName::parse("_session").is_err());
    }

    #[test]
    fn test_reject_exceeds_max_length() {
        let name = "a".repeat(64);
        assert!(SessionName::parse(&name).is_err());
    }

    #[test]
    fn test_reject_special_chars() {
        assert!(SessionName::parse("session.name").is_err());
        assert!(SessionName::parse("session@name").is_err());
        assert!(SessionName::parse("session name").is_err());
        assert!(SessionName::parse("session/name").is_err());
    }

    #[test]
    fn test_reject_unicode_in_name() {
        // SessionName checks is_ascii_alphanumeric, is_ascii_alphabetic
        // so non-ASCII chars (including accented) should be rejected
        // Note: is_ascii_alphabetic only returns true for a-z, A-Z
        // The name below starts with 'c' (ASCII letter) but contains non-ASCII 'é'
        // However, the validation checks chars().all() with is_ascii_alphanumeric || '-' || '_'
        // Non-ASCII chars like 'é' will fail is_ascii_alphanumeric()
        // Use a string that actually contains non-ASCII
        let name_with_nonascii = "caf\u{00e9}-session"; // café-session
        let result = SessionName::parse(name_with_nonascii);
        assert!(result.is_err(), "Expected non-ASCII chars to be rejected");
    }

    // ── as_str ───────────────────────────────────────────────────────────────

    #[test]
    fn test_as_str() {
        let name = SessionName::parse("my-session").expect("valid");
        assert_eq!(name.as_str(), "my-session");
    }

    // ── as_ref ───────────────────────────────────────────────────────────────

    #[test]
    fn test_as_ref() {
        let name = SessionName::parse("ref-test").expect("valid");
        assert_eq!(name.as_ref(), "ref-test");
    }

    // ── Display ──────────────────────────────────────────────────────────────

    #[test]
    fn test_display() {
        let name = SessionName::parse("display-test").expect("valid");
        assert_eq!(format!("{name}"), "display-test");
    }

    // ── Clone ────────────────────────────────────────────────────────────────

    #[test]
    fn test_clone() {
        let name = SessionName::parse("clone-test").expect("valid");
        let cloned = name.clone();
        assert_eq!(name, cloned);
    }

    // ── Hash ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SessionName::parse("a").expect("ok"));
        set.insert(SessionName::parse("b").expect("ok"));
        set.insert(SessionName::parse("a").expect("ok"));
        assert_eq!(set.len(), 2);
    }

    // ── PartialEq ────────────────────────────────────────────────────────────

    #[test]
    fn test_equality() {
        let a = SessionName::parse("same").expect("valid");
        let b = SessionName::parse("same").expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn test_inequality() {
        let a = SessionName::parse("name-a").expect("valid");
        let b = SessionName::parse("name-b").expect("valid");
        assert_ne!(a, b);
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_serde_roundtrip() {
        let name = SessionName::parse("serde-test").expect("valid");
        let json = serde_json::to_string(&name).expect("serialize ok");
        let deserialized: SessionName =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(name, deserialized);
    }

    #[test]
    fn test_serde_with_underscores_and_hyphens() {
        let name = SessionName::parse("my_session-name_v2").expect("valid");
        let json = serde_json::to_string(&name).expect("serialize ok");
        let deserialized: SessionName =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.as_str(), "my_session-name_v2");
    }

    // ── MAX_LENGTH ───────────────────────────────────────────────────────────

    #[test]
    fn test_max_length_constant() {
        assert_eq!(SessionName::MAX_LENGTH, 63);
    }

    // ── Error paths ──────────────────────────────────────────────────────────

    #[test]
    fn test_error_message_empty() {
        let result = SessionName::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message_starts_with_number() {
        let result = SessionName::parse("123bad");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message_too_long() {
        let result = SessionName::parse("a".repeat(100));
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(err.contains("63") || err.contains("exceed"));
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    proptest! {
        #[test]
        fn prop_valid_ascii_alphanumeric_start_and_ascii_alnum_dash_underscore(
            first in "[a-zA-Z]",
            rest in "[a-zA-Z0-9_-]{0,62}"
        ) {
            let name = format!("{first}{rest}");
            assert!(SessionName::parse(&name).is_ok(), "name: {name}");
        }

        #[test]
        fn prop_names_starting_with_digit_are_invalid(
            first in "[0-9]",
            rest in "[a-zA-Z0-9_-]{0,10}"
        ) {
            let name = format!("{first}{rest}");
            assert!(SessionName::parse(&name).is_err(), "name: {name}");
        }
    }
}
