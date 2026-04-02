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
