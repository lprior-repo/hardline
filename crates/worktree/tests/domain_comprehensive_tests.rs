//! Comprehensive domain tests for worktree crate
//!
//! These tests cover all error variants, boundary conditions, and edge cases.
//! Following BDD naming convention: [subject]_[outcome]_when_[condition]

use worktree::domain::{
    AbsolutePath, BranchName, Worktree, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum,
};

// ============================================================
// WorktreeName Tests
// ============================================================

mod worktree_name_tests {
    use super::*;

    #[test]
    fn worktree_name_new_valid_name_returns_ok() {
        let name = WorktreeName::new("feature-branch").unwrap();
        assert_eq!(name.as_str(), "feature-branch");
    }

    #[test]
    fn worktree_name_new_empty_returns_invalid_name_error() {
        let result = WorktreeName::new("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn worktree_name_new_with_slash_returns_invalid_name_error() {
        let result = WorktreeName::new("feature/sub");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("/"));
    }

    #[test]
    fn worktree_name_new_starts_with_dot_returns_invalid_name_error() {
        let result = WorktreeName::new(".hidden");
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().is_empty());
    }

    #[test]
    fn worktree_name_new_valid_unicode_name_returns_ok() {
        let name = WorktreeName::new("тест-ветка").unwrap();
        assert_eq!(name.as_str(), "тест-ветка");
    }

    #[test]
    fn worktree_name_new_with_underscore_returns_ok() {
        let name = WorktreeName::new("feature_branch").unwrap();
        assert_eq!(name.as_str(), "feature_branch");
    }

    #[test]
    fn worktree_name_new_with_dashes_returns_ok() {
        let name = WorktreeName::new("my-long-feature-name").unwrap();
        assert_eq!(name.as_str(), "my-long-feature-name");
    }

    #[test]
    fn worktree_name_new_with_numbers_returns_ok() {
        let name = WorktreeName::new("feature-123").unwrap();
        assert_eq!(name.as_str(), "feature-123");
    }

    #[test]
    fn worktree_name_new_boundary_one_char_returns_ok() {
        let name = WorktreeName::new("a").unwrap();
        assert_eq!(name.as_str(), "a");
    }

    #[test]
    fn worktree_name_new_boundary_long_name_returns_ok() {
        let long_name = "a".repeat(100);
        let name = WorktreeName::new(&long_name).unwrap();
        assert_eq!(name.as_str(), &long_name);
    }

    #[test]
    fn worktree_name_into_string_returns_owned_value() {
        let name = WorktreeName::new("test-worktree").unwrap();
        let owned: String = name.into();
        assert_eq!(owned, "test-worktree");
    }

    #[test]
    fn worktree_name_from_ref_to_str_returns_slice() {
        let name = WorktreeName::new("test-worktree").unwrap();
        let slice: &str = (&name).into();
        assert_eq!(slice, "test-worktree");
    }

    #[test]
    fn worktree_name_matches_same_string_returns_true() {
        let name = WorktreeName::new("my-worktree").unwrap();
        assert!(name.matches("my-worktree"));
    }

    #[test]
    fn worktree_name_matches_different_string_returns_false() {
        let name = WorktreeName::new("my-worktree").unwrap();
        assert!(!name.matches("other-worktree"));
    }

    #[test]
    fn worktree_name_matches_case_sensitive() {
        let name = WorktreeName::new("My-Worktree").unwrap();
        assert!(!name.matches("my-worktree"));
        assert!(name.matches("My-Worktree"));
    }

    #[test]
    fn worktree_name_display_impl() {
        let name = WorktreeName::new("my-worktree").unwrap();
        assert_eq!(format!("{}", name), "my-worktree");
    }

    #[test]
    fn worktree_name_clone_preserves_value() {
        let name1 = WorktreeName::new("test").unwrap();
        let name2 = name1.clone();
        assert_eq!(name1.as_str(), name2.as_str());
    }

    #[test]
    fn worktree_name_eq_impl() {
        let name1 = WorktreeName::new("test").unwrap();
        let name2 = WorktreeName::new("test").unwrap();
        assert_eq!(name1, name2);

        let name3 = WorktreeName::new("other").unwrap();
        assert_ne!(name1, name3);
    }
}

// ============================================================
// BranchName Tests
// ============================================================

mod branch_name_tests {
    use super::*;

    #[test]
    fn branch_name_new_valid_main_returns_ok() {
        let branch = BranchName::new("main").unwrap();
        assert_eq!(branch.as_str(), "main");
    }

    #[test]
    fn branch_name_new_valid_feature_returns_ok() {
        let branch = BranchName::new("feature/new-feature").unwrap();
        assert_eq!(branch.as_str(), "feature/new-feature");
    }

    #[test]
    fn branch_name_new_valid_release_returns_ok() {
        let branch = BranchName::new("release/1.0.0").unwrap();
        assert_eq!(branch.as_str(), "release/1.0.0");
    }

    #[test]
    fn branch_name_new_empty_returns_invalid_branch_error() {
        let result = BranchName::new("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn branch_name_new_starts_with_hyphen_returns_invalid_branch_error() {
        let result = BranchName::new("-feature");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hyphen"));
    }

    #[test]
    fn branch_name_new_ends_with_hyphen_returns_invalid_branch_error() {
        let result = BranchName::new("feature-");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hyphen"));
    }

    #[test]
    fn branch_name_new_starts_with_period_returns_invalid_branch_error() {
        let result = BranchName::new(".hidden");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("period"));
    }

    #[test]
    fn branch_name_new_ends_with_period_returns_invalid_branch_error() {
        let result = BranchName::new("feature.");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("period"));
    }

    #[test]
    fn branch_name_new_consecutive_periods_returns_invalid_branch_error() {
        let result = BranchName::new("feature..test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("consecutive periods"));
    }

    #[test]
    fn branch_name_new_at_symbol_returns_invalid_branch_error() {
        let result = BranchName::new("feat@ure");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid branch"));
    }

    #[test]
    fn branch_name_new_valid_master_returns_ok() {
        let branch = BranchName::new("master").unwrap();
        assert_eq!(branch.as_str(), "master");
    }

    #[test]
    fn branch_name_new_valid_develop_returns_ok() {
        let branch = BranchName::new("develop").unwrap();
        assert_eq!(branch.as_str(), "develop");
    }

    #[test]
    fn branch_name_new_valid_nested_branch_returns_ok() {
        let branch = BranchName::new("feature/team/component").unwrap();
        assert_eq!(branch.as_str(), "feature/team/component");
    }

    #[test]
    fn branch_name_new_boundary_one_char_returns_ok() {
        let branch = BranchName::new("a").unwrap();
        assert_eq!(branch.as_str(), "a");
    }

    #[test]
    fn branch_name_is_default_branch_main_returns_true() {
        let branch = BranchName::new("main").unwrap();
        assert!(branch.is_default_branch());
    }

    #[test]
    fn branch_name_is_default_branch_master_returns_true() {
        let branch = BranchName::new("master").unwrap();
        assert!(branch.is_default_branch());
    }

    #[test]
    fn branch_name_is_default_branch_feature_returns_false() {
        let branch = BranchName::new("feature/test").unwrap();
        assert!(!branch.is_default_branch());
    }
}

// ============================================================
// AbsolutePath Tests
// ============================================================

mod absolute_path_tests {
    use super::*;

    #[test]
    fn absolute_path_new_absolute_path_returns_ok() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        assert_eq!(path.as_str(), "/home/user/project");
    }

    #[test]
    fn absolute_path_new_relative_path_returns_invalid_path_error() {
        let result = AbsolutePath::new("relative/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute"));
    }

    #[test]
    fn absolute_path_new_empty_path_returns_invalid_path_error() {
        let result = AbsolutePath::new("");
        assert!(result.is_err());
    }

    #[test]
    fn absolute_path_new_root_path_returns_ok() {
        let path = AbsolutePath::new("/").unwrap();
        assert_eq!(path.as_str(), "/");
    }

    #[test]
    fn absolute_path_new_deep_path_returns_ok() {
        let path = AbsolutePath::new("/a/b/c/d/e/f/g").unwrap();
        assert_eq!(path.as_str(), "/a/b/c/d/e/f/g");
    }

    #[test]
    fn absolute_path_new_path_with_spaces_returns_ok() {
        let path = AbsolutePath::new("/home/user/my documents").unwrap();
        assert_eq!(path.as_str(), "/home/user/my documents");
    }

    #[test]
    fn absolute_path_join_child_path_returns_absolute() {
        let parent = AbsolutePath::new("/home/user").unwrap();
        let child = parent.join("project");
        assert_eq!(child.as_str(), "/home/user/project");
    }

    #[test]
    fn absolute_path_join_deep_child_path() {
        let parent = AbsolutePath::new("/home").unwrap();
        let child = parent.join("user/project/src");
        assert_eq!(child.as_str(), "/home/user/project/src");
    }

    #[test]
    fn absolute_path_parent_returns_parent_path() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.as_str(), "/home/user");
    }

    #[test]
    fn absolute_path_parent_of_root_returns_none() {
        let path = AbsolutePath::new("/").unwrap();
        let parent = path.parent();
        assert!(parent.is_none());
    }

    #[test]
    fn absolute_path_file_name_returns_filename() {
        let path = AbsolutePath::new("/home/user/project").unwrap();
        assert_eq!(path.file_name(), Some("project"));
    }

    #[test]
    fn absolute_path_file_name_of_root_returns_none() {
        let path = AbsolutePath::new("/").unwrap();
        assert!(path.file_name().is_none());
    }
}

// ============================================================
// WorktreeId Tests
// ============================================================

mod worktree_id_tests {
    use super::*;

    #[test]
    fn worktree_id_new_random_generates_unique_id() {
        let id1 = WorktreeId::new_random();
        let id2 = WorktreeId::new_random();
        assert_ne!(id1, id2);
    }

    #[test]
    fn worktree_id_from_string_valid_uuid_returns_ok() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = WorktreeId::from_string(uuid_str).unwrap();
        assert_eq!(id.as_string(), uuid_str);
    }

    #[test]
    fn worktree_id_from_string_invalid_uuid_returns_error() {
        let result = WorktreeId::from_string("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn worktree_id_from_string_invalid_format_short_returns_error() {
        let result = WorktreeId::from_string("550e8400-e29b-41d4-a716-44665544000");
        assert!(result.is_err());
    }

    #[test]
    fn worktree_id_from_string_invalid_format_long_returns_error() {
        let result = WorktreeId::from_string("550e8400-e29b-41d4-a716-4466554400000");
        assert!(result.is_err());
    }

    #[test]
    fn worktree_id_from_bytes_returns_expected_uuid() {
        let bytes = [
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
            0x00, 0x00,
        ];
        let id = WorktreeId::from_bytes(bytes);
        assert_eq!(id.as_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_as_bytes_roundtrip() {
        let original_bytes = [
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99,
        ];
        let id = WorktreeId::from_bytes(original_bytes);
        let retrieved_bytes = *id.as_bytes();
        assert_eq!(original_bytes, retrieved_bytes);
    }

    #[test]
    fn worktree_id_display_impl() {
        let id = WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format!("{}", id), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_to_string_preserves_format() {
        let id = WorktreeId::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_from_string_uppercase_uuid_returns_ok() {
        let uuid_str = "550E8400-E29B-41D4-A716-446655440000";
        let id = WorktreeId::from_string(uuid_str).unwrap();
        assert_eq!(id.as_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn worktree_id_from_string_invalid_characters_returns_error() {
        let uuid_str = "550e8400-e29b-41d4-a716-4466554400GG"; // G is invalid hex
        let result = WorktreeId::from_string(uuid_str);
        assert!(result.is_err());
    }
}

// ============================================================
// WorktreeState Tests
// ============================================================

mod worktree_state_tests {
    use super::*;

    #[test]
    fn worktree_state_from_u8_zero_returns_creating() {
        assert_eq!(WorktreeState::from_u8(0), Some(WorktreeState::Creating));
    }

    #[test]
    fn worktree_state_from_u8_two_returns_active() {
        assert_eq!(WorktreeState::from_u8(2), Some(WorktreeState::Active));
    }

    #[test]
    fn worktree_state_from_u8_five_returns_removed() {
        assert_eq!(WorktreeState::from_u8(5), Some(WorktreeState::Removed));
    }

    #[test]
    fn worktree_state_from_u8_invalid_returns_none() {
        assert_eq!(WorktreeState::from_u8(99), None);
        assert_eq!(WorktreeState::from_u8(255), None);
    }

    #[test]
    fn worktree_state_as_u8_preserves_value() {
        assert_eq!(WorktreeState::Active.as_u8(), 2);
        assert_eq!(WorktreeState::Removed.as_u8(), 5);
        assert_eq!(WorktreeState::Creating.as_u8(), 0);
    }

    #[test]
    fn worktree_state_name_returns_correct_string() {
        assert_eq!(WorktreeState::Active.name(), "Active");
        assert_eq!(WorktreeState::Removed.name(), "Removed");
        assert_eq!(WorktreeState::Suspended.name(), "Suspended");
    }

    #[test]
    fn worktree_state_is_terminal_returns_true_for_removed() {
        assert!(WorktreeState::Removed.is_terminal());
    }

    #[test]
    fn worktree_state_is_terminal_returns_false_for_active() {
        assert!(!WorktreeState::Active.is_terminal());
    }

    #[test]
    fn worktree_state_is_terminal_returns_false_for_suspended() {
        assert!(!WorktreeState::Suspended.is_terminal());
    }

    #[test]
    fn worktree_state_is_active_returns_true_for_active() {
        assert!(WorktreeState::Active.is_active());
    }

    #[test]
    fn worktree_state_is_active_returns_false_for_suspended() {
        assert!(!WorktreeState::Suspended.is_active());
    }

    #[test]
    fn worktree_state_is_active_returns_false_for_created() {
        assert!(!WorktreeState::Creating.is_active());
    }

    #[test]
    fn worktree_state_is_transient_returns_true_for_creating() {
        assert!(WorktreeState::Creating.is_transient());
    }

    #[test]
    fn worktree_state_is_transient_returns_true_for_incomplete() {
        assert!(WorktreeState::Incomplete.is_transient());
    }

    #[test]
    fn worktree_state_is_transient_returns_true_for_removing() {
        assert!(WorktreeState::Removing.is_transient());
    }

    #[test]
    fn worktree_state_is_transient_returns_false_for_active() {
        assert!(!WorktreeState::Active.is_transient());
    }

    #[test]
    fn worktree_state_is_transient_returns_false_for_removed() {
        assert!(!WorktreeState::Removed.is_transient());
    }

    #[test]
    fn worktree_state_valid_next_states_creating_returns_active_and_removed() {
        let next = WorktreeState::Creating.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&WorktreeState::Active));
        assert!(next.contains(&WorktreeState::Removed));
    }

    #[test]
    fn worktree_state_valid_next_states_active_returns_suspended_and_removing() {
        let next = WorktreeState::Active.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&WorktreeState::Suspended));
        assert!(next.contains(&WorktreeState::Removing));
    }

    #[test]
    fn worktree_state_valid_next_states_suspended_returns_active_and_removing() {
        let next = WorktreeState::Suspended.valid_next_states();
        assert_eq!(next.len(), 2);
        assert!(next.contains(&WorktreeState::Active));
        assert!(next.contains(&WorktreeState::Removing));
    }

    #[test]
    fn worktree_state_valid_next_states_removing_returns_removed() {
        let next = WorktreeState::Removing.valid_next_states();
        assert_eq!(next.len(), 1);
        assert!(next.contains(&WorktreeState::Removed));
    }

    #[test]
    fn worktree_state_valid_next_states_removed_returns_empty() {
        let next = WorktreeState::Removed.valid_next_states();
        assert!(next.is_empty());
    }

    #[test]
    fn worktree_state_can_transition_to_true_for_valid_transition() {
        assert!(WorktreeState::Creating.can_transition_to(WorktreeState::Active));
        assert!(WorktreeState::Active.can_transition_to(WorktreeState::Suspended));
    }

    #[test]
    fn worktree_state_can_transition_to_false_for_invalid_transition() {
        assert!(!WorktreeState::Creating.can_transition_to(WorktreeState::Suspended));
        assert!(!WorktreeState::Removed.can_transition_to(WorktreeState::Active));
        assert!(!WorktreeState::Active.can_transition_to(WorktreeState::Creating));
    }

    #[test]
    fn worktree_state_can_transition_to_reflexive_is_false() {
        assert!(!WorktreeState::Active.can_transition_to(WorktreeState::Active));
        assert!(!WorktreeState::Suspended.can_transition_to(WorktreeState::Suspended));
    }

    #[test]
    fn worktree_state_display_impl() {
        assert_eq!(format!("{}", WorktreeState::Active), "Active");
        assert_eq!(format!("{}", WorktreeState::Removed), "Removed");
    }

    #[test]
    fn worktree_state_from_try_from_u8_valid() {
        let state: WorktreeState = 2u8.try_into().unwrap();
        assert_eq!(state, WorktreeState::Active);
    }

    #[test]
    fn worktree_state_from_try_from_u8_invalid() {
        let result: Result<WorktreeState, _> = 99u8.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn worktree_state_into_u8_conversion() {
        let state: u8 = WorktreeState::Active.into();
        assert_eq!(state, 2);
    }
}

// ============================================================
// WorktreeTypeEnum Tests
// ============================================================

mod worktree_type_enum_tests {
    use super::*;

    #[test]
    fn worktree_type_enum_from_u8_zero_returns_development() {
        assert_eq!(
            WorktreeTypeEnum::from_u8(0),
            Some(WorktreeTypeEnum::Development)
        );
    }

    #[test]
    fn worktree_type_enum_from_u8_one_returns_testing() {
        assert_eq!(
            WorktreeTypeEnum::from_u8(1),
            Some(WorktreeTypeEnum::Testing)
        );
    }

    #[test]
    fn worktree_type_enum_from_u8_four_returns_research() {
        assert_eq!(
            WorktreeTypeEnum::from_u8(4),
            Some(WorktreeTypeEnum::Research)
        );
    }

    #[test]
    fn worktree_type_enum_from_u8_invalid_returns_none() {
        assert_eq!(WorktreeTypeEnum::from_u8(5), None);
        assert_eq!(WorktreeTypeEnum::from_u8(255), None);
    }

    #[test]
    fn worktree_type_enum_as_u8_preserves_value() {
        assert_eq!(WorktreeTypeEnum::Development.as_u8(), 0);
        assert_eq!(WorktreeTypeEnum::Testing.as_u8(), 1);
        assert_eq!(WorktreeTypeEnum::Research.as_u8(), 4);
    }

    #[test]
    fn worktree_type_enum_name_returns_correct_string() {
        assert_eq!(WorktreeTypeEnum::Development.name(), "Development");
        assert_eq!(WorktreeTypeEnum::Testing.name(), "Testing");
        assert_eq!(WorktreeTypeEnum::Review.name(), "Review");
    }

    #[test]
    fn worktree_type_enum_is_development_focused_returns_true() {
        assert!(WorktreeTypeEnum::Development.is_development_focused());
        assert!(!WorktreeTypeEnum::Testing.is_development_focused());
    }

    #[test]
    fn worktree_type_enum_is_qa_focused_returns_true() {
        assert!(WorktreeTypeEnum::Testing.is_qa_focused());
        assert!(WorktreeTypeEnum::Review.is_qa_focused());
        assert!(!WorktreeTypeEnum::Development.is_qa_focused());
    }

    #[test]
    fn worktree_type_enum_is_troubleshooting_focused_returns_true() {
        assert!(WorktreeTypeEnum::Debugging.is_troubleshooting_focused());
        assert!(WorktreeTypeEnum::Research.is_troubleshooting_focused());
        assert!(!WorktreeTypeEnum::Development.is_troubleshooting_focused());
    }

    #[test]
    fn worktree_type_enum_display_impl() {
        assert_eq!(format!("{}", WorktreeTypeEnum::Development), "Development");
        assert_eq!(format!("{}", WorktreeTypeEnum::Testing), "Testing");
    }

    #[test]
    fn worktree_type_enum_into_u8_conversion() {
        let type_enum: u8 = WorktreeTypeEnum::Development.into();
        assert_eq!(type_enum, 0);
    }
}

// ============================================================
// Worktree Domain Tests
// ============================================================

mod worktree_domain_tests {
    use super::*;

    fn create_test_worktree() -> Worktree {
        Worktree::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test-worktree").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("main").unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn worktree_new_returns_worktree_with_creating_state() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.state(), WorktreeState::Creating);
        assert_eq!(worktree.name().as_str(), "test-worktree");
        assert!(worktree.branch().is_some());
    }

    #[test]
    fn worktree_new_generates_random_id() {
        let worktree1 = create_test_worktree();
        let worktree2 = create_test_worktree();
        assert_ne!(worktree1.id(), worktree2.id());
    }

    #[test]
    fn worktree_new_with_none_branch_returns_worktree_with_none_branch() {
        let worktree = Worktree::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test-worktree").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        )
        .unwrap();
        assert!(worktree.branch().is_none());
    }

    #[test]
    fn worktree_new_with_branch_returns_worktree_with_branch() {
        let worktree = Worktree::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test-worktree").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("develop").unwrap()),
        )
        .unwrap();
        assert_eq!(worktree.branch().unwrap().as_str(), "develop");
    }

    #[test]
    fn worktree_new_timestamps_are_equal() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.created_at(), worktree.updated_at());
    }

    #[test]
    fn worktree_initialize_from_creating_returns_ok_and_sets_active() {
        let mut worktree = create_test_worktree();
        assert!(worktree.initialize().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Active);
        assert!(worktree.updated_at() >= worktree.created_at());
    }

    #[test]
    fn worktree_initialize_from_active_returns_invalid_state_transition_error() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        let result = worktree.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn worktree_suspend_from_creating_returns_invalid_state_transition_error() {
        let mut worktree = create_test_worktree();
        let result = worktree.suspend();
        assert!(result.is_err());
    }

    #[test]
    fn worktree_suspend_from_active_returns_ok_and_sets_suspended() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(worktree.suspend().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Suspended);
    }

    #[test]
    fn worktree_resume_from_suspended_returns_ok_and_sets_active() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.suspend().unwrap();
        assert!(worktree.resume().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Active);
    }

    #[test]
    fn worktree_resume_from_active_returns_invalid_state_transition_error() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        let result = worktree.resume();
        assert!(result.is_err());
    }

    #[test]
    fn worktree_mark_for_removal_from_active_returns_ok_and_sets_removing() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(worktree.mark_for_removal().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Removing);
    }

    #[test]
    fn worktree_mark_for_removal_from_suspended_returns_ok_and_sets_removing() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.suspend().unwrap();
        assert!(worktree.mark_for_removal().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Removing);
    }

    #[test]
    fn worktree_complete_removal_from_removing_returns_ok_and_sets_removed() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.mark_for_removal().unwrap();
        assert!(worktree.complete_removal().is_ok());
        assert_eq!(worktree.state(), WorktreeState::Removed);
    }

    #[test]
    fn worktree_full_removal_flow_transitions_creating_active_removing_removed() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.mark_for_removal().unwrap();
        worktree.complete_removal().unwrap();
        assert_eq!(worktree.state(), WorktreeState::Removed);
        assert!(worktree.is_removed());
    }

    #[test]
    fn worktree_add_metadata_inserts_key_value_pair() {
        let mut worktree = create_test_worktree();
        worktree.add_metadata("environment", "test");
        assert_eq!(worktree.get_metadata("environment"), Some("test"));
    }

    #[test]
    fn worktree_add_metadata_updates_timestamp() {
        let mut worktree = create_test_worktree();
        let initial_updated = worktree.updated_at();
        worktree.add_metadata("key", "value");
        assert!(worktree.updated_at() >= initial_updated);
    }

    #[test]
    fn worktree_remove_metadata_returns_old_value_and_removes_key() {
        let mut worktree = create_test_worktree();
        worktree.add_metadata("environment", "test");
        let removed = worktree.remove_metadata("environment");
        assert_eq!(removed, Some("test".to_string()));
        assert!(worktree.get_metadata("environment").is_none());
    }

    #[test]
    fn worktree_remove_metadata_nonexistent_key_returns_none() {
        let mut worktree = create_test_worktree();
        let removed = worktree.remove_metadata("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn worktree_get_metadata_nonexistent_key_returns_none() {
        let worktree = create_test_worktree();
        assert!(worktree.get_metadata("nonexistent").is_none());
    }

    #[test]
    fn worktree_all_metadata_returns_all_key_value_pairs() {
        let mut worktree = create_test_worktree();
        worktree.add_metadata("env", "test");
        worktree.add_metadata("owner", "alice");
        let metadata = worktree.all_metadata();
        assert_eq!(metadata.len(), 2);
        assert_eq!(metadata.get("env"), Some(&"test".to_string()));
        assert_eq!(metadata.get("owner"), Some(&"alice".to_string()));
    }

    #[test]
    fn worktree_is_active_returns_false_when_creating() {
        let worktree = create_test_worktree();
        assert!(!worktree.is_active());
    }

    #[test]
    fn worktree_is_active_returns_true_when_active() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(worktree.is_active());
    }

    #[test]
    fn worktree_is_removed_returns_false_when_active() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        assert!(!worktree.is_removed());
    }

    #[test]
    fn worktree_is_removed_returns_true_when_removed() {
        let mut worktree = create_test_worktree();
        worktree.initialize().unwrap();
        worktree.mark_for_removal().unwrap();
        worktree.complete_removal().unwrap();
        assert!(worktree.is_removed());
    }

    #[test]
    fn worktree_id_accessor_returns_correct_id() {
        let worktree = create_test_worktree();
        let id = worktree.id();
        assert!(!id.as_string().is_empty());
    }

    #[test]
    fn worktree_name_accessor_returns_correct_name() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.name().as_str(), "test-worktree");
    }

    #[test]
    fn worktree_path_accessor_returns_correct_path() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.path().as_str(), "/tmp/test-worktree");
    }

    #[test]
    fn worktree_parent_path_accessor_returns_correct_parent() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.parent_path().as_str(), "/home/user/project");
    }

    #[test]
    fn worktree_worktree_type_accessor_returns_correct_type() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.worktree_type(), WorktreeTypeEnum::Development);
    }

    #[test]
    fn worktree_branch_accessor_returns_some_branch() {
        let worktree = create_test_worktree();
        assert_eq!(worktree.branch().unwrap().as_str(), "main");
    }

    #[test]
    fn worktree_created_at_accessor_returns_timestamp() {
        let worktree = create_test_worktree();
        assert!(worktree.created_at() > 0);
    }

    #[test]
    fn worktree_updated_at_accessor_returns_timestamp() {
        let worktree = create_test_worktree();
        assert!(worktree.updated_at() > 0);
    }

    #[test]
    fn worktree_name_mut_accessor_allows_modification() {
        let mut worktree = create_test_worktree();
        *worktree.name_mut() = WorktreeName::new("new-name").unwrap();
        assert_eq!(worktree.name().as_str(), "new-name");
    }
}
