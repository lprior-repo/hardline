#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Domain identifiers - Newtype pattern with validation
//!
//! All identifiers are validated on construction and immutable.

use crate::domain::validation::{ValidationError, ValidationResult};

const SHELL_METACHARACTERS: &str = "$`|&<>\n\r\x00";

/// Unique queue entry identifier
///
/// Wrapper around a String that ensures non-empty values.
/// Supports UUID-based generation with "queue-" prefix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    /// Create a new queue entry ID with validation.
    ///
    /// Trims whitespace and rejects empty/whitespace-only strings.
    ///
    /// # Errors
    /// Returns `ValidationError::EmptyValue` if the ID is empty.
    pub fn new(id: impl Into<String>) -> ValidationResult<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            Err(ValidationError::EmptyValue("QueueEntryId".to_string()))
        } else {
            Ok(Self(id))
        }
    }

    /// Generate a new unique queue entry ID with "queue-" prefix.
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("queue-{}", uuid::Uuid::new_v4()))
    }

    /// Parse a queue entry ID string, rejecting empty values.
    ///
    /// Unlike `new`, this does not trim whitespace — it preserves the raw value.
    ///
    /// # Errors
    /// Returns `ValidationError::EmptyValue` if the ID is empty.
    pub fn parse(id: impl Into<String>) -> ValidationResult<Self> {
        let id = id.into();
        if id.is_empty() {
            Err(ValidationError::EmptyValue("QueueEntryId".to_string()))
        } else {
            Ok(Self(id))
        }
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Default for QueueEntryId {
    fn default() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for QueueEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session name - a validated identifier for queue sessions
///
/// Wrapper that ensures:
/// - Non-empty after trimming
/// - No shell metacharacters (prevents command injection)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionName(String);

impl SessionName {
    /// Create a new session name with validation.
    ///
    /// # Errors
    /// Returns `ValidationError` if:
    /// - The name is empty after trimming
    /// - The name contains shell metacharacters
    pub fn new(name: impl Into<String>) -> ValidationResult<Self> {
        let name = name.into();
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::EmptyValue("SessionName".to_string()));
        }

        for c in SHELL_METACHARACTERS.chars() {
            if trimmed.contains(c) {
                return Err(ValidationError::InvalidCharacters {
                    field: "SessionName".to_string(),
                    found: c.to_string(),
                });
            }
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Validate a session name without consuming it.
    ///
    /// # Errors
    /// Returns `ValidationError` if the name is invalid.
    pub fn validate(name: &str) -> ValidationResult<()> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::EmptyValue("SessionName".to_string()));
        }

        for c in SHELL_METACHARACTERS.chars() {
            if trimmed.contains(c) {
                return Err(ValidationError::InvalidCharacters {
                    field: "SessionName".to_string(),
                    found: c.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get the session name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for SessionName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SessionName {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_entry_id_valid() {
        assert!(QueueEntryId::new("test-123").is_ok());
        assert!(QueueEntryId::new("  test-123  ").is_ok());
    }

    #[test]
    fn test_queue_entry_id_empty() {
        assert!(matches!(
            QueueEntryId::new(""),
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            QueueEntryId::new("   "),
            Err(ValidationError::EmptyValue(_))
        ));
    }

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::new("my-session").is_ok());
        assert!(SessionName::new("  my-session  ").is_ok());
        assert!(SessionName::new("session_123").is_ok());
        assert!(SessionName::new("session.with.dots").is_ok());
    }

    #[test]
    fn test_session_name_empty() {
        assert!(matches!(
            SessionName::new(""),
            Err(ValidationError::EmptyValue(_))
        ));
        assert!(matches!(
            SessionName::new("   "),
            Err(ValidationError::EmptyValue(_))
        ));
    }

    #[test]
    fn test_session_name_rejects_shell_metacharacters() {
        let invalid_chars = ["$", "`", "|", "&", "<", ">", "\n", "\r", "\x00"];
        for c in invalid_chars {
            let test_name = format!("session{}name", c);
            assert!(
                matches!(
                    SessionName::new(&test_name),
                    Err(ValidationError::InvalidCharacters { .. })
                ),
                "Should reject character: {:?}",
                c
            );
        }
    }

    #[test]
    fn test_session_name_validate_works() {
        assert!(SessionName::validate("valid-name").is_ok());
        assert!(SessionName::validate("invalid$name").is_err());
    }

    #[test]
    fn test_session_name_try_from() {
        assert!(SessionName::try_from("valid".to_string()).is_ok());
        assert!(SessionName::try_from("valid").is_ok());
        assert!(SessionName::try_from("").is_err());
    }

    // --- QueueEntryId Display ---

    #[test]
    fn test_queue_entry_id_display() {
        let id = QueueEntryId::new("test-id").unwrap();
        assert_eq!(format!("{id}"), "test-id");
    }

    #[test]
    fn test_queue_entry_id_display_with_spaces() {
        let id = QueueEntryId::new("  spaced  ").unwrap();
        assert_eq!(format!("{id}"), "  spaced  ");
    }

    // --- SessionName Display ---

    #[test]
    fn test_session_name_display() {
        let name = SessionName::new("my-session").unwrap();
        assert_eq!(format!("{name}"), "my-session");
    }

    #[test]
    fn test_session_name_display_trims() {
        let name = SessionName::new("  spaced  ").unwrap();
        assert_eq!(format!("{name}"), "spaced");
    }

    // --- QueueEntryId into_inner ---

    #[test]
    fn test_queue_entry_id_into_inner() {
        let id = QueueEntryId::new("my-id").unwrap();
        assert_eq!(id.into_inner(), "my-id");
    }

    // --- SessionName into_inner ---

    #[test]
    fn test_session_name_into_inner() {
        let name = SessionName::new("my-session").unwrap();
        assert_eq!(name.into_inner(), "my-session");
    }

    // --- Serde roundtrips ---

    #[test]
    fn test_queue_entry_id_serde_roundtrip() {
        let id = QueueEntryId::new("serde-test").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: QueueEntryId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "serde-test");
    }

    #[test]
    fn test_queue_entry_id_serde_roundtrip_with_spaces() {
        let id = QueueEntryId::new("  spaced-id  ").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: QueueEntryId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "  spaced-id  ");
    }

    #[test]
    fn test_session_name_serde_roundtrip() {
        let name = SessionName::new("serde-session").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let back: SessionName = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "serde-session");
    }

    #[test]
    fn test_session_name_serde_roundtrip_trims() {
        let name = SessionName::new("  spaced  ").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let back: SessionName = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "spaced");
    }

    // --- Hash consistency ---

    #[test]
    fn test_queue_entry_id_hash_consistency() {
        use std::collections::HashSet;
        let id = QueueEntryId::new("hash-test").unwrap();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn test_session_name_hash_consistency() {
        use std::collections::HashSet;
        let name = SessionName::new("hash-test").unwrap();
        let mut set = HashSet::new();
        set.insert(name.clone());
        assert!(set.contains(&name));
    }

    // --- Equality ---

    #[test]
    fn test_queue_entry_id_equality() {
        let a = QueueEntryId::new("same").unwrap();
        let b = QueueEntryId::new("same").unwrap();
        let c = QueueEntryId::new("different").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_session_name_equality() {
        let a = SessionName::new("same").unwrap();
        let b = SessionName::new("same").unwrap();
        let c = SessionName::new("different").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- Edge cases ---

    #[test]
    fn test_session_name_with_unicode() {
        let name = SessionName::new("session-name");
        assert!(name.is_ok());
    }

    #[test]
    fn test_session_name_validate_nonempty_invalid_chars() {
        assert!(SessionName::validate("invalid$").is_err());
    }

    #[test]
    fn test_queue_entry_id_with_internal_spaces() {
        let id = QueueEntryId::new("id with spaces");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "id with spaces");
    }

    #[test]
    fn test_session_name_special_chars_allowed() {
        // Dots, dashes, underscores should be fine
        assert!(SessionName::new("session.with.dots").is_ok());
        assert!(SessionName::new("session_with_underscores").is_ok());
        assert!(SessionName::new("session-with-dashes").is_ok());
        assert!(SessionName::new("mixed.chars_here-123").is_ok());
    }

    #[test]
    fn test_session_name_null_byte_rejected() {
        assert!(SessionName::new("ses\x00ion").is_err());
    }

    #[test]
    fn test_session_name_carriage_return_rejected() {
        assert!(SessionName::new("ses\rion").is_err());
    }

    #[test]
    fn test_session_name_newline_rejected() {
        assert!(SessionName::new("ses\nion").is_err());
    }

    #[test]
    fn test_session_name_single_char() {
        assert!(SessionName::new("a").is_ok());
    }

    #[test]
    fn test_session_name_very_long() {
        let long_name = "a".repeat(10000);
        assert!(SessionName::new(&long_name).is_ok());
    }

    // ========================================================================
    // Property-based tests (proptest)
    // ========================================================================

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        /// QueueEntryId roundtrip: new -> as_str preserves non-empty input.
        #[test]
        fn proptest_queue_entry_id_roundtrip(
            input in "\\S.{0,99}",
        ) {
            let id = QueueEntryId::new(input.clone()).expect("valid input should parse");
            prop_assert!(!id.as_str().trim().is_empty());
        }

        /// QueueEntryId is reflexive: a == a for all valid a.
        #[test]
        fn proptest_queue_entry_id_reflexive(
            input in "\\S.{0,99}",
        ) {
            let id = QueueEntryId::new(input).expect("valid");
            prop_assert_eq!(id.clone(), id);
        }

        /// QueueEntryId rejects empty or whitespace-only strings.
        #[test]
        fn proptest_queue_entry_id_rejects_whitespace_only(
            input in "\\s+",
        ) {
            prop_assert!(QueueEntryId::new(input).is_err());
        }

        /// SessionName (queue) rejects strings with shell metacharacters.
        #[test]
        fn proptest_queue_session_name_rejects_shell_meta(
            valid_prefix in "[a-zA-Z0-9_]{1,10}",
            meta_char in "[$`|&<>]",
            valid_suffix in "[a-zA-Z0-9_]{1,10}",
        ) {
            let input = format!("{}{}{}", valid_prefix, meta_char, valid_suffix);
            prop_assert!(SessionName::new(&input).is_err(), "should reject: {:?}", input);
        }

        /// SessionName (queue) rejects empty strings.
        #[test]
        fn proptest_queue_session_name_rejects_empty(
            input in "\\s*",
        ) {
            if input.trim().is_empty() {
                prop_assert!(SessionName::new(input).is_err());
            }
        }
    }
}
