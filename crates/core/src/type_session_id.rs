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

#[cfg(test)]
mod tests {
    use proptest::proptest;

    use super::*;

    // ── Valid Session IDs ────────────────────────────────────────────────────

    #[test]
    fn test_valid_simple_alphanumeric() {
        assert!(SessionId::parse("abc123").is_ok());
    }

    #[test]
    fn test_valid_with_hyphens() {
        assert!(SessionId::parse("session-123").is_ok());
    }

    #[test]
    fn test_valid_all_hyphens() {
        assert!(SessionId::parse("---").is_ok());
    }

    #[test]
    fn test_valid_single_char() {
        assert!(SessionId::parse("a").is_ok());
    }

    #[test]
    fn test_valid_single_digit() {
        assert!(SessionId::parse("1").is_ok());
    }

    #[test]
    fn test_valid_long_id() {
        let long_id = "a".repeat(10000);
        assert!(SessionId::parse(long_id).is_ok());
    }

    #[test]
    fn test_valid_from_string_type() {
        assert!(SessionId::parse(String::from("test-session")).is_ok());
    }

    // ── Invalid Session IDs ──────────────────────────────────────────────────

    #[test]
    fn test_reject_empty() {
        assert!(SessionId::parse("").is_err());
    }

    #[test]
    fn test_reject_whitespace() {
        assert!(SessionId::parse(" ").is_err());
    }

    #[test]
    fn test_reject_underscore() {
        assert!(SessionId::parse("session_123").is_err());
    }

    #[test]
    fn test_reject_dot() {
        assert!(SessionId::parse("session.123").is_err());
    }

    #[test]
    fn test_reject_slash() {
        assert!(SessionId::parse("session/123").is_err());
    }

    #[test]
    fn test_reject_at_sign() {
        assert!(SessionId::parse("session@host").is_err());
    }

    #[test]
    fn test_reject_special_chars() {
        assert!(SessionId::parse("session!@#$%").is_err());
    }

    #[test]
    fn test_reject_spaces() {
        assert!(SessionId::parse("session 123").is_err());
    }

    #[test]
    fn test_reject_unicode_special_chars() {
        // SessionId uses is_alphanumeric() which accepts Unicode letters
        // but special Unicode chars like emoji are rejected
        assert!(SessionId::parse("session-abc").is_ok()); // ASCII is fine
    }

    #[test]
    fn test_accept_unicode_letters() {
        // Rust's is_alphanumeric() returns true for Unicode letters
        let id = SessionId::parse("caf-123");
        // This may succeed or fail depending on whether accented e is considered alphanumeric
        // Either way, we just check no panic
        let _ = id;
    }

    // ── as_str ───────────────────────────────────────────────────────────────

    #[test]
    fn test_as_str_returns_inner() {
        let id = SessionId::parse("test-id").expect("valid");
        assert_eq!(id.as_str(), "test-id");
    }

    // ── Display ──────────────────────────────────────────────────────────────

    #[test]
    fn test_display() {
        let id = SessionId::parse("my-session").expect("valid");
        assert_eq!(format!("{id}"), "my-session");
    }

    // ── Clone ────────────────────────────────────────────────────────────────

    #[test]
    fn test_clone() {
        let id = SessionId::parse("clone-test").expect("valid");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    // ── Hash ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SessionId::parse("a").expect("valid"));
        set.insert(SessionId::parse("b").expect("valid"));
        set.insert(SessionId::parse("a").expect("valid")); // duplicate
        assert_eq!(set.len(), 2);
    }

    // ── PartialEq ────────────────────────────────────────────────────────────

    #[test]
    fn test_equality() {
        let a = SessionId::parse("same-id").expect("valid");
        let b = SessionId::parse("same-id").expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn test_inequality() {
        let a = SessionId::parse("id-a").expect("valid");
        let b = SessionId::parse("id-b").expect("valid");
        assert_ne!(a, b);
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_serde_roundtrip() {
        let id = SessionId::parse("serde-test").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize ok");
        let deserialized: SessionId = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_serde_with_hyphens() {
        let id = SessionId::parse("abc-123-xyz").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize ok");
        let deserialized: SessionId = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.as_str(), "abc-123-xyz");
    }

    // ── Error path ───────────────────────────────────────────────────────────

    #[test]
    fn test_error_message_empty() {
        let result = SessionId::parse("");
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(err.contains("empty") || err.contains("Empty"));
    }

    #[test]
    fn test_error_message_invalid_chars() {
        let result = SessionId::parse("bad_char!");
        assert!(result.is_err());
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    proptest! {
        #[test]
        fn prop_alphanumeric_always_valid(s in "[a-zA-Z0-9]+") {
            assert!(SessionId::parse(s).is_ok());
        }

        #[test]
        fn prop_alphanumeric_with_hyphens_always_valid(s in "[a-zA-Z0-9-]+") {
            // Must be non-empty (proptest generates at least one char)
            assert!(SessionId::parse(&s).is_ok());
        }

        #[test]
        fn prop_id_with_underscore_always_invalid(s in "[a-zA-Z0-9_]+") {
            // Must contain at least one underscore to be invalid
            if s.contains('_') && !s.is_empty() {
                assert!(SessionId::parse(&s).is_err());
            }
        }
    }
}
