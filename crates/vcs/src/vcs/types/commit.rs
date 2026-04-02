//! Commit ID type
//!
//! This module provides `CommitId` - unique identifier for a commit.

use serde::{Deserialize, Serialize};

use crate::vcs::errors::VcsError;

// ============================================================================
// Helper Functions
// ============================================================================

fn is_effectively_empty(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.trim().is_empty() {
        return true;
    }

    s.chars().all(|c| c.is_whitespace() || is_invisible_char(c))
}

fn is_invisible_char(c: char) -> bool {
    matches!(
        c,
        '\u{FEFF}'
            | '\u{200B}'
            | '\u{200C}'
            | '\u{200D}'
            | '\u{2060}'
            | '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{115F}'
            | '\u{1160}'
    ) || is_in_range(c, '\u{2061}', '\u{2064}')
        || is_in_range(c, '\u{206A}', '\u{206F}')
        || is_in_range(c, '\u{17B4}', '\u{17B5}')
        || is_in_range(c, '\u{202A}', '\u{202E}')
        || is_in_range(c, '\u{2066}', '\u{2069}')
}

fn is_in_range(c: char, start: char, end: char) -> bool {
    c >= start && c <= end
}

// ============================================================================
// CommitId
// ============================================================================

/// Unique identifier for a commit
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new commit ID with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidCommitId` if ID is empty, whitespace-only, or contains only invisible characters
    pub fn new(id: impl Into<String>) -> Result<Self, VcsError> {
        let id = id.into();

        if is_effectively_empty(&id) {
            return Err(VcsError::InvalidCommitId(id));
        }

        Ok(Self(id))
    }

    /// Get the commit ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Valid commit IDs --

    #[test]
    fn commit_id_simple() {
        let id = CommitId::new("abc123").expect("valid");
        assert_eq!(id.as_str(), "abc123");
    }

    #[test]
    fn commit_id_full_sha() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let id = CommitId::new(sha).expect("valid");
        assert_eq!(id.as_str(), sha);
    }

    #[test]
    fn commit_id_numeric() {
        let id = CommitId::new("12345").expect("valid");
        assert_eq!(id.as_str(), "12345");
    }

    #[test]
    fn commit_id_with_special_chars() {
        // CommitId is very permissive - only rejects effectively empty
        let id = CommitId::new("v1.0.0+build.123").expect("valid");
        assert_eq!(id.as_str(), "v1.0.0+build.123");
    }

    // -- Invalid commit IDs --

    #[test]
    fn commit_id_empty_rejects() {
        assert!(matches!(CommitId::new(""), Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn commit_id_whitespace_only_rejects() {
        assert!(matches!(CommitId::new("   "), Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn commit_id_tabs_only_rejects() {
        assert!(matches!(CommitId::new("\t\t"), Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn commit_id_newlines_only_rejects() {
        assert!(matches!(CommitId::new("\n\n"), Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn commit_id_invisible_chars_only_rejects() {
        assert!(matches!(CommitId::new("\u{200B}"), Err(VcsError::InvalidCommitId(_))));
    }

    #[test]
    fn commit_id_bom_only_rejects() {
        assert!(matches!(CommitId::new("\u{FEFF}"), Err(VcsError::InvalidCommitId(_))));
    }

    // -- Properties --

    #[test]
    fn commit_id_clone() {
        let id = CommitId::new("abc123").expect("valid");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn commit_id_eq() {
        let a = CommitId::new("abc123").expect("valid");
        let b = CommitId::new("abc123").expect("valid");
        let c = CommitId::new("xyz789").expect("valid");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn commit_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CommitId::new("abc123").expect("valid"));
        set.insert(CommitId::new("abc123").expect("valid"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn commit_id_serde_roundtrip() {
        let id = CommitId::new("deadbeef1234567").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: CommitId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn commit_id_debug_format() {
        let id = CommitId::new("abc123").expect("valid");
        let debug = format!("{id:?}");
        assert!(debug.contains("abc123"));
    }

    // -- Helper function tests --

    #[test]
    fn is_effectively_empty_true_cases() {
        assert!(is_effectively_empty(""));
        assert!(is_effectively_empty("   "));
        assert!(is_effectively_empty("\t"));
        assert!(is_effectively_empty("\n\r"));
        assert!(is_effectively_empty("\u{200B}"));
    }

    #[test]
    fn is_effectively_empty_false_cases() {
        assert!(!is_effectively_empty("a"));
        assert!(!is_effectively_empty(" a "));
        assert!(!is_effectively_empty("abc def"));
    }

    // -- Proptests --

    proptest::proptest! {
        #[test]
        fn commit_id_non_empty_never_panics(s in "[a-zA-Z0-9_+/=-]{1,100}") {
            let result = CommitId::new(&s);
            assert!(result.is_ok(), "Failed for: {s}");
            assert_eq!(result.expect("valid").as_str(), s);
        }
    }
}
