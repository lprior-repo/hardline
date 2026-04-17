use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: String) -> Result<Self, WorkspaceError> {
        if name.is_empty() {
            return Err(WorkspaceError::InvalidBranchName("empty name".into()));
        }
        if name.contains('\0') {
            return Err(WorkspaceError::InvalidBranchName(
                "null character not allowed".into(),
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BranchName {
    fn default() -> Self {
        Self("main".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_valid() {
        let name = BranchName::new("feature/login".into());
        assert!(name.is_ok());
    }

    #[test]
    fn branch_name_empty_fails() {
        let name = BranchName::new("".into());
        assert!(name.is_err());
    }

    #[test]
    fn branch_name_null_char_fails() {
        let result = BranchName::new("feature\0evil".into());
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_single_char_succeeds() {
        let result = BranchName::new("a".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "a");
    }

    #[test]
    fn branch_name_with_slash_succeeds() {
        let result = BranchName::new("feature/user-profile".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_with_spaces_succeeds() {
        let result = BranchName::new("my branch".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_with_special_chars_succeeds() {
        let result = BranchName::new("fix/issue-123!@#$%".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_as_str_returns_inner() {
        let name = BranchName::new("main".into()).unwrap();
        assert_eq!(name.as_str(), "main");
    }

    #[test]
    fn branch_name_default_is_main() {
        assert_eq!(BranchName::default().as_str(), "main");
    }

    #[test]
    fn branch_name_equality() {
        let a = BranchName::new("feature/x".into()).unwrap();
        let b = BranchName::new("feature/x".into()).unwrap();
        let c = BranchName::new("feature/y".into()).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn branch_name_clone() {
        let name = BranchName::new("develop".into()).unwrap();
        let cloned = name.clone();
        assert_eq!(name, cloned);
    }

    #[test]
    fn branch_name_serialization_roundtrip() {
        let name = BranchName::new("feature/serde-test".into()).unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let deserialized: BranchName = serde_json::from_str(&json).unwrap();
        assert_eq!(name, deserialized);
    }

    #[test]
    fn branch_name_long_name_succeeds() {
        let long_name = "a".repeat(10_000);
        let result = BranchName::new(long_name.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), &long_name);
    }

    #[test]
    fn branch_name_only_null_char_fails() {
        let result = BranchName::new("\0".into());
        assert!(result.is_err());
    }

    // --- Additional unit tests ---

    #[test]
    fn branch_name_hash_set_deduplication() {
        use std::collections::HashSet;
        let a = BranchName::new("main".into()).unwrap();
        let b = BranchName::new("main".into()).unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn branch_name_debug_format() {
        let name = BranchName::new("debug-branch".into()).unwrap();
        let debug_str = format!("{name:?}");
        assert!(debug_str.contains("debug-branch"));
    }

    #[test]
    fn branch_name_null_char_in_middle_fails() {
        let result = BranchName::new("feat\0ure".into());
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_null_char_at_end_fails() {
        let result = BranchName::new("feature\0".into());
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_null_char_at_start_fails() {
        let result = BranchName::new("\0feature".into());
        assert!(result.is_err());
    }

    #[test]
    fn branch_name_empty_error_type() {
        let result = BranchName::new("".into());
        match result.err() {
            Some(WorkspaceError::InvalidBranchName(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidBranchName, got {other:?}"),
        }
    }

    #[test]
    fn branch_name_null_char_error_type() {
        let result = BranchName::new("test\0".into());
        match result.err() {
            Some(WorkspaceError::InvalidBranchName(msg)) => {
                assert!(msg.contains("null"));
            }
            other => panic!("expected InvalidBranchName, got {other:?}"),
        }
    }

    #[test]
    fn branch_name_unicode_succeeds() {
        let result = BranchName::new("feature/日本語".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_emoji_succeeds() {
        let result = BranchName::new("feature/rocket".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_with_newline_succeeds() {
        // Only null char is rejected
        let result = BranchName::new("branch\nwith-newline".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_with_tab_succeeds() {
        let result = BranchName::new("branch\twith-tab".into());
        assert!(result.is_ok());
    }

    #[test]
    fn branch_name_common_patterns() {
        let patterns = vec![
            "main",
            "master",
            "develop",
            "feature/USER-123-add-login",
            "bugfix/fix-crash-on-startup",
            "release/1.0.0",
            "hotfix/urgent-patch",
            "chore/update-deps",
            "refactor/cleanup-utils",
        ];
        for pattern in patterns {
            let result = BranchName::new(pattern.into());
            assert!(result.is_ok(), "pattern '{}' should be valid", pattern);
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};

        proptest! {
            #[test]
            fn branch_name_non_empty_non_null_succeeds(name in "[^\0]{1,500}") {
                let result = BranchName::new(name);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn branch_name_empty_always_fails(s in ".{0}") {
                let result = BranchName::new(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn branch_name_with_null_always_fails(name in "[a-zA-Z0-9]{1,50}") {
                let with_null = format!("{}\0", name);
                let result = BranchName::new(with_null);
                prop_assert!(result.is_err());
            }

            #[test]
            fn branch_name_null_at_any_position_fails(
                prefix in "[a-zA-Z]{1,10}",
                suffix in "[a-zA-Z]{1,10}"
            ) {
                let with_null = format!("{}\0{}", prefix, suffix);
                let result = BranchName::new(with_null);
                prop_assert!(result.is_err());
            }

            #[test]
            fn branch_name_serialization_roundtrip(name in "[a-zA-Z0-9/\\-_]{1,100}") {
                let branch = BranchName::new(name.clone()).unwrap();
                let json = serde_json::to_string(&branch).unwrap();
                let deserialized: BranchName = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(branch, deserialized);
            }

            #[test]
            fn branch_name_as_str_matches_input(name in "[a-zA-Z0-9/\\-_]{1,100}") {
                let branch = BranchName::new(name.clone()).unwrap();
                prop_assert_eq!(branch.as_str(), name);
            }

            #[test]
            fn branch_name_git_style_patterns(
                prefix in "[a-zA-Z]{1,10}",
                suffix in "[a-zA-Z0-9\\-_]{1,50}"
            ) {
                let pattern = format!("{}/{}", prefix, suffix);
                let result = BranchName::new(pattern);
                prop_assert!(result.is_ok());
            }
        }
    }
}
