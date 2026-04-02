//! Change ID type
//!
//! This module provides `ChangeId` - unique identifier for a VCS change/commit (Git SHA or JJ ID).

use serde::{Deserialize, Serialize};

use crate::vcs::errors::ParseError;
use crate::vcs::types::backend_type::BackendType;

// ============================================================================
// Helper Functions
// ============================================================================

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

fn is_effectively_empty(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.trim().is_empty() {
        return true;
    }

    s.chars().all(|c| c.is_whitespace() || is_invisible_char(c))
}

// ============================================================================
// ChangeIdInner
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ChangeIdInner {
    /// Git commit SHA (7-40 lowercase hex chars)
    Git { sha: String },
    /// JJ change ID (lowercase base36)
    Jj { id: String },
}

// ============================================================================
// ChangeId
// ============================================================================

/// Unique identifier for a VCS change/commit
///
/// # Invariants
/// - Always contains a non-empty, trimmed ID string
/// - Git SHAs are lowercase hex
/// - JJ change IDs are lowercase base36
/// - Backend type is encoded to prevent cross-backend comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId {
    inner: ChangeIdInner,
}

impl ChangeId {
    /// Create a Git `ChangeId` from a SHA string
    ///
    /// # Preconditions
    /// - P1: `sha` is not empty
    /// - P2: `sha` contains only hex characters (0-9, a-f, A-F)
    /// - P4: `sha` length is 7-40 characters
    ///
    /// # Postconditions
    /// - Q4: SHA is normalized to lowercase
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-hex chars present
    /// - `ParseError::InvalidGitShaLength` if length invalid
    pub fn from_git_sha(sha: impl AsRef<str>) -> Result<Self, ParseError> {
        let sha = sha.as_ref().trim();

        if is_effectively_empty(sha) {
            return Err(ParseError::Empty);
        }

        let len = sha.len();
        if !(7..=40).contains(&len) {
            return Err(ParseError::InvalidGitShaLength(len));
        }

        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseError::InvalidCharacters(sha.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Git {
                sha: sha.to_lowercase(),
            },
        })
    }

    /// Create a JJ `ChangeId` from a change ID string
    ///
    /// # Preconditions
    /// - P1: `id` is not empty
    /// - P3: `id` contains only base36 characters (0-9, a-z)
    /// - P5: `id` length is >= 1
    ///
    /// # Errors
    /// - `ParseError::Empty` if input is empty/whitespace
    /// - `ParseError::InvalidCharacters` if non-base36 chars present
    /// - `ParseError::InvalidJjLength` if length is 0
    pub fn from_jj_id(id: impl AsRef<str>) -> Result<Self, ParseError> {
        let id = id.as_ref().trim();

        if is_effectively_empty(id) {
            return Err(ParseError::Empty);
        }

        let len = id.len();
        if len == 0 {
            return Err(ParseError::InvalidJjLength(len));
        }

        let normalized = id.to_lowercase();
        if !normalized
            .chars()
            .all(|c: char| c.is_ascii_digit() || c.is_ascii_lowercase())
        {
            return Err(ParseError::InvalidCharacters(id.to_string()));
        }

        Ok(Self {
            inner: ChangeIdInner::Jj { id: normalized },
        })
    }

    /// Get the backend type for this `ChangeId`
    ///
    /// # Postconditions
    /// - Q3: Returns correct `BackendType`
    #[must_use]
    pub fn backend_type(&self) -> BackendType {
        match &self.inner {
            ChangeIdInner::Git { .. } => BackendType::Git,
            ChangeIdInner::Jj { .. } => BackendType::Jj,
        }
    }

    /// Get the ID as a string slice (without backend prefix)
    ///
    /// # Postconditions
    /// - Q2: Returns inner ID only
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.inner {
            ChangeIdInner::Git { sha } => sha,
            ChangeIdInner::Jj { id } => id,
        }
    }
}

impl std::str::FromStr for ChangeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();

        if is_effectively_empty(trimmed) {
            return Err(ParseError::Empty);
        }

        let is_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());

        if is_hex {
            Self::from_git_sha(trimmed)
        } else {
            Self::from_jj_id(trimmed)
        }
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ChangeIdInner::Git { sha } => write!(f, "git:{sha}"),
            ChangeIdInner::Jj { id } => write!(f, "jj:{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::proptest;

    // -- from_git_sha valid cases --

    #[test]
    fn git_sha_7_chars() {
        let id = ChangeId::from_git_sha("abcdef0").expect("valid");
        assert_eq!(id.as_str(), "abcdef0");
        assert_eq!(id.backend_type(), BackendType::Git);
    }

    #[test]
    fn git_sha_40_chars() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let id = ChangeId::from_git_sha(sha).expect("valid");
        assert_eq!(id.as_str(), sha);
    }

    #[test]
    fn git_sha_uppercase_normalized() {
        let id = ChangeId::from_git_sha("ABCDEF0123456789").expect("valid");
        assert_eq!(id.as_str(), "abcdef0123456789");
    }

    #[test]
    fn git_sha_mixed_case_normalized() {
        let id = ChangeId::from_git_sha("AbCdEf0123456789").expect("valid");
        assert_eq!(id.as_str(), "abcdef0123456789");
    }

    #[test]
    fn git_sha_trimmed() {
        let id = ChangeId::from_git_sha("  abcdef01234567  ").expect("valid");
        assert_eq!(id.as_str(), "abcdef01234567");
    }

    // -- from_git_sha invalid cases --

    #[test]
    fn git_sha_empty_rejects() {
        assert_eq!(ChangeId::from_git_sha(""), Err(ParseError::Empty));
    }

    #[test]
    fn git_sha_whitespace_rejects() {
        assert_eq!(ChangeId::from_git_sha("   "), Err(ParseError::Empty));
    }

    #[test]
    fn git_sha_too_short_rejects() {
        assert_eq!(ChangeId::from_git_sha("abc12"), Err(ParseError::InvalidGitShaLength(5)));
    }

    #[test]
    fn git_sha_too_long_rejects() {
        let long = "0123456789abcdef0123456789abcdef012345678";
        assert_eq!(ChangeId::from_git_sha(long), Err(ParseError::InvalidGitShaLength(41)));
    }

    #[test]
    fn git_sha_non_hex_rejects() {
        assert!(matches!(ChangeId::from_git_sha("ghijklm"), Err(ParseError::InvalidCharacters(_))));
    }

    #[test]
    fn git_sha_with_spaces_rejects() {
        assert!(matches!(ChangeId::from_git_sha("abc def0"), Err(ParseError::InvalidCharacters(_))));
    }

    // -- from_jj_id valid cases --

    #[test]
    fn jj_id_simple() {
        let id = ChangeId::from_jj_id("abc123").expect("valid");
        assert_eq!(id.as_str(), "abc123");
        assert_eq!(id.backend_type(), BackendType::Jj);
    }

    #[test]
    fn jj_id_uppercase_normalized() {
        let id = ChangeId::from_jj_id("ABC123").expect("valid");
        assert_eq!(id.as_str(), "abc123");
    }

    #[test]
    fn jj_id_trimmed() {
        let id = ChangeId::from_jj_id("  abc123  ").expect("valid");
        assert_eq!(id.as_str(), "abc123");
    }

    #[test]
    fn jj_id_single_char() {
        let id = ChangeId::from_jj_id("a").expect("valid");
        assert_eq!(id.as_str(), "a");
    }

    // -- from_jj_id invalid cases --

    #[test]
    fn jj_id_empty_rejects() {
        assert_eq!(ChangeId::from_jj_id(""), Err(ParseError::Empty));
    }

    #[test]
    fn jj_id_whitespace_rejects() {
        assert_eq!(ChangeId::from_jj_id("   "), Err(ParseError::Empty));
    }

    #[test]
    fn jj_id_with_hyphen_rejects() {
        // hyphen is not base36
        assert!(matches!(ChangeId::from_jj_id("abc-123"), Err(ParseError::InvalidCharacters(_))));
    }

    #[test]
    fn jj_id_with_special_chars_rejects() {
        assert!(matches!(ChangeId::from_jj_id("abc@123"), Err(ParseError::InvalidCharacters(_))));
    }

    // -- Display tests --

    #[test]
    fn display_git_format() {
        let id = ChangeId::from_git_sha("abcdef0").expect("valid");
        let display = format!("{id}");
        assert_eq!(display, "git:abcdef0");
    }

    #[test]
    fn display_jj_format() {
        let id = ChangeId::from_jj_id("abc123").expect("valid");
        let display = format!("{id}");
        assert_eq!(display, "jj:abc123");
    }

    // -- FromStr tests --

    #[test]
    fn from_str_hex_parsed_as_git() {
        let id: ChangeId = "abcdef01234567".parse().expect("valid");
        assert_eq!(id.backend_type(), BackendType::Git);
        assert_eq!(id.as_str(), "abcdef01234567");
    }

    #[test]
    fn from_str_non_hex_parsed_as_jj() {
        let id: ChangeId = "klmnop".parse().expect("valid");
        assert_eq!(id.backend_type(), BackendType::Jj);
        assert_eq!(id.as_str(), "klmnop");
    }

    #[test]
    fn from_str_empty_rejects() {
        let result: Result<ChangeId, ParseError> = "".parse();
        assert_eq!(result, Err(ParseError::Empty));
    }

    #[test]
    fn from_str_whitespace_rejects() {
        let result: Result<ChangeId, ParseError> = "   ".parse();
        assert_eq!(result, Err(ParseError::Empty));
    }

    // -- Clone, Eq, Hash --

    #[test]
    fn change_id_clone() {
        let id = ChangeId::from_git_sha("abcdef0").expect("valid");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn change_id_eq_same_backend() {
        let a = ChangeId::from_git_sha("abcdef0").expect("valid");
        let b = ChangeId::from_git_sha("abcdef0").expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn change_id_neq_different_values_same_backend() {
        let a = ChangeId::from_git_sha("abcdef0").expect("valid");
        let b = ChangeId::from_git_sha("fedcba0").expect("valid");
        assert_ne!(a, b);
    }

    #[test]
    fn change_id_neq_different_backends_same_string() {
        // "abc123" is valid for both git and jj, but they are different backends
        let git_id = ChangeId::from_git_sha("abc1230").expect("valid");
        let jj_id = ChangeId::from_jj_id("abc1230").expect("valid");
        assert_ne!(git_id, jj_id);
        assert_eq!(git_id.backend_type(), BackendType::Git);
        assert_eq!(jj_id.backend_type(), BackendType::Jj);
    }

    #[test]
    fn change_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ChangeId::from_git_sha("abcdef0").expect("valid"));
        set.insert(ChangeId::from_git_sha("abcdef0").expect("valid"));
        assert_eq!(set.len(), 1);
    }

    // -- Serde roundtrip --

    #[test]
    fn change_id_serde_roundtrip_git() {
        let id = ChangeId::from_git_sha("deadbeef1234567").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: ChangeId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, deserialized);
        assert_eq!(deserialized.backend_type(), BackendType::Git);
    }

    #[test]
    fn change_id_serde_roundtrip_jj() {
        let id = ChangeId::from_jj_id("kmnoqrstuvwxyz").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: ChangeId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, deserialized);
        assert_eq!(deserialized.backend_type(), BackendType::Jj);
    }

    #[test]
    fn change_id_debug_format() {
        let id = ChangeId::from_git_sha("abcdef0").expect("valid");
        let debug = format!("{id:?}");
        assert!(debug.contains("Git"));
        assert!(debug.contains("abcdef0"));
    }

    // -- ParseError tests --

    #[test]
    fn parse_error_eq() {
        assert_eq!(ParseError::Empty, ParseError::Empty);
        assert_eq!(ParseError::InvalidGitShaLength(5), ParseError::InvalidGitShaLength(5));
        assert_ne!(ParseError::InvalidGitShaLength(5), ParseError::InvalidGitShaLength(6));
        assert_eq!(ParseError::InvalidCharacters("abc".to_string()), ParseError::InvalidCharacters("abc".to_string()));
    }

    #[test]
    fn parse_error_clone() {
        let err = ParseError::InvalidCharacters("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn parse_error_display_empty() {
        let msg = format!("{}", ParseError::Empty);
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn parse_error_display_invalid_characters() {
        let msg = format!("{}", ParseError::InvalidCharacters("@#$".to_string()));
        assert!(msg.contains("@#$"));
    }

    #[test]
    fn parse_error_display_invalid_git_length() {
        let msg = format!("{}", ParseError::InvalidGitShaLength(3));
        assert!(msg.contains("3"));
    }

    #[test]
    fn parse_error_display_invalid_jj_length() {
        let msg = format!("{}", ParseError::InvalidJjLength(0));
        assert!(msg.contains("0"));
    }

    // -- Proptests --

    proptest! {
        #[test]
        fn git_sha_valid_hex_always_succeeds(s in "[0-9a-f]{7,40}") {
            let result = ChangeId::from_git_sha(&s);
            assert!(result.is_ok(), "Failed for: {s}");
            let id = result.expect("valid");
            assert_eq!(id.backend_type(), BackendType::Git);
            assert_eq!(id.as_str(), s.to_lowercase());
        }

        #[test]
        fn jj_id_valid_alphanumeric_always_succeeds(s in "[0-9a-z]{1,30}") {
            let result = ChangeId::from_jj_id(&s);
            assert!(result.is_ok(), "Failed for: {s}");
            let id = result.expect("valid");
            assert_eq!(id.backend_type(), BackendType::Jj);
        }

        #[test]
        fn git_sha_display_includes_prefix(s in "[0-9a-f]{7,40}") {
            let id = ChangeId::from_git_sha(&s).expect("valid");
            let display = format!("{id}");
            assert!(display.starts_with("git:"));
        }

        #[test]
        fn jj_id_display_includes_prefix(s in "[0-9a-z]{1,30}") {
            let id = ChangeId::from_jj_id(&s).expect("valid");
            let display = format!("{id}");
            assert!(display.starts_with("jj:"));
        }
    }
}
