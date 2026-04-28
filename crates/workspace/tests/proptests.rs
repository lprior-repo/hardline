//! Exhaustive proptest invariants for workspace crate value objects

use proptest::prelude::*;
use scp_workspace::domain::value_objects::{
    branch_name::BranchName, lock_holder::LockHolder, workspace_name::WorkspaceName,
    workspace_path::WorkspacePath,
};

// === WorkspaceName proptests ===

proptest! {
    #[test]
    fn proptest_workspace_name_roundtrip(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input.clone())?;
        prop_assert_eq!(name.as_str(), &input);
    }

    #[test]
    fn proptest_workspace_name_reflexive(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input)?;
        prop_assert_eq!(&name, &name.clone());
    }

    #[test]
    fn proptest_workspace_name_rejects_invalid_chars(
        prefix in "[a-zA-Z0-9_-]{0,10}",
        bad_char in "[^a-zA-Z0-9_-]",
        suffix in "[a-zA-Z0-9_-]{0,10}",
    ) {
        let input = format!("{}{}{}", prefix, bad_char, suffix);
        if input.is_empty() || input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Ok(());
        }
        prop_assert!(WorkspaceName::new(input.clone()).is_err(), "should reject: {:?}", input);
    }

    #[test]
    fn proptest_workspace_name_too_long_rejected(
        input in "[a-zA-Z0-9_-]{256,500}",
    ) {
        prop_assert!(WorkspaceName::new(input).is_err());
    }

    #[test]
    fn proptest_workspace_name_serialization_json_roundtrip(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input)?;
        let json = serde_json::to_string(&name).unwrap();
        let deserialized: WorkspaceName = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(name, deserialized);
    }

    #[test]
    fn proptest_workspace_name_bincode_roundtrip(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        let name = WorkspaceName::new(input)?;
        let encoded = bincode::serialize(&name).unwrap();
        let decoded: WorkspaceName = bincode::deserialize(&encoded).unwrap();
        prop_assert_eq!(name, decoded);
    }

    #[test]
    fn proptest_workspace_name_hash_consistency(
        input in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,62}",
    ) {
        use std::collections::HashSet;
        let a = WorkspaceName::new(input.clone())?;
        let b = WorkspaceName::new(input)?;
        let mut set = HashSet::new();
        set.insert(a);
        prop_assert!(set.contains(&b));
    }
}

#[test]
fn proptest_workspace_name_empty_rejected() {
    assert!(WorkspaceName::new("".into()).is_err());
}

// === WorkspacePath proptests ===

proptest! {
    #[test]
    fn proptest_workspace_path_absolute_roundtrip(
        segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
    ) {
        let path_str = format!("/{}", segments.join("/"));
        let path = WorkspacePath::new(path_str.clone())?;
        prop_assert!(path.as_path().is_absolute());
        prop_assert_eq!(path.as_str(), Some(path_str.as_str()));
    }

    #[test]
    fn proptest_workspace_path_serialization_json_roundtrip(
        segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
    ) {
        let path_str = format!("/{}", segments.join("/"));
        let path = WorkspacePath::new(path_str)?;
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: WorkspacePath = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(path, deserialized);
    }

    #[test]
    fn proptest_workspace_path_bincode_roundtrip(
        segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
    ) {
        let path_str = format!("/{}", segments.join("/"));
        let path = WorkspacePath::new(path_str)?;
        let encoded = bincode::serialize(&path).unwrap();
        let decoded: WorkspacePath = bincode::deserialize(&encoded).unwrap();
        prop_assert_eq!(path, decoded);
    }

    #[test]
    fn proptest_workspace_path_equality_for_same_absolute(
        segments in proptest::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
    ) {
        let path_str = format!("/{}", segments.join("/"));
        let a = WorkspacePath::new(path_str.clone())?;
        let b = WorkspacePath::new(path_str)?;
        prop_assert_eq!(a, b);
    }

    #[test]
    fn proptest_workspace_path_empty_always_fails(s in ".{0}") {
        let result = WorkspacePath::new(s);
        prop_assert!(result.is_err());
    }
}

// === LockHolder proptests ===

proptest! {
    #[test]
    fn proptest_lock_holder_non_empty_succeeds(holder in ".{1,500}") {
        let result = LockHolder::new(holder);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn proptest_lock_holder_empty_always_fails(s in ".{0}") {
        let result = LockHolder::new(s);
        prop_assert!(result.is_err());
    }

    #[test]
    fn proptest_lock_holder_serialization_json_roundtrip(holder in "[a-zA-Z0-9_-]{1,100}") {
        let lh = LockHolder::new(holder)?;
        let json = serde_json::to_string(&lh).unwrap();
        let deserialized: LockHolder = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(lh, deserialized);
    }

    #[test]
    fn proptest_lock_holder_bincode_roundtrip(holder in "[a-zA-Z0-9_-]{1,100}") {
        let lh = LockHolder::new(holder)?;
        let encoded = bincode::serialize(&lh).unwrap();
        let decoded: LockHolder = bincode::deserialize(&encoded).unwrap();
        prop_assert_eq!(lh, decoded);
    }

    #[test]
    fn proptest_lock_holder_as_str_matches(holder in "[a-zA-Z0-9_-]{1,100}") {
        let lh = LockHolder::new(holder.clone())?;
        prop_assert_eq!(lh.as_str(), &holder);
    }

    #[test]
    fn proptest_lock_holder_equality_for_same(holder in "[a-zA-Z0-9_-]{1,100}") {
        let a = LockHolder::new(holder.clone())?;
        let b = LockHolder::new(holder)?;
        prop_assert_eq!(a, b);
    }

    #[test]
    fn proptest_lock_holder_hash_consistency(holder in "[a-zA-Z0-9_-]{1,100}") {
        use std::collections::HashSet;
        let a = LockHolder::new(holder.clone())?;
        let b = LockHolder::new(holder)?;
        let mut set = HashSet::new();
        set.insert(a);
        prop_assert!(set.contains(&b));
    }
}

// === BranchName proptests ===

proptest! {
    #[test]
    fn proptest_workspace_branch_name_roundtrip(
        input in "[a-zA-Z0-9/_.-]{1,100}",
    ) {
        let name = BranchName::new(input.clone())?;
        prop_assert_eq!(name.as_str(), &input);
    }

    #[test]
    fn proptest_workspace_branch_name_reflexive(
        input in "[a-zA-Z0-9/_.-]{1,100}",
    ) {
        let name = BranchName::new(input)?;
        prop_assert_eq!(&name, &name.clone());
    }

    #[test]
    fn proptest_workspace_branch_name_null_rejected(
        prefix in "[a-zA-Z0-9/_.-]{0,10}",
        suffix in "[a-zA-Z0-9/_.-]{0,10}",
    ) {
        let input = format!("{}{}{}", prefix, '\0', suffix);
        prop_assert!(BranchName::new(input).is_err());
    }

    #[test]
    fn proptest_workspace_branch_name_bincode_roundtrip(
        input in "[a-zA-Z0-9/_.-]{1,100}",
    ) {
        let name = BranchName::new(input)?;
        let encoded = bincode::serialize(&name).unwrap();
        let decoded: BranchName = bincode::deserialize(&encoded).unwrap();
        prop_assert_eq!(name, decoded);
    }
}

#[test]
fn proptest_workspace_branch_name_empty_rejected() {
    assert!(BranchName::new("".into()).is_err());
}
