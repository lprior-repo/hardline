//! Exhaustive proptest invariants for workspace crate value objects

use proptest::prelude::*;
use scp_workspace::domain::entities::workspace::WorkspaceId;
use scp_workspace::domain::value_objects::branch_name::BranchName;
use scp_workspace::domain::value_objects::lock_holder::LockHolder;
use scp_workspace::domain::value_objects::workspace_name::WorkspaceName;
use scp_workspace::domain::value_objects::workspace_path::WorkspacePath;

// === WorkspaceId proptests ===

proptest! {
    /// parse() accepts any non-empty string — verified with diverse Unicode,
    /// UUID-like, prefixed, and arbitrary content.
    #[test]
    fn proptest_workspace_id_parse_non_empty_succeeds(
        input in ".{1,500}",
    ) {
        let id = WorkspaceId::parse(input.clone())?;
        prop_assert_eq!(id.as_str(), &input);
    }

    /// parse() rejects the empty string.
    #[test]
    fn proptest_workspace_id_parse_empty_fails(s in ".{0}") {
        let result = WorkspaceId::parse(s);
        prop_assert!(result.is_err());
    }

    /// parse() → as_str() round-trip: the inner value is preserved exactly.
    #[test]
    fn proptest_workspace_id_parse_roundtrip(
        input in ".{1,200}",
    ) {
        let id = WorkspaceId::parse(input.clone())?;
        prop_assert_eq!(id.as_str(), &input);
        let again = WorkspaceId::parse(id.as_str().to_owned())?;
        prop_assert_eq!(again, id);
    }

    /// FromStr round-trip identity: parse via FromStr, display, re-parse.
    #[test]
    fn proptest_workspace_id_from_str_roundtrip(
        input in ".{1,200}",
    ) {
        let id: WorkspaceId = input.parse()?;
        prop_assert_eq!(id.as_str(), &input);
        let display = format!("{id}");
        prop_assert_eq!(&display, &input);
        let reparsed: WorkspaceId = display.parse()?;
        prop_assert_eq!(reparsed, id);
    }

    /// Display output equals as_str() output.
    #[test]
    fn proptest_workspace_id_display_matches_as_str(
        input in ".{1,200}",
    ) {
        let id = WorkspaceId::parse(input)?;
        prop_assert_eq!(format!("{id}"), id.as_str());
    }

    /// Debug output wraps the inner value in WorkspaceId("...").
    #[test]
    fn proptest_workspace_id_debug_format(
        input in "[a-zA-Z0-9_-]{1,100}",
    ) {
        let id = WorkspaceId::parse(input.clone())?;
        let debug = format!("{id:?}");
        prop_assert!(debug.starts_with("WorkspaceId(\""), "Debug output {:?} missing type name", debug);
        prop_assert!(debug.contains(&input), "Debug output {:?} missing inner value {:?}", debug, input);
    }

    /// Clone produces an equal value.
    #[test]
    fn proptest_workspace_id_clone_equal(
        input in ".{1,200}",
    ) {
        let a = WorkspaceId::parse(input)?;
        let b = a.clone();
        prop_assert_eq!(a, b);
    }

    /// PartialEq is reflexive (a == a) for any valid ID.
    #[test]
    fn proptest_workspace_id_eq_reflexive(
        input in ".{1,200}",
    ) {
        let id = WorkspaceId::parse(input)?;
        prop_assert_eq!(&id, &id);
    }

    /// Same input produces equal IDs; different inputs produce unequal IDs.
    #[test]
    fn proptest_workspace_id_eq_consistent(
        a in ".{1,100}",
        b in ".{1,100}",
    ) {
        let id_a1 = WorkspaceId::parse(a.clone())?;
        let id_a2 = WorkspaceId::parse(a)?;
        let id_b = WorkspaceId::parse(b)?;
        prop_assert_eq!(&id_a1, &id_a2);
        if id_a1.as_str() != id_b.as_str() {
            prop_assert_ne!(&id_a1, &id_b);
        }
    }

    /// Hash consistency: equal values have equal hashes.
    #[test]
    fn proptest_workspace_id_hash_consistency(
        input in ".{1,200}",
    ) {
        use std::collections::HashSet;
        let a = WorkspaceId::parse(input.clone())?;
        let b = WorkspaceId::parse(input)?;
        let mut set = HashSet::new();
        set.insert(a);
        prop_assert!(set.contains(&b));
    }

    /// JSON serialization round-trip preserves equality.
    #[test]
    fn proptest_workspace_id_json_roundtrip(
        input in ".{1,200}",
    ) {
        let id = WorkspaceId::parse(input)?;
        let json = serde_json::to_string(&id).unwrap();
        let decoded: WorkspaceId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id, decoded);
    }

    /// Bincode serialization round-trip preserves equality.
    #[test]
    fn proptest_workspace_id_bincode_roundtrip(
        input in ".{1,200}",
    ) {
        let id = WorkspaceId::parse(input)?;
        let encoded = bincode::serialize(&id).unwrap();
        let decoded: WorkspaceId = bincode::deserialize(&encoded).unwrap();
        prop_assert_eq!(id, decoded);
    }

    /// generate() produces unique IDs across 100 consecutive calls.
    #[test]
    fn proptest_workspace_id_generate_uniqueness(
        _seed in 0u64..10,
    ) {
        use std::collections::HashSet;
        let ids: Vec<WorkspaceId> = (0..100).map(|_| WorkspaceId::generate()).collect();
        let unique: HashSet<_> = ids.iter().collect();
        prop_assert_eq!(unique.len(), 100, "generate() produced duplicate IDs");
    }

    /// generate() always produces the ws- prefix.
    #[test]
    fn proptest_workspace_id_generate_has_prefix(
        _seed in 0u64..50,
    ) {
        let id = WorkspaceId::generate();
        prop_assert!(id.as_str().starts_with("ws-"), "generated ID {:?} lacks ws- prefix", id.as_str());
    }
}

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
