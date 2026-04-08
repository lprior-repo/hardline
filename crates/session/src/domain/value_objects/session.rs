use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    #[error("identifier cannot be empty")]
    Empty,
    #[error("identifier too long: {0} characters (max {1})")]
    TooLong(usize, usize),
    #[error("identifier contains invalid characters: {0}")]
    InvalidCharacters(String),
    #[error("identifier must start with a letter")]
    InvalidStart,
    #[error("identifier must be ASCII only")]
    NotAscii,
}

pub type SessionNameError = IdentifierError;
pub type WorkspaceIdError = IdentifierError;
pub type BeadIdError = IdentifierError;

fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if s.len() > 63 {
        return Err(IdentifierError::TooLong(s.len(), 63));
    }
    if !s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(IdentifierError::InvalidStart);
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IdentifierError::InvalidCharacters(
            "must contain only letters, numbers, hyphens, or underscores".into(),
        ));
    }
    Ok(())
}

fn validate_hex_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if !s.starts_with("bd-") {
        return Err(IdentifierError::InvalidCharacters(
            "must start with 'bd-'".into(),
        ));
    }
    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentifierError::InvalidCharacters(
            "must be valid hex after 'bd-'".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    pub const MAX_LENGTH: usize = 63;

    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        if s.is_empty() {
            return Err(IdentifierError::Empty);
        }
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        Self(format!("ws-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_hex_id(&s)?;
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        let hex = format!("{:x}", uuid::Uuid::new_v4());
        Self(format!("bd-{}", &hex[..12]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::prop_assert;

    proptest! {
        fn prop_session_value_object_name_adversarial(s in ".*") {
            let res = SessionName::parse(s.clone());
            let trimmed = s.trim();

            if let Ok(name) = res {
                let name_str = name.as_str();
                prop_assert!(!name_str.is_empty(), "Empty string allowed");
                prop_assert!(name_str.len() <= SessionName::MAX_LENGTH, "Max length exceeded: {} > {}", name_str.len(), SessionName::MAX_LENGTH);

                let first_char = name_str.chars().next().unwrap();
                prop_assert!(first_char.is_ascii_alphabetic(), "First char not ascii alphabetic: {}", first_char);

                let valid_chars = name_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                prop_assert!(valid_chars, "Contains invalid chars: {}", name_str);
            } else {
                // If it failed, it must violate one of the rules.
                let violates_rules = trimmed.is_empty()
                    || trimmed.len() > SessionName::MAX_LENGTH
                    || !trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                    || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

                prop_assert!(violates_rules, "Valid string was rejected: {:?}", s);
            }
        }
    }

    #[test]
    fn test_valid_session_name() {
        assert!(SessionName::parse("my-session").is_ok());
        assert!(SessionName::parse("test_123").is_ok());
    }

    #[test]
    fn test_invalid_session_name_empty() {
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn test_valid_bead_id() {
        assert!(BeadId::parse("bd-abc123").is_ok());
    }

    #[test]
    fn test_invalid_bead_id_no_prefix() {
        assert!(BeadId::parse("abc123").is_err());
    }

    // =========================================================================
    // SessionName Extended Tests
    // =========================================================================

    mod session_name_extended_tests {
        use super::*;

        #[test]
        fn session_name_at_max_length() {
            let max_name = "a".repeat(SessionName::MAX_LENGTH);
            let name = SessionName::parse(&max_name).expect("valid at max");
            assert_eq!(name.as_str().len(), SessionName::MAX_LENGTH);
        }

        #[test]
        fn session_name_exceeds_max_length_rejects() {
            let too_long = "a".repeat(SessionName::MAX_LENGTH + 1);
            let result = SessionName::parse(&too_long);
            assert!(result.is_err());
        }

        #[test]
        fn session_name_trims_whitespace() {
            let name = SessionName::parse("  padded  ").expect("valid");
            assert_eq!(name.as_str(), "padded");
        }

        #[test]
        fn session_name_whitespace_only_rejects() {
            let result = SessionName::parse("   ");
            assert!(result.is_err());
        }

        #[test]
        fn session_name_starts_with_number_rejects() {
            let result = SessionName::parse("123invalid");
            assert!(result.is_err());
        }

        #[test]
        fn session_name_with_hyphens_and_underscores() {
            assert!(SessionName::parse("my-session_name").is_ok());
            assert!(SessionName::parse("a_b-c_d").is_ok());
        }

        #[test]
        fn session_name_with_space_rejects() {
            assert!(SessionName::parse("my session").is_err());
        }

        #[test]
        fn session_name_display() {
            let name = SessionName::parse("display-test").expect("valid");
            assert_eq!(format!("{name}"), "display-test");
        }

        #[test]
        fn session_name_try_from_string() {
            let name = SessionName::try_from("valid-name".to_string()).expect("valid");
            assert_eq!(name.as_str(), "valid-name");
        }

        #[test]
        fn session_name_equality() {
            let n1 = SessionName::parse("same").expect("valid");
            let n2 = SessionName::parse("same").expect("valid");
            let n3 = SessionName::parse("different").expect("valid");
            assert_eq!(n1, n2);
            assert_ne!(n1, n3);
        }

        #[test]
        fn session_name_serde_roundtrip() {
            let name = SessionName::parse("serde-test").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            let parsed: SessionName = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(name, parsed);
        }

        #[test]
        fn session_name_serde_json_output() {
            let name = SessionName::parse("hello").expect("valid");
            let json = serde_json::to_string(&name).expect("serialize");
            assert_eq!(json, "\"hello\"");
        }

        #[test]
        fn session_name_hash_consistency() {
            use std::collections::HashSet;
            let n1 = SessionName::parse("hash").expect("valid");
            let n2 = SessionName::parse("hash").expect("valid");
            let mut set = HashSet::new();
            set.insert(n1);
            assert!(set.contains(&n2));
        }
    }

    // =========================================================================
    // WorkspaceId Extended Tests
    // =========================================================================

    mod workspace_id_extended_tests {
        use super::*;

        #[test]
        fn workspace_id_empty_rejects() {
            let result = WorkspaceId::parse("");
            assert!(result.is_err());
        }

        #[test]
        fn workspace_id_any_string_valid() {
            assert!(WorkspaceId::parse("anything goes!").is_ok());
            assert!(WorkspaceId::parse("spaces allowed").is_ok());
        }

        #[test]
        fn workspace_id_generate_has_prefix() {
            let id = WorkspaceId::generate();
            assert!(id.as_str().starts_with("ws-"));
        }

        #[test]
        fn workspace_id_generate_is_unique() {
            let id1 = WorkspaceId::generate();
            let id2 = WorkspaceId::generate();
            assert_ne!(id1, id2);
        }

        #[test]
        fn workspace_id_display() {
            let id = WorkspaceId::parse("ws-test").expect("valid");
            assert_eq!(format!("{id}"), "ws-test");
        }

        #[test]
        fn workspace_id_serde_roundtrip() {
            let id = WorkspaceId::parse("ws-abc").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: WorkspaceId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }
    }

    // =========================================================================
    // BeadId Extended Tests
    // =========================================================================

    mod bead_id_extended_tests {
        use super::*;

        #[test]
        fn bead_id_empty_rejects() {
            assert!(BeadId::parse("").is_err());
        }

        #[test]
        fn bead_id_no_prefix_rejects() {
            assert!(BeadId::parse("abc123").is_err());
        }

        #[test]
        fn bead_id_empty_suffix_rejects() {
            assert!(BeadId::parse("bd-").is_err());
        }

        #[test]
        fn bead_id_invalid_hex_rejects() {
            assert!(BeadId::parse("bd-xyz").is_err());
        }

        #[test]
        fn bead_id_uppercase_hex_valid() {
            assert!(BeadId::parse("bd-ABCDEF").is_ok());
        }

        #[test]
        fn bead_id_mixed_case_hex_valid() {
            assert!(BeadId::parse("bd-AbCdEf123456").is_ok());
        }

        #[test]
        fn bead_id_generate_has_prefix() {
            let id = BeadId::generate();
            assert!(id.as_str().starts_with("bd-"));
        }

        #[test]
        fn bead_id_generate_has_non_empty_suffix() {
            let id = BeadId::generate();
            let suffix = &id.as_str()[3..];
            assert!(!suffix.is_empty());
            assert!(suffix.len() >= 8);
        }

        #[test]
        fn bead_id_generate_is_unique() {
            let id1 = BeadId::generate();
            let id2 = BeadId::generate();
            assert_ne!(id1, id2);
        }

        #[test]
        fn bead_id_display() {
            let id = BeadId::parse("bd-cafe").expect("valid");
            assert_eq!(format!("{id}"), "bd-cafe");
        }

        #[test]
        fn bead_id_serde_roundtrip() {
            let id = BeadId::parse("bd-deadbeef").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: BeadId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn bead_id_serde_json_output() {
            let id = BeadId::parse("bd-1").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            assert_eq!(json, "\"bd-1\"");
        }

        #[test]
        fn bead_id_equality() {
            let id1 = BeadId::parse("bd-deadbeef").expect("valid");
            let id2 = BeadId::parse("bd-deadbeef").expect("valid");
            let id3 = BeadId::parse("bd-cafebabe").expect("valid");
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }
    }

    // =========================================================================
    // IdentifierError Tests
    // =========================================================================

    mod identifier_error_tests {
        use super::*;

        #[test]
        fn identifier_error_empty_display() {
            let err = IdentifierError::Empty;
            assert_eq!(format!("{err}"), "identifier cannot be empty");
        }

        #[test]
        fn identifier_error_too_long_display() {
            let err = IdentifierError::TooLong(100, 63);
            assert!(format!("{err}").contains("100"));
            assert!(format!("{err}").contains("63"));
        }

        #[test]
        fn identifier_error_invalid_start_display() {
            let err = IdentifierError::InvalidStart;
            assert!(format!("{err}").contains("letter"));
        }

        #[test]
        fn identifier_error_not_ascii_display() {
            let err = IdentifierError::NotAscii;
            assert!(format!("{err}").contains("ASCII"));
        }

        #[test]
        fn identifier_error_equality() {
            assert_eq!(IdentifierError::Empty, IdentifierError::Empty);
            assert_ne!(IdentifierError::Empty, IdentifierError::InvalidStart);
        }

        #[test]
        fn identifier_error_clone() {
            let err = IdentifierError::InvalidCharacters("test".into());
            assert_eq!(err.clone(), err);
        }
    }

    // =========================================================================
    // SessionName Proptests (disabled - using proptest in cfg(test) causes macro parsing issues)
    // =========================================================================

    mod session_name_proptests {
        use super::*;

        #[test]
        fn session_name_various_valid_names() {
            for name in &["a", "ab", "a_b-c_d", "test123"] {
                assert!(SessionName::parse(*name).is_ok());
            }
        }

        #[test]
        fn session_name_whitespace_rejected() {
            assert!(SessionName::parse("   ").is_err());
        }

        #[test]
        fn session_name_serde_roundtrip_many() {
            for name in &["a", "ab", "test-session", "x_y_z"] {
                let parsed = SessionName::parse(*name).unwrap();
                let json = serde_json::to_string(&parsed).unwrap();
                let back: SessionName = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, back);
            }
        }
    }

    // =========================================================================
    // SessionName Validation Edge Cases (ha-vms)
    // =========================================================================

    mod session_name_validation_edge_cases {
        use super::*;

        // --- Special characters rejected ---

        #[test]
        fn session_name_rejects_dot() {
            let err = SessionName::parse("name.with.dots").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_slash() {
            let err = SessionName::parse("path/name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_backslash() {
            let err = SessionName::parse("path\\name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_at_sign() {
            let err = SessionName::parse("user@domain").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_exclamation() {
            let err = SessionName::parse("name!").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_hash() {
            let err = SessionName::parse("name#1").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_dollar() {
            let err = SessionName::parse("$name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidStart));
        }

        #[test]
        fn session_name_rejects_percent() {
            let err = SessionName::parse("name%20").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_ampersand() {
            let err = SessionName::parse("a&b").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_asterisk() {
            let err = SessionName::parse("wild*card").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_equals() {
            let err = SessionName::parse("key=value").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_plus() {
            let err = SessionName::parse("a+b").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidCharacters(_)));
        }

        #[test]
        fn session_name_rejects_brackets() {
            assert!(SessionName::parse("name[0]").is_err());
            assert!(SessionName::parse("name{0}").is_err());
        }

        #[test]
        fn session_name_rejects_parens() {
            assert!(SessionName::parse("name(1)").is_err());
        }

        #[test]
        fn session_name_rejects_pipe() {
            assert!(SessionName::parse("a|b").is_err());
        }

        #[test]
        fn session_name_rejects_semicolon() {
            assert!(SessionName::parse("a;b").is_err());
        }

        #[test]
        fn session_name_rejects_colon() {
            assert!(SessionName::parse("a:b").is_err());
        }

        #[test]
        fn session_name_rejects_single_quote() {
            assert!(SessionName::parse("it's").is_err());
        }

        #[test]
        fn session_name_rejects_double_quote() {
            assert!(SessionName::parse("a\"b").is_err());
        }

        #[test]
        fn session_name_rejects_less_greater() {
            assert!(SessionName::parse("a<b").is_err());
            assert!(SessionName::parse("a>b").is_err());
        }

        #[test]
        fn session_name_rejects_question_mark() {
            assert!(SessionName::parse("what?").is_err());
        }

        #[test]
        fn session_name_rejects_comma() {
            assert!(SessionName::parse("a,b").is_err());
        }

        #[test]
        fn session_name_rejects_tilde() {
            assert!(SessionName::parse("~name").unwrap_err() == IdentifierError::InvalidStart);
        }

        #[test]
        fn session_name_rejects_backtick() {
            assert!(SessionName::parse("a`b").is_err());
        }

        // --- Unicode / non-ASCII ---

        #[test]
        fn session_name_rejects_accented_chars() {
            assert!(SessionName::parse("café").is_err());
            assert!(SessionName::parse("naïve").is_err());
        }

        #[test]
        fn session_name_rejects_cjk() {
            assert!(SessionName::parse("日本語").is_err());
        }

        #[test]
        fn session_name_rejects_emoji() {
            assert!(SessionName::parse("test🎉").is_err());
        }

        #[test]
        fn session_name_rejects_non_breaking_space() {
            let name = "a\u{00A0}b"; // non-breaking space
            assert!(SessionName::parse(name).is_err());
        }

        // --- Control characters ---

        #[test]
        fn session_name_rejects_embedded_tab() {
            assert!(SessionName::parse("name\tvalue").is_err());
        }

        #[test]
        fn session_name_rejects_embedded_newline() {
            assert!(SessionName::parse("name\nvalue").is_err());
        }

        #[test]
        fn session_name_rejects_embedded_carriage_return() {
            assert!(SessionName::parse("name\rvalue").is_err());
        }

        #[test]
        fn session_name_rejects_null_byte() {
            assert!(SessionName::parse("name\0value").is_err());
        }

        // --- Start character edge cases ---

        #[test]
        fn session_name_start_with_hyphen_rejects() {
            let err = SessionName::parse("-name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidStart));
        }

        #[test]
        fn session_name_start_with_underscore_rejects() {
            let err = SessionName::parse("_name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidStart));
        }

        #[test]
        fn session_name_start_with_digit_rejects() {
            let err = SessionName::parse("1name").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidStart));
        }

        #[test]
        fn session_name_start_with_zero_rejects() {
            let err = SessionName::parse("0abc").unwrap_err();
            assert!(matches!(err, IdentifierError::InvalidStart));
        }

        // --- Boundary length tests with error specificity ---

        #[test]
        fn session_name_exactly_max_length_returns_ok() {
            let name = "a".repeat(63);
            let result = SessionName::parse(&name);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().as_str().len(), 63);
        }

        #[test]
        fn session_name_one_over_max_returns_too_long() {
            let name = "a".repeat(64);
            let err = SessionName::parse(&name).unwrap_err();
            assert!(matches!(err, IdentifierError::TooLong(64, 63)));
        }

        #[test]
        fn session_name_one_below_max_is_valid() {
            let name = "a".repeat(62);
            assert!(SessionName::parse(&name).is_ok());
        }

        #[test]
        fn session_name_single_char_is_valid() {
            let name = SessionName::parse("a").expect("single letter is valid");
            assert_eq!(name.as_str(), "a");
        }

        #[test]
        fn session_name_very_long_rejected() {
            let name = "a".repeat(1000);
            let err = SessionName::parse(&name).unwrap_err();
            assert!(matches!(err, IdentifierError::TooLong(1000, 63)));
        }

        // --- Whitespace handling specifics ---

        #[test]
        fn session_name_trailing_whitespace_trimmed() {
            let name = SessionName::parse("test  ").expect("valid after trim");
            assert_eq!(name.as_str(), "test");
        }

        #[test]
        fn session_name_leading_whitespace_trimmed() {
            let name = SessionName::parse("  test").expect("valid after trim");
            assert_eq!(name.as_str(), "test");
        }

        #[test]
        fn session_name_tab_only_rejected() {
            assert!(SessionName::parse("\t").is_err());
        }

        #[test]
        fn session_name_newline_only_rejected() {
            assert!(SessionName::parse("\n").is_err());
        }

        #[test]
        fn session_name_mixed_whitespace_only_rejected() {
            assert!(SessionName::parse(" \t \n ").is_err());
        }

        // --- Valid names stored correctly ---

        #[test]
        fn session_name_single_letter_stored() {
            let name = SessionName::parse("Z").expect("valid");
            assert_eq!(name.as_str(), "Z");
        }

        #[test]
        fn session_name_all_digits_after_first_letter() {
            let name = SessionName::parse("a123456789").expect("valid");
            assert_eq!(name.as_str(), "a123456789");
        }

        #[test]
        fn session_name_consecutive_hyphens_valid() {
            assert!(SessionName::parse("a--b").is_ok());
        }

        #[test]
        fn session_name_consecutive_underscores_valid() {
            assert!(SessionName::parse("a__b").is_ok());
        }

        #[test]
        fn session_name_mixed_hyphens_underscores_valid() {
            let name = SessionName::parse("my_test-session_name").expect("valid");
            assert_eq!(name.as_str(), "my_test-session_name");
        }

        #[test]
        fn session_name_case_sensitive_stored() {
            let lower = SessionName::parse("abc").expect("valid");
            let upper = SessionName::parse("ABC").expect("valid");
            assert_ne!(lower, upper);
            assert_eq!(lower.as_str(), "abc");
            assert_eq!(upper.as_str(), "ABC");
        }

        // --- Empty string specificity ---

        #[test]
        fn session_name_empty_returns_empty_error() {
            let err = SessionName::parse("").unwrap_err();
            assert!(matches!(err, IdentifierError::Empty));
        }

        // --- Error message content verification ---

        #[test]
        fn session_name_too_long_error_message() {
            let err = SessionName::parse(&"a".repeat(64)).unwrap_err();
            let msg = format!("{err}");
            assert!(msg.contains("64"), "error should mention actual length");
            assert!(msg.contains("63"), "error should mention max length");
        }

        #[test]
        fn session_name_invalid_chars_error_message() {
            let err = SessionName::parse("bad!name").unwrap_err();
            let msg = format!("{err}");
            assert!(
                msg.contains("invalid") || msg.contains("Invalid"),
                "error should mention invalid characters"
            );
        }

        #[test]
        fn session_name_invalid_start_error_message() {
            let err = SessionName::parse("1bad").unwrap_err();
            let msg = format!("{err}");
            assert!(
                msg.contains("letter") || msg.contains("start"),
                "error should mention letter/start requirement"
            );
        }
    }

    // =========================================================================
    // BeadId Proptests
    // =========================================================================

    mod bead_id_generate_tests {
        use super::*;

        #[test]
        fn bead_id_generate_is_valid() {
            let id = BeadId::generate();
            assert!(id.as_str().starts_with("bd-"));
            let suffix = &id.as_str()[3..];
            assert!(!suffix.is_empty());
            assert!(suffix.len() >= 8);
        }

        #[test]
        fn bead_id_generate_many_are_unique() {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..100 {
                ids.insert(BeadId::generate());
            }
            assert_eq!(ids.len(), 100);
        }

        #[test]
        fn bead_id_serde_roundtrip_generated() {
            let id = BeadId::generate();
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: BeadId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }

        #[test]
        fn bead_id_parse_roundtrip_various_lengths() {
            for len in [1, 4, 8, 12, 16, 32, 64] {
                let suffix: String = (0..len)
                    .map(|i| b"0123456789abcdef"[i % 16] as char)
                    .collect();
                let full = format!("bd-{suffix}");
                let id = BeadId::parse(&full).expect("valid");
                assert_eq!(id.as_str(), full);
            }
        }

        #[test]
        fn bead_id_display_matches_as_str() {
            let id = BeadId::parse("bd-cafebabe").expect("valid");
            assert_eq!(format!("{id}"), "bd-cafebabe");
        }

        #[test]
        fn bead_id_equality_and_hash() {
            let id1 = BeadId::parse("bd-1234").expect("valid");
            let id2 = BeadId::parse("bd-1234").expect("valid");
            let id3 = BeadId::parse("bd-5678").expect("valid");
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(id1.clone());
            assert!(set.contains(&id2));
            assert!(!set.contains(&id3));
        }
    }
}
