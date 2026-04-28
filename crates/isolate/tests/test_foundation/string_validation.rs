#![allow(clippy::uninlined_format_args)]
//! Property-Based Tests for String Validation
//!
//! This module demonstrates proptest usage with deterministic configuration
//! for validating session names and other string inputs.
//!
//! ## Properties Tested
//!
//! - Valid session names are alphanumeric with hyphens and underscores
//! - Empty strings are always rejected
//! - Strings exceeding max length are rejected
//! - Round-trip encoding/decoding preserves content

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;

use crate::test_foundation::proptest_config::deterministic_config;

/// Maximum length for session names
const MAX_SESSION_NAME_LENGTH: usize = 64;

/// Validate a session name according to Isolate rules.
///
/// # Rules
///
/// - Must not be empty
/// - Must not exceed 64 characters
/// - Must contain only alphanumeric characters, hyphens, and underscores
///
/// # Errors
///
/// Returns a string describing the validation error.
pub fn validate_session_name(name: &str) -> Result<&str, String> {
    if name.is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    if name.len() > MAX_SESSION_NAME_LENGTH {
        return Err(format!(
            "session name exceeds maximum length of {} characters",
            MAX_SESSION_NAME_LENGTH
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "session name must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        );
    }

    Ok(name)
}

/// Strategy for generating valid session names.
///
/// Generates strings of 1-64 characters from the valid character set.
pub fn valid_session_name_strategy() -> impl Strategy<Value = String> {
    // Generate alphanumeric strings with optional hyphens and underscores
    (1..=MAX_SESSION_NAME_LENGTH).prop_flat_map(|len| {
        proptest::collection::vec(
            prop_oneof![
                // lowercase letter
                (0_u8..26).prop_map(|c| (b'a' + c) as char),
                // uppercase letter
                (0_u8..26).prop_map(|c| (b'A' + c) as char),
                // digit
                (0_u8..10).prop_map(|c| (b'0' + c) as char),
                // hyphen
                Just('-'),
                // underscore
                Just('_'),
            ],
            len..=len,
        )
        .prop_map(|chars| chars.into_iter().collect())
    })
}

/// Strategy for generating invalid session names.
///
/// Generates strings that violate one or more validation rules.
pub fn invalid_session_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just(String::new()),
        // Too long
        (MAX_SESSION_NAME_LENGTH + 1..=MAX_SESSION_NAME_LENGTH + 100)
            .prop_map(|len| { "a".repeat(len) }),
        // Contains spaces
        (1..64usize).prop_map(|len| format!("session{}", " ".repeat(len))),
        // Contains special characters
        (1..64usize).prop_map(|len| format!("session{}name", "@".repeat(len.min(1)))),
    ]
}

/// Strategy for generating any string (for boundary testing).
pub fn any_string_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9\\-_ ]{0,100}"
}

proptest! {
    #![proptest_config(deterministic_config())]

    /// Property: Valid session names should always pass validation
    #[test]
    fn prop_valid_session_names_pass(name in valid_session_name_strategy()) {
        let result = validate_session_name(&name);
        prop_assert!(result.is_ok(), "Valid name '{}' should pass validation", name);
    }

    /// Property: Invalid session names should always fail validation
    #[test]
    fn prop_invalid_session_names_fail(name in invalid_session_name_strategy()) {
        let result = validate_session_name(&name);
        prop_assert!(result.is_err(), "Invalid name '{}' should fail validation", name);
    }

    /// Property: Empty strings are always rejected
    #[test]
    fn prop_empty_strings_rejected(name in "") {
        let result = validate_session_name(&name);
        prop_assert!(result.is_err(), "Empty string should be rejected");
    }

    /// Property: Strings exceeding max length are always rejected
    #[test]
    fn prop_long_strings_rejected(name in "[a]{65,100}") {
        let result = validate_session_name(&name);
        prop_assert!(result.is_err(), "Long string should be rejected: length={}", name.len());
    }

    /// Property: Round-trip - trimming and re-validating valid names should still work
    #[test]
    fn prop_roundtrip_validation(name in valid_session_name_strategy()) {
        let trimmed = name.trim();
        // Our valid names shouldn't have leading/trailing whitespace
        let result = validate_session_name(trimmed);
        prop_assert!(result.is_ok(), "Trimmed valid name '{}' should still pass", trimmed);
    }

    /// Property: Strings with only valid chars up to max length should pass
    #[test]
    fn prop_valid_chars_pass(s in "[a-zA-Z0-9\\-_]{1,64}") {
        let result = validate_session_name(&s);
        prop_assert!(result.is_ok(), "String with valid chars '{}' should pass", s);
    }

    /// Property: Validation is deterministic - same input always gives same result
    #[test]
    fn prop_validation_deterministic(name in any_string_strategy()) {
        let result1 = validate_session_name(&name);
        let result2 = validate_session_name(&name);
        prop_assert_eq!(result1.is_ok(), result2.is_ok());
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_valid_simple_name() {
        let result = validate_session_name("my-session");
        assert!(result.is_ok());
        assert_eq!(result.expect("valid name"), "my-session");
    }

    #[test]
    fn test_valid_name_with_underscores() {
        let result = validate_session_name("my_session_123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_special_chars() {
        let result = validate_session_name("session@name");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_spaces() {
        let result = validate_session_name("session name");
        assert!(result.is_err());
    }

    #[test]
    fn test_exactly_max_length() {
        let name = "a".repeat(64);
        let result = validate_session_name(&name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_one_over_max_length() {
        let name = "a".repeat(65);
        let result = validate_session_name(&name);
        assert!(result.is_err());
    }
}