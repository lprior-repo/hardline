//! Proptest invariants for SessionId and SessionName
//!
//! 10 invariants:
//! 1. SessionId roundtrip: any non-empty ASCII string parses and roundtrips
//! 2. SessionId rejects all non-ASCII inputs
//! 3. SessionId rejects empty string
//! 4. SessionId Display matches as_str
//! 5. SessionId equality: same input => equal; different => not equal
//! 6. SessionName roundtrip: valid name chars parse and roundtrip
//! 7. SessionName rejects names not starting with a letter
//! 8. SessionName length boundary: ≤63 OK, >63 fails
//! 9. SessionName rejects invalid characters
//! 10. SessionName Display matches as_str

use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

use crate::domain::identifiers::error::IdentifierError;
use crate::domain::identifiers::{SessionId, SessionName};

// ═══════════════════════════════════════════════════════════════════════════
// Strategies
// ═══════════════════════════════════════════════════════════════════════════

/// Generate any non-empty ASCII string (valid for SessionId)
fn valid_session_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9]{1,64}",
        "[a-zA-Z0-9_-]{1,64}",
        "[a-zA-Z0-9 _.-]{1,64}",
        "[a-zA-Z][a-zA-Z0-9_-]{0,100}",
    ]
}

/// Generate a valid session name: starts with letter, alphanumeric/hyphen/underscore, 1-63 chars
fn valid_session_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,62}"
}

/// Generate invalid session names (wrong start char)
fn invalid_start_session_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9][a-zA-Z0-9_-]{0,10}",
        Just("-bad".to_string()),
        Just("_bad".to_string()),
        Just(".bad".to_string()),
    ]
}

/// Generate strings with invalid characters for session names.
/// Note: spaces are NOT included because SessionName::parse trims whitespace
/// before validation, so trailing/leading spaces don't cause rejection.
/// Whitespace handling is tested in proptest_session_name_trims_whitespace.
fn invalid_chars_session_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}\\.[a-zA-Z0-9_-]{1,5}",
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}:[a-zA-Z0-9_-]{1,5}",
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}\\$[a-zA-Z0-9_-]{1,5}",
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}@[a-zA-Z0-9_-]{1,5}",
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}\\#[a-zA-Z0-9_-]{1,5}",
        "[a-zA-Z][a-zA-Z0-9_-]{1,5}\\![a-zA-Z0-9_-]{1,5}",
    ]
}

/// Generate non-ASCII strings (invalid for SessionId)
fn non_ascii_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("café".to_string()),
        Just("日本語".to_string()),
        Just("session-abc\u{301}".to_string()),
        Just("session\u{00E9}".to_string()),
        Just("\u{1F600}".to_string()),
        Just("prefix-café-suffix".to_string()),
        Just("hello-世界".to_string()),
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionId Proptests
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Invariant 1: Any non-empty ASCII string parses and roundtrips
    #[test]
    fn proptest_session_id_roundtrip(s in valid_session_id_strategy()) {
        let id = SessionId::parse(&s);
        prop_assert!(id.is_ok(), "Valid ASCII string '{s}' should parse");
        let parsed = id.expect("already checked");
        prop_assert_eq!(parsed.as_str(), s);
        // Double roundtrip: as_str() -> parse() again
        let raw = parsed.as_str();
        let again = SessionId::parse(raw).expect("roundtrip");
        prop_assert_eq!(again.as_str(), raw);
    }

    /// Invariant 2: All non-ASCII inputs are rejected
    #[test]
    fn proptest_session_id_rejects_non_ascii(s in non_ascii_strategy()) {
        let result = SessionId::parse(&s);
        prop_assert!(result.is_err(), "Non-ASCII string should be rejected");
        prop_assert!(
            matches!(result, Err(IdentifierError::NotAscii { .. })),
            "Expected NotAscii error, got: {:?}",
            result
        );
    }

    /// Invariant 3: Empty string always fails
    #[test]
    fn proptest_session_id_rejects_empty(_s in "\\PC{0,0}") {
        let result = SessionId::parse("");
        prop_assert!(result.is_err());
        prop_assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    /// Invariant 4: Display output matches as_str
    #[test]
    fn proptest_session_id_display_matches_as_str(s in valid_session_id_strategy()) {
        let id = SessionId::parse(&s).expect("valid");
        let display = format!("{id}");
        let as_str = id.as_str();
        prop_assert!(display == as_str);
        prop_assert!(display == s);
    }

    /// Invariant 5: Equality is consistent
    #[test]
    fn proptest_session_id_equality(a in valid_session_id_strategy(), b in valid_session_id_strategy()) {
        let id_a = SessionId::parse(&a).expect("valid");
        let id_b = SessionId::parse(&b).expect("valid");
        let id_a2 = SessionId::parse(&a).expect("valid");

        // Reflexive
        prop_assert!(id_a == id_a, "reflexive");
        prop_assert!(id_b == id_b, "reflexive");

        // Symmetric
        prop_assert_eq!(id_a == id_b, id_b == id_a);

        // Same string => equal
        prop_assert!(id_a == id_a2, "same input => equal");

        // Hash consistency: equal values => equal hashes
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let hash_val = |v: &SessionId| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        prop_assert_eq!(hash_val(&id_a), hash_val(&id_a2));
    }

    /// SessionId TryFrom<String> matches parse
    #[test]
    fn proptest_session_id_try_from_consistency(s in valid_session_id_strategy()) {
        let via_parse = SessionId::parse(&s);
        let via_try_from = SessionId::try_from(s);
        prop_assert_eq!(via_parse.is_ok(), via_try_from.is_ok());
        if let (Ok(ref p), Ok(ref t)) = (via_parse, via_try_from) {
            prop_assert_eq!(p.as_str(), t.as_str());
        }
    }

    /// SessionId serde roundtrip
    #[test]
    fn proptest_session_id_serde_roundtrip(s in valid_session_id_strategy()) {
        let id = SessionId::parse(&s).expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: SessionId = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(id.as_str(), deserialized.as_str());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionName Proptests
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Invariant 6: Valid session names parse and roundtrip
    #[test]
    fn proptest_session_name_roundtrip(s in valid_session_name_strategy()) {
        let name = SessionName::parse(&s);
        prop_assert!(name.is_ok(), "Valid name '{s}' should parse, got {:?}", name);
        let parsed = name.expect("already checked");
        prop_assert_eq!(parsed.as_str(), s);
        // Double roundtrip
        let raw = parsed.as_str();
        let again = SessionName::parse(raw).expect("roundtrip");
        prop_assert_eq!(again.as_str(), raw);
    }

    /// Invariant 7: Names not starting with a letter are rejected
    #[test]
    fn proptest_session_name_rejects_invalid_start(s in invalid_start_session_name_strategy()) {
        let result = SessionName::parse(&s);
        prop_assert!(
            result.is_err(),
            "Name '{s}' should be rejected (invalid start char)"
        );
    }

    /// Invariant 8: Length boundary — ≤63 chars OK, >63 fails
    #[test]
    fn proptest_session_name_length_boundary(
        n in 1usize..=63usize,
        extra in 1usize..=10usize
    ) {
        // Exactly n chars (1-63) should be valid
        let valid_name = "a".repeat(n);
        let result = SessionName::parse(&valid_name);
        prop_assert!(result.is_ok(), "Name of length {n} should be valid");

        // 63 + extra chars should be invalid
        let too_long = "a".repeat(63 + extra);
        let result = SessionName::parse(&too_long);
        prop_assert!(result.is_err(), "Name of length {} should be rejected", 63 + extra);
        prop_assert!(
            matches!(result, Err(IdentifierError::TooLong { max: 63, .. })),
            "Expected TooLong error, got: {:?}",
            result
        );
    }

    /// Invariant 9: Invalid characters are always rejected
    #[test]
    fn proptest_session_name_rejects_invalid_chars(s in invalid_chars_session_name_strategy()) {
        let result = SessionName::parse(&s);
        prop_assert!(
            result.is_err(),
            "Name '{s}' should be rejected (invalid chars)"
        );
    }

    /// Invariant 10: Display output matches as_str
    #[test]
    fn proptest_session_name_display_matches_as_str(s in valid_session_name_strategy()) {
        let name = SessionName::parse(&s).expect("valid");
        let display = format!("{name}");
        let as_str = name.as_str();
        prop_assert!(display == as_str);
        prop_assert!(display == s);
    }

    /// SessionName equality consistency
    #[test]
    fn proptest_session_name_equality(
        a in valid_session_name_strategy(),
        b in valid_session_name_strategy()
    ) {
        let name_a = SessionName::parse(&a).expect("valid");
        let name_b = SessionName::parse(&b).expect("valid");
        let name_a2 = SessionName::parse(&a).expect("valid");

        // Reflexive
        prop_assert!(name_a == name_a, "reflexive");

        // Symmetric
        prop_assert_eq!(name_a == name_b, name_b == name_a);

        // Hash consistency
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let hash_val = |v: &SessionName| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        prop_assert_eq!(hash_val(&name_a), hash_val(&name_a2));

        // Same string => same result
        prop_assert!(name_a == name_a2, "same input => equal");
    }

    /// SessionName TryFrom<String> matches parse
    #[test]
    fn proptest_session_name_try_from_consistency(s in valid_session_name_strategy()) {
        let via_parse = SessionName::parse(&s);
        let via_try_from = SessionName::try_from(s);
        prop_assert_eq!(via_parse.is_ok(), via_try_from.is_ok());
        if let (Ok(ref p), Ok(ref t)) = (via_parse, via_try_from) {
            prop_assert_eq!(p.as_str(), t.as_str());
        }
    }

    /// SessionName serde roundtrip
    #[test]
    fn proptest_session_name_serde_roundtrip(s in valid_session_name_strategy()) {
        let name = SessionName::parse(&s).expect("valid");
        let json = serde_json::to_string(&name).expect("serialize");
        let deserialized: SessionName = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(name.as_str(), deserialized.as_str());
    }

    /// SessionName FromStr trait consistency
    #[test]
    fn proptest_session_name_from_str_trait(s in valid_session_name_strategy()) {
        use std::str::FromStr;
        let via_parse = SessionName::parse(&s);
        let via_from_str = SessionName::from_str(&s);
        prop_assert_eq!(via_parse.is_ok(), via_from_str.is_ok());
        if let (Ok(ref p), Ok(ref f)) = (via_parse, via_from_str) {
            prop_assert_eq!(p.as_str(), f.as_str());
        }
    }

    /// SessionName whitespace trimming
    #[test]
    fn proptest_session_name_trims_whitespace(
        s in valid_session_name_strategy(),
        pad in 0usize..=5usize
    ) {
        let padding = " ".repeat(pad);
        let padded = format!("{padding}{s}{padding}");
        let result = SessionName::parse(&padded);
        prop_assert!(result.is_ok(), "Padded valid name should parse");
        let parsed = result.expect("ok");
        // After trimming, should match original trimmed input
        let trimmed = padded.trim();
        prop_assert_eq!(parsed.as_str(), trimmed);
    }

    /// SessionName empty and whitespace-only always fail
    #[test]
    fn proptest_session_name_rejects_whitespace_only(ws in "[ \t\n]+") {
        let result = SessionName::parse(&ws);
        prop_assert!(result.is_err(), "Whitespace-only string should be rejected");
        prop_assert!(
            matches!(result, Err(IdentifierError::Empty)),
            "Expected Empty error, got: {:?}",
            result
        );
    }
}
