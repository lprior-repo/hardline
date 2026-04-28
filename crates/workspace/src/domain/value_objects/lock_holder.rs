use serde::{Deserialize, Serialize};

use crate::error::WorkspaceError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockHolder(String);

impl LockHolder {
    pub fn new(holder: String) -> Result<Self, WorkspaceError> {
        if holder.is_empty() {
            return Err(WorkspaceError::InvalidLockHolder("empty holder".into()));
        }
        Ok(Self(holder))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LockHolder {
    fn default() -> Self {
        Self("system".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_holder_valid() {
        let holder = LockHolder::new("agent-42".into());
        assert!(holder.is_ok());
    }

    #[test]
    fn lock_holder_empty_fails() {
        let holder = LockHolder::new("".into());
        assert!(holder.is_err());
    }

    #[test]
    fn lock_holder_empty_error_message() {
        let result = LockHolder::new("".into());
        match result.err() {
            Some(WorkspaceError::InvalidLockHolder(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidLockHolder, got {other:?}"),
        }
    }

    #[test]
    fn lock_holder_single_char_succeeds() {
        let result = LockHolder::new("a".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "a");
    }

    #[test]
    fn lock_holder_with_spaces_succeeds() {
        // Only empty string is rejected
        let result = LockHolder::new("agent one".into());
        assert!(result.is_ok());
    }

    #[test]
    fn lock_holder_with_special_chars_succeeds() {
        let result = LockHolder::new("agent/special!@#$%".into());
        assert!(result.is_ok());
    }

    #[test]
    fn lock_holder_as_str_returns_inner() {
        let holder = LockHolder::new("system".into()).unwrap();
        assert_eq!(holder.as_str(), "system");
    }

    #[test]
    fn lock_holder_default_is_system() {
        assert_eq!(LockHolder::default().as_str(), "system");
    }

    #[test]
    fn lock_holder_equality() {
        let a = LockHolder::new("agent-1".into()).unwrap();
        let b = LockHolder::new("agent-1".into()).unwrap();
        let c = LockHolder::new("agent-2".into()).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn lock_holder_clone() {
        let holder = LockHolder::new("clone-me".into()).unwrap();
        let cloned = holder.clone();
        assert_eq!(holder, cloned);
    }

    #[test]
    fn lock_holder_hash() {
        use std::collections::HashSet;
        let a = LockHolder::new("hash-me".into()).unwrap();
        let b = LockHolder::new("hash-me".into()).unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn lock_holder_serialization_roundtrip() {
        let holder = LockHolder::new("serde-agent".into()).unwrap();
        let json = serde_json::to_string(&holder).unwrap();
        let deserialized: LockHolder = serde_json::from_str(&json).unwrap();
        assert_eq!(holder, deserialized);
    }

    #[test]
    fn lock_holder_whitespace_only_succeeds() {
        // Only empty is rejected; whitespace-only is technically non-empty
        let result = LockHolder::new("  ".into());
        assert!(result.is_ok());
    }

    // --- Additional unit tests ---

    #[test]
    fn lock_holder_empty_error_type() {
        let result = LockHolder::new("".into());
        match result.err() {
            Some(WorkspaceError::InvalidLockHolder(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected InvalidLockHolder, got {other:?}"),
        }
    }

    #[test]
    fn lock_holder_debug_format() {
        let holder = LockHolder::new("debug-agent".into()).unwrap();
        let debug_str = format!("{holder:?}");
        assert!(debug_str.contains("debug-agent"));
    }

    #[test]
    fn lock_holder_very_long_succeeds() {
        let long = "a".repeat(10_000);
        let result = LockHolder::new(long.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), &long);
    }

    #[test]
    fn lock_holder_with_newline_succeeds() {
        let result = LockHolder::new("agent\nwith-newline".into());
        assert!(result.is_ok());
    }

    #[test]
    fn lock_holder_unicode_succeeds() {
        let result = LockHolder::new("日本語".into());
        assert!(result.is_ok());
    }

    #[test]
    fn lock_holder_serialization_contains_value() {
        let holder = LockHolder::new("ser-content".into()).unwrap();
        let json = serde_json::to_string(&holder).unwrap();
        assert!(json.contains("ser-content"));
    }

    #[test]
    fn lock_holder_default_returns_valid() {
        let default = LockHolder::default();
        assert_eq!(default.as_str(), "system");
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::{prelude::*, prop_assert, prop_assert_eq};

        use super::*;

        proptest! {
            #[test]
            fn lock_holder_non_empty_always_succeeds(holder in ".{1,500}") {
                let result = LockHolder::new(holder);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn lock_holder_empty_always_fails(s in ".{0}") {
                let result = LockHolder::new(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn lock_holder_serialization_roundtrip(holder in "[a-zA-Z0-9_-]{1,100}") {
                let lh = LockHolder::new(holder).unwrap();
                let json = serde_json::to_string(&lh).unwrap();
                let deserialized: LockHolder = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(lh, deserialized);
            }

            #[test]
            fn lock_holder_as_str_matches_input(holder in "[a-zA-Z0-9_-]{1,100}") {
                let lh = LockHolder::new(holder.clone()).unwrap();
                prop_assert_eq!(lh.as_str(), holder);
            }

            #[test]
            fn lock_holder_equality_for_same_input(holder in "[a-zA-Z0-9_-]{1,100}") {
                let a = LockHolder::new(holder.clone()).unwrap();
                let b = LockHolder::new(holder).unwrap();
                prop_assert_eq!(a, b);
            }

            #[test]
            fn lock_holder_hash_consistency(holder in "[a-zA-Z0-9_-]{1,100}") {
                use std::collections::HashSet;
                let a = LockHolder::new(holder.clone()).unwrap();
                let b = LockHolder::new(holder).unwrap();
                let mut set = HashSet::new();
                set.insert(a);
                prop_assert!(set.contains(&b));
            }
        }
    }
}
