//! Branch name type
//!
//! This module provides `BranchName` - named reference to a line of development.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::vcs::errors::VcsError;

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

fn has_invalid_branch_syntax(name: &str) -> bool {
    if name == "@" {
        return true;
    }

    if name.starts_with('/') || name.starts_with('-') || name.ends_with('/') || name.ends_with('.') {
        return true;
    }

    if name.contains("..")
        || name.contains("@{")
        || std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
    {
        return true;
    }

    if name.chars().any(|char| {
        char.is_control() || matches!(char, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return true;
    }

    name.split('/').any(str::is_empty)
}

// ============================================================================
// BranchName
// ============================================================================

/// Name of a branch in the VCS
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchName(String);

impl Serialize for BranchName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for BranchName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(|e| serde::de::Error::custom(format!("{e}")))
    }
}

impl BranchName {
    /// Create a new branch name with validation
    ///
    /// # Errors
    /// - `VcsError::InvalidBranchName` if name is empty, whitespace-only, or contains only invisible characters
    pub fn new(name: impl Into<String>) -> Result<Self, VcsError> {
        let name = name.into();

        if is_effectively_empty(&name) || has_invalid_branch_syntax(&name) {
            return Err(VcsError::InvalidBranchName(name));
        }

        Ok(Self(name))
    }

    /// Get the branch name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Valid branch names --

    #[test]
    fn branch_name_simple() {
        let b = BranchName::new("main").expect("valid");
        assert_eq!(b.as_str(), "main");
    }

    #[test]
    fn branch_name_with_slashes() {
        let b = BranchName::new("feature/my-awesome-feature").expect("valid");
        assert_eq!(b.as_str(), "feature/my-awesome-feature");
    }

    #[test]
    fn branch_name_with_dots() {
        let b = BranchName::new("release/v1.0.0").expect("valid");
        assert_eq!(b.as_str(), "release/v1.0.0");
    }

    #[test]
    fn branch_name_with_hyphens() {
        let b = BranchName::new("fix-123-bug-description").expect("valid");
        assert_eq!(b.as_str(), "fix-123-bug-description");
    }

    #[test]
    fn branch_name_with_underscores() {
        let b = BranchName::new("my_feature_branch").expect("valid");
        assert_eq!(b.as_str(), "my_feature_branch");
    }

    #[test]
    fn branch_name_single_char() {
        let b = BranchName::new("a").expect("valid");
        assert_eq!(b.as_str(), "a");
    }

    #[test]
    fn branch_name_numeric() {
        let b = BranchName::new("12345").expect("valid");
        assert_eq!(b.as_str(), "12345");
    }

    #[test]
    fn branch_name_with_tilde_after_first_char() {
        // Tilde is invalid, so this should be rejected
        let result = BranchName::new("test~");
        assert!(result.is_err());
    }

    // -- Invalid branch names --

    #[test]
    fn branch_name_empty_rejects() {
        assert!(matches!(BranchName::new(""), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_whitespace_only_rejects() {
        assert!(matches!(BranchName::new("   "), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_tab_only_rejects() {
        assert!(matches!(BranchName::new("\t"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_newline_rejects() {
        assert!(matches!(BranchName::new("test\nbranch"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_space_in_middle_rejects() {
        assert!(matches!(BranchName::new("feature name"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_leading_slash_rejects() {
        assert!(matches!(BranchName::new("/main"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_trailing_slash_rejects() {
        assert!(matches!(BranchName::new("main/"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_trailing_dot_rejects() {
        assert!(matches!(BranchName::new("main."), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_double_dot_rejects() {
        assert!(matches!(BranchName::new("feature/.."), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_double_dot_in_middle_rejects() {
        assert!(matches!(BranchName::new("feat..ure"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_at_brace_rejects() {
        assert!(matches!(BranchName::new("main@{1}"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_tilde_rejects() {
        assert!(matches!(BranchName::new("main~1"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_caret_rejects() {
        assert!(matches!(BranchName::new("main^1"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_colon_rejects() {
        assert!(matches!(BranchName::new("main:feature"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_question_mark_rejects() {
        assert!(matches!(BranchName::new("main?"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_asterisk_rejects() {
        assert!(matches!(BranchName::new("main*"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_open_bracket_rejects() {
        assert!(matches!(BranchName::new("main["), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_backslash_rejects() {
        assert!(matches!(BranchName::new("main\\test"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_single_at_rejects() {
        assert!(matches!(BranchName::new("@"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_single_hyphen_rejects() {
        assert!(matches!(BranchName::new("-"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_leading_hyphen_rejects() {
        assert!(matches!(BranchName::new("-foo"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_leading_double_hyphen_rejects() {
        assert!(matches!(BranchName::new("--verbose"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_lock_extension_rejects() {
        assert!(matches!(BranchName::new("test.LOCK"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_lock_extension_lowercase_rejects() {
        assert!(matches!(BranchName::new("test.lock"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_control_char_rejects() {
        assert!(matches!(BranchName::new("test\x00name"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_empty_segment_rejects() {
        // Double slash creates an empty segment
        assert!(matches!(BranchName::new("feature//main"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_invisible_chars_rejects() {
        assert!(matches!(BranchName::new("main\u{200B}"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_bom_only_rejects() {
        assert!(matches!(BranchName::new("\u{FEFF}"), Err(VcsError::InvalidBranchName(_))));
    }

    #[test]
    fn branch_name_zero_width_space_only_rejects() {
        assert!(matches!(BranchName::new("\u{200B}"), Err(VcsError::InvalidBranchName(_))));
    }

    // -- BranchName properties --

    #[test]
    fn branch_name_clone() {
        let b = BranchName::new("main").expect("valid");
        let cloned = b.clone();
        assert_eq!(b, cloned);
    }

    #[test]
    fn branch_name_eq() {
        let a = BranchName::new("main").expect("valid");
        let b = BranchName::new("main").expect("valid");
        let c = BranchName::new("develop").expect("valid");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn branch_name_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BranchName::new("main").expect("valid"));
        set.insert(BranchName::new("main").expect("valid"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn branch_name_serde_roundtrip() {
        let b = BranchName::new("feature/test").expect("valid");
        let json = serde_json::to_string(&b).expect("serialize");
        let deserialized: BranchName = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b, deserialized);
    }

    #[test]
    fn branch_name_serde_rejects_invalid() {
        let json = "\"--upload-pack=evil\"";
        let result = serde_json::from_str::<BranchName>(json);
        assert!(result.is_err(), "serde must reject -- prefixed branch names");
    }

    #[test]
    fn branch_name_serde_rejects_leading_hyphen() {
        let json = "\"-foo\"";
        let result = serde_json::from_str::<BranchName>(json);
        assert!(result.is_err(), "serde must reject - prefixed branch names");
    }

    #[test]
    fn branch_name_serde_rejects_empty() {
        let json = "\"\"";
        let result = serde_json::from_str::<BranchName>(json);
        assert!(result.is_err(), "serde must reject empty branch names");
    }

    #[test]
    fn branch_name_serde_rejects_tilde() {
        let json = "\"main~1\"";
        let result = serde_json::from_str::<BranchName>(json);
        assert!(result.is_err(), "serde must reject tilde in branch names");
    }

    #[test]
    fn branch_name_debug_format() {
        let b = BranchName::new("main").expect("valid");
        let debug = format!("{b:?}");
        assert!(debug.contains("main"));
    }

    // -- Helper function tests --

    #[test]
    fn is_effectively_empty_true_cases() {
        assert!(is_effectively_empty(""));
        assert!(is_effectively_empty("   "));
        assert!(is_effectively_empty("\t\n\r"));
    }

    #[test]
    fn is_effectively_empty_false_cases() {
        assert!(!is_effectively_empty("main"));
        assert!(!is_effectively_empty(" a "));
    }

    #[test]
    fn has_invalid_branch_syntax_valid_cases() {
        assert!(!has_invalid_branch_syntax("main"));
        assert!(!has_invalid_branch_syntax("feature/test"));
        assert!(!has_invalid_branch_syntax("release/v1.0"));
    }

    #[test]
    fn has_invalid_branch_syntax_invalid_cases() {
        assert!(has_invalid_branch_syntax("@"));
        assert!(has_invalid_branch_syntax("-"));
        assert!(has_invalid_branch_syntax("-foo"));
        assert!(has_invalid_branch_syntax("--verbose"));
        assert!(has_invalid_branch_syntax("/main"));
        assert!(has_invalid_branch_syntax("main/"));
        assert!(has_invalid_branch_syntax("main."));
        assert!(has_invalid_branch_syntax("feat..ure"));
        assert!(has_invalid_branch_syntax("test@{1}"));
        assert!(has_invalid_branch_syntax("test~"));
        assert!(has_invalid_branch_syntax("test^"));
        assert!(has_invalid_branch_syntax("test:branch"));
        assert!(has_invalid_branch_syntax("test?"));
        assert!(has_invalid_branch_syntax("test*"));
        assert!(has_invalid_branch_syntax("test["));
        assert!(has_invalid_branch_syntax("test\\path"));
        assert!(has_invalid_branch_syntax("test.lock"));
        assert!(has_invalid_branch_syntax("feat//ure"));
    }

    // -- Proptests --

    proptest::proptest! {
        #[test]
        fn branch_name_alphanumeric_never_panics(s in "[a-zA-Z0-9_-]{1,100}") {
            let _ = BranchName::new(&s);
        }

        #[test]
        fn branch_name_with_slashes_never_panics(s in "[a-zA-Z0-9_./-]{1,100}") {
            let _ = BranchName::new(&s);
        }
    }
}
