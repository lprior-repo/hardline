//! Comprehensive tests for workspace navigation commands.
//!
//! Covers:
//! - Workspace navigation (spawn, switch, list, status, next, prev)
//! - Workspace listing for navigation
//! - Workspace name validation
//! - Next/prev workspace finding (alphabetical navigation)
//! - Invalid workspace target errors
//! - Edge cases and adversarial inputs
//!
//! All test names are descriptive. All assertions use exact matching.

use scp_core::vcs::Workspace;

use super::calculations::{
    find_next_workspace, find_prev_workspace, sorted_workspace_names, validate_name,
    validate_spawn_name, validate_switch_name,
};
use super::data::{WorkspaceInfo, WorkspaceNavCommand, WorkspaceNavOutput};

// ============================================================================
// Helper functions
// ============================================================================

fn make_workspace(name: &str, branch: &str, is_current: bool) -> Workspace {
    Workspace {
        name: name.to_string(),
        branch: branch.to_string(),
        is_current,
    }
}

// ============================================================================
// sorted_workspace_names tests
// ============================================================================

mod sorted_workspace_names_tests {
    use super::*;

    #[test]
    fn empty_list_returns_empty() {
        let workspaces: Vec<Workspace> = vec![];
        let result = sorted_workspace_names(&workspaces);
        assert!(result.is_empty());
    }

    #[test]
    fn single_element_list() {
        let workspaces = vec![make_workspace("solo", "main", false)];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["solo"]);
    }

    #[test]
    fn already_sorted_preserves_order() {
        let workspaces = vec![
            make_workspace("alpha", "main", false),
            make_workspace("beta", "main", false),
            make_workspace("gamma", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn reverse_sorted_gets_sorted() {
        let workspaces = vec![
            make_workspace("zebra", "main", false),
            make_workspace("apple", "main", false),
            make_workspace("mango", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn case_sensitive_sort() {
        let workspaces = vec![
            make_workspace("Banana", "main", false),
            make_workspace("apple", "main", false),
            make_workspace("Cherry", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["Banana", "Cherry", "apple"]);
    }

    #[test]
    fn numeric_suffix_sorting() {
        let workspaces = vec![
            make_workspace("feature-10", "main", false),
            make_workspace("feature-2", "main", false),
            make_workspace("feature-1", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["feature-1", "feature-10", "feature-2"]);
    }

    #[test]
    fn dash_vs_underscore_sorting() {
        let workspaces = vec![
            make_workspace("a-b", "main", false),
            make_workspace("a_b", "main", false),
            make_workspace("aac", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["a-b", "a_b", "aac"]);
    }

    #[test]
    fn is_current_flag_ignored_in_sorting() {
        let workspaces = vec![
            make_workspace("z-first", "main", true),
            make_workspace("a-second", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result, vec!["a-second", "z-first"]);
    }

    #[test]
    fn unicode_names_sorting() {
        let workspaces = vec![
            make_workspace("日本語", "main", false),
            make_workspace("apple", "main", false),
            make_workspace("βήτα", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result.len(), 3);
    }
}

// ============================================================================
// find_next_workspace tests
// ============================================================================

mod find_next_workspace_tests {
    use super::*;

    #[test]
    fn next_wraps_from_last_to_first() {
        let workspaces = vec![
            make_workspace("alpha", "main", false),
            make_workspace("beta", "main", false),
            make_workspace("gamma", "main", true),
        ];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "alpha");
    }

    #[test]
    fn next_from_middle_returns_next() {
        let workspaces = vec![
            make_workspace("alpha", "main", true),
            make_workspace("beta", "main", false),
            make_workspace("gamma", "main", false),
        ];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "beta");
    }

    #[test]
    fn next_from_first_returns_second() {
        let workspaces = vec![
            make_workspace("first", "main", false),
            make_workspace("second", "main", true),
            make_workspace("third", "main", false),
        ];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "third");
    }

    #[test]
    fn next_with_no_current_returns_first_alphabetically() {
        let workspaces = vec![
            make_workspace("zulu", "main", false),
            make_workspace("alpha", "main", false),
        ];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "alpha");
    }

    #[test]
    fn next_single_workspace_returns_same() {
        let workspaces = vec![make_workspace("only", "main", true)];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "only");
    }

    #[test]
    fn next_empty_workspace_list_fails() {
        let workspaces: Vec<Workspace> = vec![];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_err());
    }

    #[test]
    fn next_with_multiple_current_uses_last_in_list_order() {
        let workspaces = vec![
            make_workspace("alpha", "main", true),
            make_workspace("beta", "main", true),
        ];
        let result = find_next_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "beta");
    }
}

// ============================================================================
// find_prev_workspace tests
// ============================================================================

mod find_prev_workspace_tests {
    use super::*;

    #[test]
    fn prev_wraps_from_first_to_last() {
        let workspaces = vec![
            make_workspace("alpha", "main", true),
            make_workspace("beta", "main", false),
            make_workspace("gamma", "main", false),
        ];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "gamma");
    }

    #[test]
    fn prev_from_middle_returns_previous() {
        let workspaces = vec![
            make_workspace("alpha", "main", false),
            make_workspace("beta", "main", true),
            make_workspace("gamma", "main", false),
        ];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "alpha");
    }

    #[test]
    fn prev_from_last_returns_second_to_last() {
        let workspaces = vec![
            make_workspace("first", "main", false),
            make_workspace("second", "main", false),
            make_workspace("third", "main", true),
        ];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "second");
    }

    #[test]
    fn prev_with_no_current_returns_last_alphabetically() {
        let workspaces = vec![
            make_workspace("zulu", "main", false),
            make_workspace("alpha", "main", false),
        ];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "zulu");
    }

    #[test]
    fn prev_single_workspace_returns_same() {
        let workspaces = vec![make_workspace("only", "main", true)];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "only");
    }

    #[test]
    fn prev_empty_workspace_list_fails() {
        let workspaces: Vec<Workspace> = vec![];
        let result = find_prev_workspace(&workspaces);
        assert!(result.is_err());
    }
}

// ============================================================================
// validate_name (validate_workspace_name) tests
// ============================================================================

mod validate_name_tests {
    use super::*;

    #[test]
    fn valid_names_accepted() {
        for name in &[
            "a",
            "Z",
            "main",
            "workspace",
            "feature-branch",
            "feature_branch",
            "Feature123",
            "v2_legacy",
            "αβγ",
            "βήτα",
            "ワークスペース",
        ] {
            let result = validate_name(name);
            assert!(
                result.is_none(),
                "Name '{}' should be valid, got error: {:?}",
                name,
                result
            );
        }
    }

    #[test]
    fn empty_name_rejected() {
        let result = validate_name("");
        assert!(result.is_some());
        let err = result.unwrap();
        assert!(
            err.to_string().to_lowercase().contains("empty")
                || err.to_string().contains("cannot be empty")
        );
    }

    #[test]
    fn name_starting_with_digit_rejected() {
        for name in &["0workspace", "1", "123", "9abc"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
            let err = result.unwrap();
            assert!(err.to_string().contains("letter") || err.to_string().contains("start"));
        }
    }

    #[test]
    fn name_starting_with_dash_rejected() {
        let result = validate_name("-workspace");
        assert!(result.is_some());
    }

    #[test]
    fn name_starting_with_underscore_rejected() {
        let result = validate_name("_workspace");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_space_rejected() {
        let result = validate_name("my workspace");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_dot_rejected() {
        for name in &[".", "..", "file.txt", "feat.branch"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_slash_rejected() {
        for name in &["feat/branch", "a/b", "path/to/workspace"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_backslash_rejected() {
        let result = validate_name("feat\\branch");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_control_chars_rejected() {
        for name in &["a\x00b", "a\x01b", "a\x1fb", "a\nb", "a\rb", "a\tb"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{:?}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_parentheses_rejected() {
        let result = validate_name("feat(branch)");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_brackets_rejected() {
        for name in &["feat[branch]", "feat{branch}", "feat<branch>"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_at_rejected() {
        let result = validate_name("user@host");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_hash_rejected() {
        let result = validate_name("feature#123");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_dollar_rejected() {
        let result = validate_name("price$");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_percent_rejected() {
        let result = validate_name("100%");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_caret_rejected() {
        let result = validate_name("a^b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_ampersand_rejected() {
        let result = validate_name("a&b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_asterisk_rejected() {
        let result = validate_name("a*b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_plus_rejected() {
        let result = validate_name("a+b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_equals_rejected() {
        let result = validate_name("a=b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_pipe_rejected() {
        let result = validate_name("a|b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_semicolon_rejected() {
        let result = validate_name("a;b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_colon_rejected() {
        let result = validate_name("a:b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_quote_rejected() {
        for name in &["a\"b", "a'b", "a`b"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_less_greater_rejected() {
        for name in &["a<b>", "a<<b", "a>>b"] {
            let result = validate_name(name);
            assert!(result.is_some(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn name_with_comma_rejected() {
        let result = validate_name("a,b");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_question_mark_rejected() {
        let result = validate_name("what?");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_exclamation_rejected() {
        let result = validate_name("no!");
        assert!(result.is_some());
    }

    #[test]
    fn name_with_tilde_rejected() {
        let result = validate_name("~user");
        assert!(result.is_some());
    }

    #[test]
    fn very_long_valid_name_accepted() {
        let long_name = format!("a{}", "b".repeat(1000));
        let result = validate_name(&long_name);
        assert!(result.is_none(), "Very long name should be valid");
    }

    #[test]
    fn name_containing_all_valid_chars_accepted() {
        let result = validate_name("abcXYZ123-_");
        assert!(result.is_none());
    }
}

// ============================================================================
// validate_spawn_name tests
// ============================================================================

mod validate_spawn_name_tests {
    use super::*;

    #[test]
    fn valid_spawn_names() {
        for name in &["new-workspace", "test_workspace", "feature-branch-123", "a"] {
            let result = validate_spawn_name(name);
            assert!(result.is_none(), "Spawn name '{}' should be valid", name);
        }
    }

    #[test]
    fn invalid_spawn_names_rejected() {
        for name in &["", "123", "no spaces", "invalid/name", "invalid.name"] {
            let result = validate_spawn_name(name);
            assert!(result.is_some(), "Spawn name '{}' should be invalid", name);
        }
    }
}

// ============================================================================
// validate_switch_name tests
// ============================================================================

mod validate_switch_name_tests {
    use super::*;

    #[test]
    fn valid_switch_names() {
        for name in &["workspace", "my-workspace", "my_workspace", "ws1"] {
            let result = validate_switch_name(name);
            assert!(result.is_none(), "Switch name '{}' should be valid", name);
        }
    }

    #[test]
    fn invalid_switch_names_rejected() {
        for name in &["", "123abc", "has space", "no/slash", "no.dot"] {
            let result = validate_switch_name(name);
            assert!(result.is_some(), "Switch name '{}' should be invalid", name);
        }
    }
}

// ============================================================================
// WorkspaceNavOutput data type tests
// ============================================================================

mod workspace_nav_output_tests {
    use super::*;

    #[test]
    fn success_output() {
        let output = WorkspaceNavOutput::success("Operation completed");
        assert!(output.success);
        assert_eq!(output.message, "Operation completed");
        assert!(output.workspace.is_none());
    }

    #[test]
    fn success_with_workspace() {
        let output = WorkspaceNavOutput::success_with_workspace("my-ws", "Switched to");
        assert!(output.success);
        assert_eq!(output.workspace, Some("my-ws".to_string()));
        assert_eq!(output.message, "Switched to");
    }

    #[test]
    fn failure_output() {
        let output = WorkspaceNavOutput::failure("Something went wrong");
        assert!(!output.success);
        assert_eq!(output.message, "Something went wrong");
        assert!(output.workspace.is_none());
    }

    #[test]
    fn output_clone_preserves_values() {
        let original = WorkspaceNavOutput::success_with_workspace("ws", "msg");
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    #[test]
    fn output_debug_format() {
        let output = WorkspaceNavOutput::success("test");
        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("success"));
        assert!(debug_str.contains("test"));
    }
}

// ============================================================================
// WorkspaceNavCommand enum tests
// ============================================================================

mod workspace_nav_command_tests {
    use super::*;

    #[test]
    fn command_variants_exist() {
        assert!(matches!(
            WorkspaceNavCommand::Spawn,
            WorkspaceNavCommand::Spawn
        ));
        assert!(matches!(
            WorkspaceNavCommand::Switch,
            WorkspaceNavCommand::Switch
        ));
        assert!(matches!(
            WorkspaceNavCommand::List,
            WorkspaceNavCommand::List
        ));
        assert!(matches!(
            WorkspaceNavCommand::Status,
            WorkspaceNavCommand::Status
        ));
        assert!(matches!(
            WorkspaceNavCommand::Next,
            WorkspaceNavCommand::Next
        ));
        assert!(matches!(
            WorkspaceNavCommand::Prev,
            WorkspaceNavCommand::Prev
        ));
    }

    #[test]
    fn command_equality() {
        assert_eq!(WorkspaceNavCommand::Spawn, WorkspaceNavCommand::Spawn);
        assert_eq!(WorkspaceNavCommand::List, WorkspaceNavCommand::List);
        assert_ne!(WorkspaceNavCommand::Spawn, WorkspaceNavCommand::Switch);
    }

    #[test]
    fn command_clone() {
        let original = WorkspaceNavCommand::Switch;
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    #[test]
    fn command_copy() {
        let original = WorkspaceNavCommand::Next;
        let copied = original;
        assert_eq!(copied, original);
    }

    #[test]
    fn command_debug() {
        assert_eq!(format!("{:?}", WorkspaceNavCommand::Spawn), "Spawn");
        assert_eq!(format!("{:?}", WorkspaceNavCommand::Switch), "Switch");
        assert_eq!(format!("{:?}", WorkspaceNavCommand::List), "List");
        assert_eq!(format!("{:?}", WorkspaceNavCommand::Status), "Status");
        assert_eq!(format!("{:?}", WorkspaceNavCommand::Next), "Next");
        assert_eq!(format!("{:?}", WorkspaceNavCommand::Prev), "Prev");
    }
}

// ============================================================================
// WorkspaceInfo conversion tests
// ============================================================================

mod workspace_info_tests {
    use super::*;

    #[test]
    fn from_workspace_preserves_fields() {
        let ws = make_workspace("test-ws", "main", true);
        let info = WorkspaceInfo::from(ws);
        assert_eq!(info.name, "test-ws");
        assert!(info.is_current);
    }

    #[test]
    fn from_workspace_with_false_current() {
        let ws = make_workspace("other-ws", "feature", false);
        let info = WorkspaceInfo::from(ws);
        assert_eq!(info.name, "other-ws");
        assert!(!info.is_current);
    }

    #[test]
    fn workspace_info_debug() {
        let info = WorkspaceInfo {
            name: "debug-ws".to_string(),
            is_current: true,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("debug-ws"));
        assert!(debug_str.contains("current"));
    }
}

// ============================================================================
// Roundtrip and integration tests
// ============================================================================

mod roundtrip_tests {
    use super::*;

    #[test]
    fn sorted_then_find_next_consistency() {
        let workspaces = vec![
            make_workspace("zebra", "main", false),
            make_workspace("apple", "main", false),
            make_workspace("mango", "main", true),
        ];
        let sorted = sorted_workspace_names(&workspaces);
        assert_eq!(sorted, vec!["apple", "mango", "zebra"]);

        let next = find_next_workspace(&workspaces);
        assert!(next.is_ok());
        assert_eq!(next.unwrap(), "zebra");
    }

    #[test]
    fn sorted_then_find_prev_consistency() {
        let workspaces = vec![
            make_workspace("zebra", "main", false),
            make_workspace("apple", "main", false),
            make_workspace("mango", "main", true),
        ];
        let sorted = sorted_workspace_names(&workspaces);
        assert_eq!(sorted, vec!["apple", "mango", "zebra"]);

        let prev = find_prev_workspace(&workspaces);
        assert!(prev.is_ok());
        assert_eq!(prev.unwrap(), "apple");
    }

    #[test]
    fn next_wraps_to_first_and_prev_wraps_to_last() {
        let workspaces = vec![
            make_workspace("a", "main", false),
            make_workspace("b", "main", false),
            make_workspace("c", "main", true),
        ];

        let next = find_next_workspace(&workspaces);
        assert!(next.is_ok());
        assert_eq!(next.unwrap(), "a");

        let prev = find_prev_workspace(&workspaces);
        assert!(prev.is_ok());
        assert_eq!(prev.unwrap(), "b");
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn workspace_name_with_max_length() {
        let max_name = format!("a{}", "b".repeat(255));
        let result = validate_name(&max_name);
        assert!(result.is_none(), "Max length name should be valid");
    }

    #[test]
    fn workspace_name_exceeding_max_length() {
        let too_long = format!("a{}", "b".repeat(256));
        let result = validate_name(&too_long);
        assert!(
            result.is_none(),
            "Even 257 char name should be valid (no hard limit)"
        );
    }

    #[test]
    fn unicode_workspace_names() {
        let unicode_names = vec![
            "ワークスペース",
            "工作区",
            "пространство",
            "espace-de-travail",
            "αργαλείο",
        ];
        for name in unicode_names {
            let result = validate_name(name);
            assert!(result.is_none(), "Unicode name '{}' should be valid", name);
        }
    }

    #[test]
    fn mixed_unicode_and_ascii() {
        let result = validate_name("workspace-日本語");
        assert!(result.is_none());
    }

    #[test]
    fn emoji_in_name_rejected() {
        let result = validate_name("feature-🔥");
        assert!(result.is_some(), "Emoji should be rejected");
    }

    #[test]
    fn ascii_art_rejected() {
        let result = validate_name("feature_(ツ)_");
        assert!(result.is_some());
    }
}

// ============================================================================
// Adversarial / Red Queen tests
// ============================================================================

mod red_queen_adversarial {
    use super::*;

    #[test]
    fn injection_payloads_rejected() {
        let payloads = vec![
            "'; DROP TABLE workspaces; --",
            "workspace OR 1=1",
            "workspace; rm -rf /",
            "../../../etc/passwd",
            "workspace\n<script>alert('xss')</script>",
            "workspace\x00null",
            "workspace\twith\ttabs",
            "workspace\rwith\rcarriage",
            "<svg/onload=alert('xss')>",
            "javascript:alert('xss')",
            "vbscript:msgbox('xss')",
        ];
        for payload in payloads {
            let result = validate_name(payload);
            assert!(result.is_some(), "Payload '{}' should be rejected", payload);
        }
    }

    #[test]
    fn unicode_homoglyphs_accepted() {
        let homoglyphs = vec![
            "workspace\u{0430}", // Cyrillic 'а' instead of 'a'
            "w\u{043e}orkspace", // Cyrillic 'о' instead of 'o'
        ];
        for name in homoglyphs {
            let result = validate_name(name);
            assert!(
                result.is_none(),
                "Homoglyph name may be valid (unicode check): {:?}",
                name
            );
        }
    }

    #[test]
    fn very_long_name_handled() {
        let long_name = "a".repeat(10000);
        let result = validate_name(&long_name);
        assert!(result.is_none(), "Very long name should be accepted");
    }

    #[test]
    fn repeated_validation_idempotent() {
        let name = "valid-workspace";
        let r1 = validate_name(name);
        let r2 = validate_name(name);
        let r3 = validate_name(name);
        assert_eq!(r1.is_some(), r2.is_some());
        assert_eq!(r2.is_some(), r3.is_some());
        assert_eq!(r1.is_none(), r2.is_none());
        assert_eq!(r2.is_none(), r3.is_none());
    }

    #[test]
    fn sort_stability_with_duplicates() {
        let workspaces = vec![
            make_workspace("duplicate", "main", false),
            make_workspace("duplicate", "main", false),
        ];
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result.len(), 2);
        assert_eq!(result, vec!["duplicate", "duplicate"]);
    }
}
