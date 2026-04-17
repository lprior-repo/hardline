//! Branch state representation
//!
//! Can be either detached (no branch) or on a named branch.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchState {
    Detached,
    OnBranch(String),
}

impl BranchState {
    pub fn detached() -> Self {
        Self::Detached
    }

    pub fn on_branch(branch: impl Into<String>) -> Self {
        Self::OnBranch(branch.into())
    }

    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch(name) => Some(name),
        }
    }

    #[must_use]
    pub fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }
}

impl Serialize for BranchState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Detached => serializer.serialize_str("detached"),
            Self::OnBranch(name) => serializer.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for BranchState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "detached" {
            Ok(Self::Detached)
        } else {
            Ok(Self::OnBranch(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn test_detached() {
        let state = BranchState::detached();
        assert_eq!(state, BranchState::Detached);
        assert!(state.is_detached());
        assert!(state.branch_name().is_none());
    }

    #[test]
    fn test_on_branch() {
        let state = BranchState::on_branch("main");
        assert_eq!(state, BranchState::OnBranch("main".to_string()));
        assert!(!state.is_detached());
        assert_eq!(state.branch_name(), Some("main"));
    }

    #[test]
    fn test_on_branch_with_string() {
        let state = BranchState::on_branch(String::from("develop"));
        assert_eq!(state.branch_name(), Some("develop"));
    }

    // ── Query methods ────────────────────────────────────────────────────────

    #[test]
    fn test_is_detached_true() {
        assert!(BranchState::Detached.is_detached());
    }

    #[test]
    fn test_is_detached_false() {
        assert!(!BranchState::on_branch("main").is_detached());
    }

    #[test]
    fn test_branch_name_detached() {
        assert_eq!(BranchState::Detached.branch_name(), None);
    }

    #[test]
    fn test_branch_name_on_branch() {
        assert_eq!(
            BranchState::on_branch("feature/x").branch_name(),
            Some("feature/x")
        );
    }

    #[test]
    fn test_branch_name_empty_string() {
        // An empty string branch name is valid (the type does not prevent it)
        let state = BranchState::on_branch("");
        assert_eq!(state.branch_name(), Some(""));
        assert!(!state.is_detached());
    }

    // ── Equality and Hashing ─────────────────────────────────────────────────

    #[test]
    fn test_equality_same_branch() {
        let a = BranchState::on_branch("main");
        let b = BranchState::on_branch("main");
        assert_eq!(a, b);
    }

    #[test]
    fn test_equality_different_branch() {
        let a = BranchState::on_branch("main");
        let b = BranchState::on_branch("develop");
        assert_ne!(a, b);
    }

    #[test]
    fn test_equality_detached_vs_branch() {
        assert_ne!(BranchState::Detached, BranchState::on_branch("main"));
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        assert!(set.insert(BranchState::Detached));
        assert!(set.insert(BranchState::on_branch("main")));
        assert!(set.insert(BranchState::on_branch("develop")));

        // Duplicate inserts should not increase size
        assert!(!set.insert(BranchState::Detached));
        assert!(!set.insert(BranchState::on_branch("main")));

        assert_eq!(set.len(), 3);
    }

    // ── Clone ────────────────────────────────────────────────────────────────

    #[test]
    fn test_clone_detached() {
        let state = BranchState::Detached;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_clone_on_branch() {
        let state = BranchState::on_branch("feature/test");
        let cloned = state.clone();
        assert_eq!(state, cloned);
        assert_eq!(cloned.branch_name(), Some("feature/test"));
    }

    // ── Debug ────────────────────────────────────────────────────────────────

    #[test]
    fn test_debug_detached() {
        let debug_str = format!("{:?}", BranchState::Detached);
        assert!(debug_str.contains("Detached"));
    }

    #[test]
    fn test_debug_on_branch() {
        let debug_str = format!("{:?}", BranchState::on_branch("main"));
        assert!(debug_str.contains("OnBranch"));
        assert!(debug_str.contains("main"));
    }

    // ── Serialization ────────────────────────────────────────────────────────

    #[test]
    fn test_serialize_detached() {
        let json = serde_json::to_string(&BranchState::Detached).expect("serialize ok");
        assert_eq!(json, "\"detached\"");
    }

    #[test]
    fn test_serialize_on_branch() {
        let json =
            serde_json::to_string(&BranchState::on_branch("feature/test")).expect("serialize ok");
        assert_eq!(json, "\"feature/test\"");
    }

    #[test]
    fn test_serialize_on_branch_special_chars() {
        // Branch names with slashes, dots, etc.
        let state = BranchState::on_branch("feature/issue-123.fix");
        let json = serde_json::to_string(&state).expect("serialize ok");
        assert_eq!(json, "\"feature/issue-123.fix\"");
    }

    // ── Deserialization ──────────────────────────────────────────────────────

    #[test]
    fn test_deserialize_detached() {
        let state: BranchState = serde_json::from_str("\"detached\"").expect("deserialize ok");
        assert_eq!(state, BranchState::Detached);
    }

    #[test]
    fn test_deserialize_branch_name() {
        let state: BranchState = serde_json::from_str("\"main\"").expect("deserialize ok");
        assert_eq!(state, BranchState::on_branch("main"));
    }

    #[test]
    fn test_deserialize_branch_with_slash() {
        let state: BranchState =
            serde_json::from_str("\"feature/my-feature\"").expect("deserialize ok");
        assert_eq!(state, BranchState::on_branch("feature/my-feature"));
    }

    // ── Roundtrip ────────────────────────────────────────────────────────────

    #[test]
    fn test_serde_roundtrip_detached() {
        let original = BranchState::Detached;
        let json = serde_json::to_string(&original).expect("serialize ok");
        let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_on_branch() {
        let original = BranchState::on_branch("release/v2.0");
        let json = serde_json::to_string(&original).expect("serialize ok");
        let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(original, deserialized);
    }

    // ── Transitions (behavioral) ─────────────────────────────────────────────
    // Note: BranchState itself has no transition method, but we test that
    // the constructors produce the correct variant states.

    #[test]
    fn test_detached_stays_detached() {
        let state = BranchState::detached();
        let _cloned = state.clone();
        assert!(state.is_detached());
    }

    #[test]
    fn test_on_branch_name_lifecycle() {
        // Construct, query, and clone a branch state
        let state = BranchState::on_branch("feature/xyz");
        assert!(!state.is_detached());
        assert_eq!(state.branch_name(), Some("feature/xyz"));

        // Simulate switching to detached
        let new_state = BranchState::detached();
        assert_ne!(state, new_state);
        assert!(new_state.is_detached());

        // Simulate switching to another branch
        let switched = BranchState::on_branch("main");
        assert_ne!(state, switched);
        assert_eq!(switched.branch_name(), Some("main"));
    }
}
