use serde::{Deserialize, Serialize};

use crate::error::{BeadError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(BeadError::InvalidId("ID cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(BeadError::InvalidId(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(BeadError::InvalidId(
                "ID must contain only alphanumeric characters, hyphens, and underscores".into(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for BeadId {
    type Error = BeadError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for BeadId {
    type Error = BeadError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alphanumeric_id() {
        let id = BeadId::new("abc123").unwrap();
        assert_eq!(id.as_str(), "abc123");
    }

    #[test]
    fn valid_id_with_hyphens() {
        let id = BeadId::new("abc-123").unwrap();
        assert_eq!(id.as_str(), "abc-123");
    }

    #[test]
    fn valid_id_with_underscores() {
        let id = BeadId::new("abc_123").unwrap();
        assert_eq!(id.as_str(), "abc_123");
    }

    #[test]
    fn empty_id_is_rejected() {
        let result = BeadId::new("");
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidId(msg) => assert!(msg.contains("empty")),
            other => panic!("expected InvalidId, got {other:?}"),
        }
    }

    #[test]
    fn whitespace_only_id_is_rejected() {
        let result = BeadId::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn id_exceeding_max_length_is_rejected() {
        let long_id = "a".repeat(BeadId::MAX_LENGTH + 1);
        let result = BeadId::new(long_id);
        assert!(result.is_err());
    }

    #[test]
    fn id_at_max_length_is_accepted() {
        let id = BeadId::new("a".repeat(BeadId::MAX_LENGTH)).unwrap();
        assert_eq!(id.as_str().len(), BeadId::MAX_LENGTH);
    }

    #[test]
    fn id_with_spaces_is_rejected() {
        let result = BeadId::new("has spaces");
        assert!(result.is_err());
    }

    #[test]
    fn id_with_special_chars_is_rejected() {
        let result = BeadId::new("has@special#chars");
        assert!(result.is_err());
    }

    #[test]
    fn display_returns_inner_value() {
        let id = BeadId::new("test-id").unwrap();
        assert_eq!(format!("{id}"), "test-id");
    }

    #[test]
    fn into_inner_returns_owned_string() {
        let id = BeadId::new("my-id").unwrap();
        let inner = id.into_inner();
        assert_eq!(inner, "my-id");
    }

    #[test]
    fn try_from_string_works() {
        let id: BeadId = "valid_id".try_into().unwrap();
        assert_eq!(id.as_str(), "valid_id");
    }

    #[test]
    fn try_from_ref_str_works() {
        let id = BeadId::try_from("valid_id").unwrap();
        assert_eq!(id.as_str(), "valid_id");
    }

    #[test]
    fn try_from_invalid_string_fails() {
        let result = BeadId::try_from("bad id!");
        assert!(result.is_err());
    }

    #[test]
    fn equality_works() {
        let a = BeadId::new("same").unwrap();
        let b = BeadId::new("same").unwrap();
        let c = BeadId::new("different").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BeadId::new("x").unwrap());
        assert!(set.contains(&BeadId::new("x").unwrap()));
        assert!(!set.contains(&BeadId::new("y").unwrap()));
    }

    #[test]
    fn serde_roundtrip() {
        let id = BeadId::new("serde-test").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: BeadId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn id_with_single_char_is_accepted() {
        let id = BeadId::new("a").unwrap();
        assert_eq!(id.as_str(), "a");
    }

    #[test]
    fn id_exactly_max_length_plus_one_is_rejected() {
        let long_id: String = "a".repeat(BeadId::MAX_LENGTH);
        let too_long: String = "b".repeat(BeadId::MAX_LENGTH + 1);
        assert!(BeadId::new(long_id).is_ok());
        assert!(BeadId::new(too_long).is_err());
    }

    #[test]
    fn id_with_only_hyphens_is_accepted() {
        let id = BeadId::new("---").unwrap();
        assert_eq!(id.as_str(), "---");
    }

    #[test]
    fn id_with_only_underscores_is_accepted() {
        let id = BeadId::new("___").unwrap();
        assert_eq!(id.as_str(), "___");
    }

    #[test]
    fn id_with_newline_is_rejected() {
        let result = BeadId::new("has\nnewline");
        assert!(result.is_err());
    }

    #[test]
    fn id_with_tab_is_rejected() {
        let result = BeadId::new("has\ttab");
        assert!(result.is_err());
    }

    #[test]
    fn id_with_slash_is_rejected() {
        let result = BeadId::new("a/b/c");
        assert!(result.is_err());
    }

    #[test]
    fn id_with_dot_is_rejected() {
        let result = BeadId::new("a.b.c");
        assert!(result.is_err());
    }

    mod proptest_bead_id {
        use super::*;
        use proptest::proptest;

        proptest! {
            #[test]
            fn valid_id_roundtrips(ref s in "[a-zA-Z0-9_-]{1,100}") {
                let id = BeadId::new(s.as_str()).unwrap();
                assert_eq!(id.as_str(), s.as_str());
            }

            #[test]
            fn id_exceeding_max_is_rejected(ref s in ".{101,200}") {
                let result = BeadId::new(s.as_str());
                assert!(result.is_err());
            }

            #[test]
            fn id_with_invalid_chars_rejected(ref s in "[a-zA-Z0-9_-]{0,10}[ @!#.][a-zA-Z0-9_-]{0,10}") {
                if !s.is_empty() && s.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                    let result = BeadId::new(s.as_str());
                    assert!(result.is_err(), "expected rejection for: {:?}", s);
                }
            }
        }
    }
}
