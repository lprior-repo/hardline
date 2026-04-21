//! Serde serialization round-trip verification tests
//!
//! Tests that all Serialize/Deserialize impls in worktree round-trip correctly.

use chrono::Utc;
use worktree::worktree::Incomplete;
use worktree::{
    AbsolutePath, BranchName, Worktree, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum,
};

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq>(
    value: &T,
    name: &str,
) -> Result<(), String> {
    let json =
        serde_json::to_string(value).map_err(|e| format!("{}: serialize error: {}", name, e))?;
    let parsed: T =
        serde_json::from_str(&json).map_err(|e| format!("{}: deserialize error: {}", name, e))?;
    if &parsed != value {
        return Err(format!(
            "{}: round-trip failed. Original: {:?}, Parsed: {:?}, JSON: {}",
            name, value, parsed, json
        ));
    }
    Ok(())
}

mod absolute_path {
    use super::*;

    #[test]
    fn roundtrip_absolute_path() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        roundtrip(&path, "AbsolutePath").unwrap();
    }

    #[test]
    fn roundtrip_absolute_path_with_nested() {
        let path = AbsolutePath::new("/var/log/nginx").unwrap();
        roundtrip(&path, "AbsolutePath nested").unwrap();
    }
}

mod worktree_name {
    use super::*;

    #[test]
    fn roundtrip_worktree_name() {
        let name = WorktreeName::new("feature-branch").unwrap();
        roundtrip(&name, "WorktreeName").unwrap();
    }

    #[test]
    fn roundtrip_worktree_name_with_underscore() {
        let name = WorktreeName::new("feature_branch_123").unwrap();
        roundtrip(&name, "WorktreeName underscore").unwrap();
    }
}

mod worktree_id {
    use super::*;

    #[test]
    fn roundtrip_worktree_id() {
        let id = WorktreeId::new_random();
        roundtrip(&id, "WorktreeId").unwrap();
    }

    #[test]
    fn roundtrip_worktree_id_from_string() {
        let id = WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        roundtrip(&id, "WorktreeId from string").unwrap();
    }
}

mod branch_name {
    use super::*;

    #[test]
    fn roundtrip_branch_name() {
        let branch = BranchName::new("main").unwrap();
        roundtrip(&branch, "BranchName main").unwrap();
    }

    #[test]
    fn roundtrip_branch_name_with_slash() {
        let branch = BranchName::new("feature/new-ui").unwrap();
        roundtrip(&branch, "BranchName with slash").unwrap();
    }

    #[test]
    fn roundtrip_branch_name_with_underscore() {
        let branch = BranchName::new("feature_branch_name").unwrap();
        roundtrip(&branch, "BranchName underscore").unwrap();
    }
}

mod worktree_state {
    use super::*;

    #[test]
    fn roundtrip_all_states() {
        for state in [
            WorktreeState::Creating,
            WorktreeState::Incomplete,
            WorktreeState::Active,
            WorktreeState::Suspended,
            WorktreeState::Removing,
            WorktreeState::Removed,
        ] {
            roundtrip(&state, &format!("WorktreeState::{:?}", state)).unwrap();
        }
    }

    #[test]
    fn worktree_state_serde_representation() {
        let state = WorktreeState::Active;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "2"); // u8 representation
    }
}

mod worktree_type_enum {
    use super::*;

    #[test]
    fn roundtrip_all_types() {
        for wt_type in [
            WorktreeTypeEnum::Development,
            WorktreeTypeEnum::Testing,
            WorktreeTypeEnum::Review,
            WorktreeTypeEnum::Debugging,
            WorktreeTypeEnum::Research,
        ] {
            roundtrip(&wt_type, &format!("WorktreeTypeEnum::{:?}", wt_type)).unwrap();
        }
    }

    #[test]
    fn worktree_type_enum_serde_representation() {
        let wt_type = WorktreeTypeEnum::Debugging;
        let json = serde_json::to_string(&wt_type).unwrap();
        assert_eq!(json, "3"); // u8 representation
    }
}

mod worktree_full {
    use super::*;
    use std::collections::HashMap;

    fn create_test_worktree() -> Worktree {
        let now = Utc::now().timestamp();
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        Worktree::uninitialized_with_metadata(
            WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/tmp").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("main").unwrap()),
            WorktreeState::Active,
            now,
            now,
            metadata,
        )
    }

    #[test]
    fn roundtrip_worktree() {
        let worktree = create_test_worktree();
        roundtrip(&worktree, "Worktree").unwrap();
    }

    #[test]
    fn roundtrip_worktree_with_metadata() {
        let worktree = create_test_worktree();
        roundtrip(&worktree, "Worktree with metadata").unwrap();
    }

    #[test]
    fn roundtrip_worktree_without_branch() {
        let now = Utc::now().timestamp();
        let worktree: Worktree<Incomplete> = Worktree::uninitialized_with_metadata(
            WorktreeId::new_random(),
            WorktreeName::new("no-branch").unwrap(),
            AbsolutePath::new("/tmp/no-branch").unwrap(),
            AbsolutePath::new("/tmp").unwrap(),
            WorktreeTypeEnum::Testing,
            None,
            WorktreeState::Incomplete,
            now,
            now,
            HashMap::new(),
        );
        roundtrip(&worktree, "Worktree without branch").unwrap();
    }

    #[test]
    fn worktree_state_field_is_skipped() {
        let worktree = create_test_worktree();
        let json = serde_json::to_string(&worktree).unwrap();
        assert!(
            !json.contains("_state"),
            "JSON should not contain _state field: {}",
            json
        );
        assert!(
            json.contains("\"worktree_state\":2"),
            "JSON should contain worktree_state: {}",
            json
        );
    }
}

mod missing_and_extra_fields {
    use super::*;

    #[test]
    fn worktree_accepts_extra_fields() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "test",
            "path": "/tmp/test",
            "parent_path": "/tmp",
            "state": 2,
            "worktree_type": 0,
            "branch": "main",
            "created_at": 1234567890,
            "updated_at": 1234567890,
            "metadata": {},
            "worktree_state": 2,
            "extra_field": "should be ignored"
        }"#;
        let result: Result<Worktree, _> = serde_json::from_str(json);
        // serde_json by default is lenient with extra fields
        assert!(result.is_ok(), "Should accept extra fields: {:?}", result);
    }

    #[test]
    fn worktreestate_from_invalid_u8_returns_none() {
        let json = "99";
        let result: Result<WorktreeState, _> = serde_json::from_str(json);
        // Should fail for invalid enum representation
        assert!(result.is_err(), "Invalid u8 for WorktreeState should fail");
    }

    #[test]
    fn worktreetype_from_invalid_u8_returns_none() {
        let json = "99";
        let result: Result<WorktreeTypeEnum, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Invalid u8 for WorktreeTypeEnum should fail"
        );
    }
}
