use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(name: String) -> Result<Self, WorkspaceError> {
        if name.is_empty() {
            return Err(WorkspaceError::InvalidWorkspaceName("empty name".into()));
        }
        if name.len() > 255 {
            return Err(WorkspaceError::InvalidWorkspaceName("name too long".into()));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(WorkspaceError::InvalidWorkspaceName(
                "name contains invalid characters".into(),
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkspaceName {
    fn default() -> Self {
        Self::new("default".into()).expect("default name is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_name_valid() {
        let name = WorkspaceName::new("my-workspace_123".into());
        assert!(name.is_ok());
    }

    #[test]
    fn workspace_name_empty_fails() {
        let name = WorkspaceName::new("".into());
        assert!(name.is_err());
    }

    #[test]
    fn workspace_name_with_slash_fails() {
        let name = WorkspaceName::new("my/workspace".into());
        assert!(name.is_err());
    }

    #[test]
    fn workspace_name_default_is_workspace() {
        assert_eq!(WorkspaceName::default().as_str(), "default");
    }

    #[test]
    fn workspace_name_too_long_fails() {
        let long_name = "a".repeat(256);
        let result = WorkspaceName::new(long_name);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_name_at_max_length_succeeds() {
        let name = "a".repeat(255);
        let result = WorkspaceName::new(name.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), &name);
    }

    #[test]
    fn workspace_name_with_space_fails() {
        let result = WorkspaceName::new("my workspace".into());
        assert!(result.is_err());
    }

    #[test]
    fn workspace_name_with_dot_fails() {
        let result = WorkspaceName::new("my.workspace".into());
        assert!(result.is_err());
    }

    #[test]
    fn workspace_name_underscores_allowed() {
        let result = WorkspaceName::new("my_workspace_name".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_single_char_succeeds() {
        let result = WorkspaceName::new("a".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "a");
    }

    #[test]
    fn workspace_name_numeric_only_succeeds() {
        let result = WorkspaceName::new("12345".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_as_str_returns_inner_value() {
        let name = WorkspaceName::new("test-name".into()).unwrap();
        assert_eq!(name.as_str(), "test-name");
    }

    #[test]
    fn workspace_name_equality() {
        let a = WorkspaceName::new("same".into()).unwrap();
        let b = WorkspaceName::new("same".into()).unwrap();
        let c = WorkspaceName::new("different".into()).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn workspace_name_serialization_roundtrip() {
        let name = WorkspaceName::new("serde-test".into()).unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let deserialized: WorkspaceName = serde_json::from_str(&json).unwrap();
        assert_eq!(name, deserialized);
    }

    // --- Additional unit tests ---

    #[test]
    fn workspace_name_hash_set_deduplication() {
        use std::collections::HashSet;
        let a = WorkspaceName::new("dup".into()).unwrap();
        let b = WorkspaceName::new("dup".into()).unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn workspace_name_clone() {
        let name = WorkspaceName::new("clone-me".into()).unwrap();
        let cloned = name.clone();
        assert_eq!(name, cloned);
    }

    #[test]
    fn workspace_name_debug_format() {
        let name = WorkspaceName::new("debug-name".into()).unwrap();
        let debug_str = format!("{name:?}");
        assert!(debug_str.contains("debug-name"));
    }

    #[test]
    fn workspace_name_starts_with_hyphen() {
        // Only empty, too long, and invalid chars are rejected
        let result = WorkspaceName::new("-name".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_ends_with_hyphen() {
        let result = WorkspaceName::new("name-".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_only_hyphens() {
        let result = WorkspaceName::new("---".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_only_underscores() {
        let result = WorkspaceName::new("___".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_mixed_case() {
        let result = WorkspaceName::new("MyWorkspaceName".into());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_at_boundary_255_succeeds() {
        let name = "a".repeat(255);
        let result = WorkspaceName::new(name);
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_name_at_boundary_256_fails() {
        let name = "a".repeat(256);
        let result = WorkspaceName::new(name);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_name_empty_error_type() {
        let result = WorkspaceName::new("".into());
        match result.err() {
            Some(WorkspaceError::InvalidWorkspaceName(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidWorkspaceName, got {other:?}"),
        }
    }

    #[test]
    fn workspace_name_invalid_chars_error_type() {
        let result = WorkspaceName::new("has.space".into());
        match result.err() {
            Some(WorkspaceError::InvalidWorkspaceName(msg)) => {
                assert!(msg.contains("invalid characters"));
            }
            other => panic!("expected InvalidWorkspaceName, got {other:?}"),
        }
    }

    #[test]
    fn workspace_name_too_long_error_type() {
        let result = WorkspaceName::new("a".repeat(256));
        match result.err() {
            Some(WorkspaceError::InvalidWorkspaceName(msg)) => {
                assert!(msg.contains("too long"));
            }
            other => panic!("expected InvalidWorkspaceName, got {other:?}"),
        }
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::prelude::*;
        use proptest::{prop_assert, prop_assert_eq};
        use super::*;

        proptest! {
            #[test]
            fn workspace_name_valid_patterns(name in "[a-zA-Z0-9_-]{1,255}") {
                let result = WorkspaceName::new(name);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn workspace_name_invalid_chars_rejected(name in "[a-zA-Z0-9_-]{1,10}[. @#][a-zA-Z0-9_-]{0,10}") {
                let result = WorkspaceName::new(name);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_name_serialization_roundtrip(name in "[a-zA-Z0-9_-]{1,100}") {
                let ws_name = WorkspaceName::new(name).unwrap();
                let json = serde_json::to_string(&ws_name).unwrap();
                let deserialized: WorkspaceName = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(ws_name, deserialized);
            }

            #[test]
            fn workspace_name_empty_always_fails(s in ".{0}") {
                let result = WorkspaceName::new(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_name_over_255_always_fails(
                prefix in "[a-z]{1,10}",
                extra_len in 250u32..500u32
            ) {
                let long_name = format!("{}{}", prefix, "a".repeat(extra_len as usize));
                prop_assume!(long_name.len() > 255);
                let result = WorkspaceName::new(long_name);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_name_as_str_matches_input(name in "[a-zA-Z0-9_-]{1,200}") {
                let ws_name = WorkspaceName::new(name.clone()).unwrap();
                prop_assert_eq!(ws_name.as_str(), name);
            }

            #[test]
            fn workspace_name_equality_for_same_input(name in "[a-zA-Z0-9_-]{1,200}") {
                let a = WorkspaceName::new(name.clone()).unwrap();
                let b = WorkspaceName::new(name).unwrap();
                prop_assert_eq!(a, b);
            }
        }
    }
}
