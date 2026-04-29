//! Bead value objects: Id, Title, Description.
//!
//! These are immutable types that enforce domain invariants.

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

/// Unique bead identifier.
///
/// # Invariants (I6)
/// - Must be non-empty
/// - Must be ≤100 characters
/// - Must contain only alphanumeric characters, hyphens, and underscores
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(id: impl Into<String>) -> Result<Self, SessionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(SessionError::InvalidBeadId("ID cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidBeadId(format!(
                "ID exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(SessionError::InvalidBeadId(
                "ID must contain only alphanumeric characters, hyphens, and underscores".into(),
            ));
        }
        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated bead title.
///
/// # Invariants (I7)
/// - Must be non-empty
/// - Must be ≤200 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadTitle(String);

impl BeadTitle {
    pub const MAX_LENGTH: usize = 200;

    pub fn new(title: impl Into<String>) -> Result<Self, SessionError> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(SessionError::InvalidBeadTitle(
                "Title cannot be empty".into(),
            ));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(SessionError::InvalidBeadTitle(format!(
                "Title exceeds maximum length of {}",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BeadTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Optional bead description
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadDescription(Option<String>);

impl BeadDescription {
    pub fn new(description: impl Into<String>) -> Result<Self, SessionError> {
        let description = description.into();
        let trimmed = description.trim();
        if trimmed.is_empty() {
            return Ok(Self(None));
        }

        Ok(Self(Some(trimmed.to_string())))
    }

    #[must_use]
    pub const fn as_option(&self) -> Option<&String> {
        self.0.as_ref()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.as_ref().is_none_or(|s| s.is_empty())
    }
}

impl std::fmt::Display for BeadDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => write!(f, "{}", s),
            None => write!(f, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_id_valid() {
        let id = BeadId::new("bd-123").unwrap();
        assert_eq!(id.as_str(), "bd-123");
    }

    #[test]
    fn bead_id_empty_fails() {
        let result = BeadId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn bead_id_too_long_fails() {
        let long_id = "a".repeat(101);
        let result = BeadId::new(long_id);
        assert!(result.is_err());
    }

    #[test]
    fn bead_id_invalid_chars_fails() {
        let result = BeadId::new("bd-123!");
        assert!(result.is_err());
    }

    #[test]
    fn bead_title_valid() {
        let title = BeadTitle::new("Test Bead").unwrap();
        assert_eq!(title.as_str(), "Test Bead");
    }

    #[test]
    fn bead_title_trims_whitespace() {
        let title = BeadTitle::new("  Test Bead  ").unwrap();
        assert_eq!(title.as_str(), "Test Bead");
    }

    #[test]
    fn bead_title_empty_fails() {
        let result = BeadTitle::new("");
        assert!(result.is_err());
    }

    #[test]
    fn bead_title_too_long_fails() {
        let long_title = "a".repeat(201);
        let result = BeadTitle::new(long_title);
        assert!(result.is_err());
    }

    #[test]
    fn bead_description_empty() {
        let desc = BeadDescription::new("").unwrap();
        assert!(desc.is_empty());
        assert!(desc.as_option().is_none());
    }

    #[test]
    fn bead_description_with_content() {
        let desc = BeadDescription::new("Some description").unwrap();
        assert!(!desc.is_empty());
        assert_eq!(desc.as_option(), Some(&"Some description".to_string()));
    }

    // =========================================================================
    // BeadId Extended Tests
    // =========================================================================

    mod bead_value_id_tests {
        use super::*;

        #[test]
        fn bead_id_at_max_length() {
            let max_id = "a".repeat(BeadId::MAX_LENGTH);
            let id = BeadId::new(max_id).expect("at max length");
            assert_eq!(id.as_str().len(), BeadId::MAX_LENGTH);
        }

        #[test]
        fn bead_id_exceeds_max_length_rejects() {
            let too_long = "a".repeat(BeadId::MAX_LENGTH + 1);
            let result = BeadId::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn bead_id_with_spaces_rejects() {
            let result = BeadId::new("bd 123");
            assert!(result.is_err());
        }

        #[test]
        fn bead_id_display() {
            let id = BeadId::new("bd-display-test").expect("valid");
            assert_eq!(format!("{id}"), "bd-display-test");
        }

        #[test]
        fn bead_id_serde_roundtrip() {
            let id = BeadId::new("bd-serde-test").expect("valid");
            let json = serde_json::to_string(&id).expect("serialize");
            let parsed: BeadId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(id, parsed);
        }
    }

    // =========================================================================
    // BeadTitle Extended Tests
    // =========================================================================

    mod bead_value_title_tests {
        use super::*;

        #[test]
        fn bead_title_at_max_length() {
            let max_title = "a".repeat(BeadTitle::MAX_LENGTH);
            let title = BeadTitle::new(max_title).expect("at max");
            assert_eq!(title.as_str().len(), BeadTitle::MAX_LENGTH);
        }

        #[test]
        fn bead_title_exceeds_max_rejects() {
            let too_long = "a".repeat(BeadTitle::MAX_LENGTH + 1);
            let result = BeadTitle::new(too_long);
            assert!(result.is_err());
        }

        #[test]
        fn bead_title_display() {
            let title = BeadTitle::new("Show Title").expect("valid");
            assert_eq!(format!("{title}"), "Show Title");
        }

        #[test]
        fn bead_title_serde_roundtrip() {
            let title = BeadTitle::new("Serde Title").expect("valid");
            let json = serde_json::to_string(&title).expect("serialize");
            let parsed: BeadTitle = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(title, parsed);
        }
    }

    // =========================================================================
    // BeadDescription Extended Tests
    // =========================================================================

    mod bead_value_desc_tests {
        use super::*;

        #[test]
        fn bead_description_whitespace_only_becomes_none() {
            let desc = BeadDescription::new("   ").expect("valid");
            assert!(desc.is_empty());
            assert!(desc.as_option().is_none());
        }

        #[test]
        fn bead_description_trims_whitespace() {
            let desc = BeadDescription::new("  padded  ").expect("valid");
            assert_eq!(desc.as_option(), Some(&"padded".to_string()));
        }

        #[test]
        fn bead_description_display_none() {
            let desc = BeadDescription::new("").expect("valid");
            assert_eq!(format!("{desc}"), "");
        }

        #[test]
        fn bead_description_display_with_content() {
            let desc = BeadDescription::new("Hello").expect("valid");
            assert_eq!(format!("{desc}"), "Hello");
        }

        #[test]
        fn bead_description_serde_roundtrip() {
            let desc = BeadDescription::new("Serialize me").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: BeadDescription = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(desc, parsed);
        }

        #[test]
        fn bead_description_serde_roundtrip_empty() {
            let desc = BeadDescription::new("").expect("valid");
            let json = serde_json::to_string(&desc).expect("serialize");
            let parsed: BeadDescription = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(desc, parsed);
        }
    }
}
