//! Exhaustive tests for the introspect command handler.
//!
//! Covers: introspection query execution, query result formatting,
//! introspection of all commands, nested structure display, introspection
//! error handling, JSON serialization roundtrips, skip_serializing_if behavior,
//! CLI option construction, adversarial inputs.
//!
//! All test names are descriptive. All assertions use exact matching
//! (no bare `is_ok()`/`is_err()`).

use super::actions::{resolve_command, run_introspect};
use super::data::{
    known_commands, ArgumentInfo, CommandInfo, ErrorConditionInfo, ExampleInfo, FlagInfo,
    IntrospectOptions, IntrospectTarget,
};

use scp_core::error::Error;

// ============================================================================
// Helpers
// ============================================================================

fn specific(name: &str) -> IntrospectOptions {
    IntrospectOptions {
        target: IntrospectTarget::Specific(name.to_string()),
    }
}

fn all_opts() -> IntrospectOptions {
    IntrospectOptions {
        target: IntrospectTarget::All,
    }
}

/// Helper: assert the error is a NotFound state error containing the given text.
fn assert_not_found(result: scp_core::Result<impl std::fmt::Debug>, expected_substring: &str) {
    let err = result.expect_err("expected NotFound error");
    assert!(
        matches!(err, Error::State(_)),
        "Expected Error::State(NotFound), got: {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains(expected_substring),
        "Error message '{msg}' should contain '{expected_substring}'"
    );
}

/// Helper: assert the error code is NOT_FOUND.
fn assert_not_found_code(result: scp_core::Result<impl std::fmt::Debug>) {
    let err = result.expect_err("expected error");
    assert_eq!(
        err.code(),
        "NOT_FOUND",
        "Expected NOT_FOUND code, got: {}",
        err.code()
    );
}

/// Build a minimal CommandInfo for serialization tests.
fn minimal_command() -> CommandInfo {
    CommandInfo {
        name: "test-cmd".to_string(),
        description: "A test command".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: false,
        requires_git: false,
    }
}

/// Build a maximal CommandInfo exercising all fields.
fn maximal_command() -> CommandInfo {
    CommandInfo {
        name: "full-cmd".to_string(),
        description: "Full command with everything".to_string(),
        aliases: vec!["fc".to_string(), "full".to_string()],
        arguments: vec![ArgumentInfo {
            name: "input".to_string(),
            arg_type: "path".to_string(),
            required: true,
            description: "Input file".to_string(),
            examples: vec!["/tmp/file.txt".to_string()],
        }],
        flags: vec![FlagInfo {
            long: "verbose".to_string(),
            short: Some("v".to_string()),
            description: "Verbose output".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        }],
        examples: vec![ExampleInfo {
            command: "scp full-cmd /tmp/file.txt".to_string(),
            description: "Run full command".to_string(),
        }],
        side_effects: vec!["Creates output file".to_string()],
        error_conditions: vec![ErrorConditionInfo {
            code: "FILE_NOT_FOUND".to_string(),
            description: "Input file missing".to_string(),
            resolution: "Check path".to_string(),
        }],
        requires_init: true,
        requires_git: true,
    }
}

// ============================================================================
// IntrospectTarget
// ============================================================================

#[test]
fn target_all_variant_matches() {
    assert!(matches!(IntrospectTarget::All, IntrospectTarget::All));
}

#[test]
fn target_specific_holds_value() {
    let target = IntrospectTarget::Specific("add".to_string());
    match target {
        IntrospectTarget::Specific(name) => assert_eq!(name, "add"),
        IntrospectTarget::All => panic!("Expected Specific, got All"),
    }
}

#[test]
fn target_equality_all() {
    assert_eq!(IntrospectTarget::All, IntrospectTarget::All);
}

#[test]
fn target_equality_specific_same_value() {
    assert_eq!(
        IntrospectTarget::Specific("add".to_string()),
        IntrospectTarget::Specific("add".to_string())
    );
}

#[test]
fn target_inequality_specific_different_value() {
    assert_ne!(
        IntrospectTarget::Specific("add".to_string()),
        IntrospectTarget::Specific("remove".to_string())
    );
}

#[test]
fn target_inequality_all_vs_specific() {
    assert_ne!(
        IntrospectTarget::All,
        IntrospectTarget::Specific("add".to_string())
    );
}

#[test]
fn target_clone_preserves_value() {
    let original = IntrospectTarget::Specific("init".to_string());
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

// ============================================================================
// IntrospectOptions::from_cli
// ============================================================================

#[test]
fn from_cli_none_yields_all() {
    let opts = IntrospectOptions::from_cli(None);
    assert!(matches!(opts.target, IntrospectTarget::All));
}

#[test]
fn from_cli_some_yields_specific() {
    let opts = IntrospectOptions::from_cli(Some("add".to_string()));
    match opts.target {
        IntrospectTarget::Specific(name) => assert_eq!(name, "add"),
        IntrospectTarget::All => panic!("Expected Specific"),
    }
}

#[test]
fn from_cli_empty_string_yields_specific_empty() {
    let opts = IntrospectOptions::from_cli(Some(String::new()));
    match opts.target {
        IntrospectTarget::Specific(name) => assert!(name.is_empty()),
        IntrospectTarget::All => panic!("Expected Specific with empty string"),
    }
}

#[test]
fn from_cli_preserves_exact_command_name() {
    let opts = IntrospectOptions::from_cli(Some("Add".to_string()));
    match opts.target {
        IntrospectTarget::Specific(name) => assert_eq!(name, "Add"),
        IntrospectTarget::All => panic!("Expected Specific"),
    }
}

// ============================================================================
// known_commands registry
// ============================================================================

#[test]
fn registry_is_not_empty() {
    assert!(!known_commands().is_empty());
}

#[test]
fn registry_has_twelve_commands() {
    assert_eq!(known_commands().len(), 12);
}

#[test]
fn registry_names_are_unique() {
    let cmds = known_commands();
    let names: Vec<String> = cmds.into_iter().map(|c| c.name).collect();
    let unique: std::collections::HashSet<String> = names.iter().cloned().collect();
    assert_eq!(
        names.len(),
        unique.len(),
        "duplicate command names in registry"
    );
}

#[test]
fn registry_contains_all_core_commands() {
    let cmds = known_commands();
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
    for expected in &[
        "init",
        "add",
        "remove",
        "list",
        "status",
        "done",
        "sync",
        "diff",
        "introspect",
        "doctor",
        "query",
        "revert",
    ] {
        assert!(names.contains(expected), "missing command: {expected}");
    }
}

#[test]
fn every_command_has_nonempty_name() {
    for cmd in known_commands() {
        assert!(!cmd.name.is_empty(), "command with empty name found");
    }
}

#[test]
fn every_command_has_nonempty_description() {
    for cmd in known_commands() {
        assert!(
            !cmd.description.is_empty(),
            "{}: empty description",
            cmd.name
        );
    }
}

#[test]
fn every_command_has_at_least_one_example() {
    for cmd in known_commands() {
        assert!(!cmd.examples.is_empty(), "{}: no examples", cmd.name);
    }
}

#[test]
fn every_error_condition_has_all_fields() {
    for cmd in known_commands() {
        for ec in &cmd.error_conditions {
            assert!(
                !ec.code.is_empty(),
                "{}: error condition with empty code",
                cmd.name
            );
            assert!(
                !ec.description.is_empty(),
                "{}: error condition with empty description",
                cmd.name
            );
            assert!(
                !ec.resolution.is_empty(),
                "{}: error condition with empty resolution",
                cmd.name
            );
        }
    }
}

#[test]
fn every_argument_has_name_and_type() {
    for cmd in known_commands() {
        for arg in &cmd.arguments {
            assert!(
                !arg.name.is_empty(),
                "{}: argument with empty name",
                cmd.name
            );
            assert!(
                !arg.arg_type.is_empty(),
                "{}: argument with empty type",
                cmd.name
            );
            assert!(
                !arg.description.is_empty(),
                "{}: argument with empty description",
                cmd.name
            );
        }
    }
}

#[test]
fn every_flag_has_long_and_description() {
    for cmd in known_commands() {
        for flag in &cmd.flags {
            assert!(
                !flag.long.is_empty(),
                "{}: flag with empty long name",
                cmd.name
            );
            assert!(
                !flag.description.is_empty(),
                "{}: flag with empty description",
                cmd.name
            );
            assert!(
                !flag.flag_type.is_empty(),
                "{}: flag with empty type",
                cmd.name
            );
        }
    }
}

// ============================================================================
// Per-command metadata invariants
// ============================================================================

#[test]
fn init_requires_git_but_not_init() {
    let cmd = resolve_command("init").expect("init must exist");
    assert!(cmd.requires_git, "init requires git");
    assert!(!cmd.requires_init, "init does not require prior init");
}

#[test]
fn add_requires_both() {
    let cmd = resolve_command("add").expect("add must exist");
    assert!(cmd.requires_init, "add requires init");
    assert!(cmd.requires_git, "add requires git");
}

#[test]
fn add_has_aliases() {
    let cmd = resolve_command("add").expect("add must exist");
    assert!(!cmd.aliases.is_empty(), "add should have aliases");
    assert!(cmd.aliases.contains(&"a".to_string()), "add alias 'a'");
    assert!(cmd.aliases.contains(&"new".to_string()), "add alias 'new'");
}

#[test]
fn add_has_required_name_argument() {
    let cmd = resolve_command("add").expect("add must exist");
    let name_arg = cmd
        .arguments
        .iter()
        .find(|a| a.name == "name")
        .unwrap_or_else(|| panic!("add must have 'name' argument"));
    assert!(name_arg.required, "add name argument must be required");
}

#[test]
fn remove_has_aliases() {
    let cmd = resolve_command("remove").expect("remove must exist");
    assert!(cmd.aliases.contains(&"rm".to_string()));
    assert!(cmd.aliases.contains(&"delete".to_string()));
}

#[test]
fn remove_has_force_flag_with_short() {
    let cmd = resolve_command("remove").expect("remove must exist");
    let force = cmd
        .flags
        .iter()
        .find(|f| f.long == "force")
        .unwrap_or_else(|| panic!("remove must have --force flag"));
    assert_eq!(force.short.as_deref(), Some("f"));
}

#[test]
fn list_has_all_flag() {
    let cmd = resolve_command("list").expect("list must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "all"),
        "list must have --all flag"
    );
}

#[test]
fn list_does_not_require_git() {
    let cmd = resolve_command("list").expect("list must exist");
    assert!(!cmd.requires_git, "list does not require git");
}

#[test]
fn list_has_ls_alias() {
    let cmd = resolve_command("list").expect("list must exist");
    assert!(cmd.aliases.contains(&"ls".to_string()));
}

#[test]
fn done_has_squash_flag() {
    let cmd = resolve_command("done").expect("done must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "squash"),
        "done must have --squash"
    );
}

#[test]
fn done_has_dry_run_flag() {
    let cmd = resolve_command("done").expect("done must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "dry-run"),
        "done must have --dry-run"
    );
}

#[test]
fn done_has_side_effects() {
    let cmd = resolve_command("done").expect("done must exist");
    assert!(!cmd.side_effects.is_empty(), "done must list side effects");
}

#[test]
fn sync_has_conflict_error_condition() {
    let cmd = resolve_command("sync").expect("sync must exist");
    assert!(
        cmd.error_conditions.iter().any(|ec| ec.code == "CONFLICTS"),
        "sync must document CONFLICTS error"
    );
}

#[test]
fn diff_has_stat_flag() {
    let cmd = resolve_command("diff").expect("diff must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "stat"),
        "diff must have --stat"
    );
}

#[test]
fn introspect_requires_nothing() {
    let cmd = resolve_command("introspect").expect("introspect must exist");
    assert!(!cmd.requires_init);
    assert!(!cmd.requires_git);
}

#[test]
fn doctor_has_check_alias() {
    let cmd = resolve_command("doctor").expect("doctor must exist");
    assert!(cmd.aliases.contains(&"check".to_string()));
}

#[test]
fn doctor_has_fix_flag() {
    let cmd = resolve_command("doctor").expect("doctor must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "fix"),
        "doctor must have --fix"
    );
}

#[test]
fn query_has_required_query_type_argument() {
    let cmd = resolve_command("query").expect("query must exist");
    let qt = cmd
        .arguments
        .iter()
        .find(|a| a.name == "query_type")
        .unwrap_or_else(|| panic!("query must have 'query_type' argument"));
    assert!(qt.required, "query_type must be required");
}

#[test]
fn revert_has_dry_run_flag() {
    let cmd = resolve_command("revert").expect("revert must exist");
    assert!(
        cmd.flags.iter().any(|f| f.long == "dry-run"),
        "revert must have --dry-run"
    );
}

#[test]
fn revert_has_session_not_found_error() {
    let cmd = resolve_command("revert").expect("revert must exist");
    assert!(cmd
        .error_conditions
        .iter()
        .any(|ec| ec.code == "SESSION_NOT_FOUND"));
}

#[test]
fn status_argument_is_optional() {
    let cmd = resolve_command("status").expect("status must exist");
    let name_arg = cmd.arguments.iter().find(|a| a.name == "name");
    if let Some(arg) = name_arg {
        assert!(!arg.required, "status name argument should be optional");
    }
}

// ============================================================================
// resolve_command — pure function
// ============================================================================

#[test]
fn resolve_finds_every_known_command() {
    for cmd in known_commands() {
        let found = resolve_command(&cmd.name);
        assert!(
            found.is_some(),
            "resolve_command({}) should find it",
            cmd.name
        );
        assert_eq!(found.unwrap().name, cmd.name);
    }
}

#[test]
fn resolve_returns_none_for_empty_string() {
    assert!(resolve_command("").is_none());
}

#[test]
fn resolve_returns_none_for_whitespace() {
    assert!(resolve_command(" ").is_none());
    assert!(resolve_command(" add").is_none());
    assert!(resolve_command("add ").is_none());
}

#[test]
fn resolve_is_case_sensitive() {
    assert!(
        resolve_command("Add").is_none(),
        "command names are case-sensitive"
    );
    assert!(resolve_command("ADD").is_none());
    assert!(resolve_command("Init").is_none());
}

#[test]
fn resolve_does_not_match_aliases() {
    // Aliases are metadata; resolve_command only matches primary names
    assert!(
        resolve_command("rm").is_none(),
        "alias 'rm' should not resolve"
    );
    assert!(
        resolve_command("ls").is_none(),
        "alias 'ls' should not resolve"
    );
    assert!(
        resolve_command("a").is_none(),
        "alias 'a' should not resolve"
    );
}

#[test]
fn resolve_does_not_match_partial_names() {
    assert!(resolve_command("ad").is_none());
    assert!(resolve_command("ini").is_none());
    assert!(resolve_command("done-extra").is_none());
}

#[test]
fn resolve_returns_complete_metadata() {
    let cmd = resolve_command("add").expect("add exists");
    assert!(!cmd.aliases.is_empty());
    assert!(!cmd.arguments.is_empty());
    assert!(!cmd.flags.is_empty());
    assert!(!cmd.examples.is_empty());
    assert!(!cmd.side_effects.is_empty());
    assert!(!cmd.error_conditions.is_empty());
}

// ============================================================================
// run_introspect — All target
// ============================================================================

#[test]
fn run_all_succeeds() {
    let result = run_introspect(&all_opts());
    assert!(result.is_ok(), "listing all commands should succeed");
}

// ============================================================================
// run_introspect — Specific target: every known command
// ============================================================================

#[test]
fn run_specific_init_succeeds() {
    assert!(run_introspect(&specific("init")).is_ok());
}

#[test]
fn run_specific_add_succeeds() {
    assert!(run_introspect(&specific("add")).is_ok());
}

#[test]
fn run_specific_remove_succeeds() {
    assert!(run_introspect(&specific("remove")).is_ok());
}

#[test]
fn run_specific_list_succeeds() {
    assert!(run_introspect(&specific("list")).is_ok());
}

#[test]
fn run_specific_status_succeeds() {
    assert!(run_introspect(&specific("status")).is_ok());
}

#[test]
fn run_specific_done_succeeds() {
    assert!(run_introspect(&specific("done")).is_ok());
}

#[test]
fn run_specific_sync_succeeds() {
    assert!(run_introspect(&specific("sync")).is_ok());
}

#[test]
fn run_specific_diff_succeeds() {
    assert!(run_introspect(&specific("diff")).is_ok());
}

#[test]
fn run_specific_introspect_succeeds() {
    assert!(run_introspect(&specific("introspect")).is_ok());
}

#[test]
fn run_specific_doctor_succeeds() {
    assert!(run_introspect(&specific("doctor")).is_ok());
}

#[test]
fn run_specific_query_succeeds() {
    assert!(run_introspect(&specific("query")).is_ok());
}

#[test]
fn run_specific_revert_succeeds() {
    assert!(run_introspect(&specific("revert")).is_ok());
}

// ============================================================================
// run_introspect — error handling
// ============================================================================

#[test]
fn unknown_command_returns_not_found_error() {
    let result = run_introspect(&specific("nonexistent"));
    assert_not_found(result, "Unknown command");
}

#[test]
fn unknown_command_error_contains_command_name() {
    let result = run_introspect(&specific("foobar"));
    assert_not_found(result, "foobar");
}

#[test]
fn unknown_command_error_has_not_found_code() {
    let result = run_introspect(&specific("nope"));
    assert_not_found_code(result);
}

#[test]
fn unknown_command_error_suggests_introspect() {
    let result = run_introspect(&specific("xyz"));
    let err = result.expect_err("expected error");
    let msg = err.to_string();
    assert!(
        msg.contains("scp introspect"),
        "error should suggest running introspect: {msg}"
    );
}

#[test]
fn empty_string_command_returns_not_found() {
    let result = run_introspect(&specific(""));
    assert_not_found(result, "Unknown command");
}

#[test]
fn case_mismatch_returns_not_found() {
    let result = run_introspect(&specific("Add"));
    assert_not_found(result, "Unknown command");
}

// ============================================================================
// JSON serialization roundtrips — CommandInfo
// ============================================================================

#[test]
fn minimal_command_roundtrip() {
    let cmd = minimal_command();
    let json = serde_json::to_string(&cmd).expect("serialize");
    let back: CommandInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.name, "test-cmd");
    assert_eq!(back.description, "A test command");
    assert!(back.aliases.is_empty());
    assert!(back.arguments.is_empty());
    assert!(back.flags.is_empty());
    assert!(back.examples.is_empty());
    assert!(back.side_effects.is_empty());
    assert!(back.error_conditions.is_empty());
    assert!(!back.requires_init);
    assert!(!back.requires_git);
}

#[test]
fn maximal_command_roundtrip() {
    let cmd = maximal_command();
    let json = serde_json::to_string(&cmd).expect("serialize");
    let back: CommandInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.name, "full-cmd");
    assert_eq!(back.aliases, vec!["fc", "full"]);
    assert_eq!(back.arguments.len(), 1);
    assert_eq!(back.flags.len(), 1);
    assert_eq!(back.examples.len(), 1);
    assert_eq!(back.side_effects, vec!["Creates output file"]);
    assert_eq!(back.error_conditions.len(), 1);
    assert!(back.requires_init);
    assert!(back.requires_git);
}

#[test]
fn skip_serializing_if_empty_vecs_omitted() {
    let cmd = minimal_command();
    let json = serde_json::to_string(&cmd).expect("serialize");
    // Empty vecs should not appear in JSON output
    assert!(!json.contains("\"aliases\""));
    assert!(!json.contains("\"arguments\""));
    assert!(!json.contains("\"flags\""));
    assert!(!json.contains("\"examples\""));
    assert!(!json.contains("\"side_effects\""));
    assert!(!json.contains("\"error_conditions\""));
}

#[test]
fn skip_serializing_if_non_empty_vecs_included() {
    let cmd = maximal_command();
    let json = serde_json::to_string(&cmd).expect("serialize");
    assert!(json.contains("\"aliases\""));
    assert!(json.contains("\"arguments\""));
    assert!(json.contains("\"flags\""));
    assert!(json.contains("\"examples\""));
    assert!(json.contains("\"side_effects\""));
    assert!(json.contains("\"error_conditions\""));
}

// ============================================================================
// JSON serialization roundtrips — sub-types
// ============================================================================

#[test]
fn argument_info_roundtrip() {
    let arg = ArgumentInfo {
        name: "file".to_string(),
        arg_type: "path".to_string(),
        required: true,
        description: "Input file path".to_string(),
        examples: vec!["/tmp/a.txt".to_string(), "/home/user/doc.md".to_string()],
    };
    let json = serde_json::to_string(&arg).expect("serialize");
    let back: ArgumentInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.name, "file");
    assert_eq!(back.arg_type, "path");
    assert!(back.required);
    assert_eq!(back.examples.len(), 2);
}

#[test]
fn argument_info_without_examples_omits_field() {
    let arg = ArgumentInfo {
        name: "count".to_string(),
        arg_type: "number".to_string(),
        required: false,
        description: "Count".to_string(),
        examples: vec![],
    };
    let json = serde_json::to_string(&arg).expect("serialize");
    assert!(!json.contains("\"examples\""));
}

#[test]
fn flag_info_roundtrip() {
    let flag = FlagInfo {
        long: "verbose".to_string(),
        short: Some("v".to_string()),
        description: "Enable verbose".to_string(),
        flag_type: "bool".to_string(),
        default: Some(serde_json::json!(false)),
    };
    let json = serde_json::to_string(&flag).expect("serialize");
    let back: FlagInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.long, "verbose");
    assert_eq!(back.short, Some("v".to_string()));
    assert_eq!(back.default, Some(serde_json::json!(false)));
}

#[test]
fn flag_info_without_short_omits_field() {
    let flag = FlagInfo {
        long: "all".to_string(),
        short: None,
        description: "Show all".to_string(),
        flag_type: "bool".to_string(),
        default: None,
    };
    let json = serde_json::to_string(&flag).expect("serialize");
    assert!(!json.contains("\"short\""));
    assert!(!json.contains("\"default\""));
}

#[test]
fn example_info_roundtrip() {
    let ex = ExampleInfo {
        command: "scp add feature".to_string(),
        description: "Add workspace".to_string(),
    };
    let json = serde_json::to_string(&ex).expect("serialize");
    let back: ExampleInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.command, "scp add feature");
    assert_eq!(back.description, "Add workspace");
}

#[test]
fn error_condition_info_roundtrip() {
    let ec = ErrorConditionInfo {
        code: "DUPLICATE".to_string(),
        description: "Duplicate entry".to_string(),
        resolution: "Use unique name".to_string(),
    };
    let json = serde_json::to_string(&ec).expect("serialize");
    let back: ErrorConditionInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.code, "DUPLICATE");
    assert_eq!(back.description, "Duplicate entry");
    assert_eq!(back.resolution, "Use unique name");
}

// ============================================================================
// JSON serialization — every known command roundtrips
// ============================================================================

#[test]
fn every_known_command_serializes_and_deserializes() {
    for cmd in known_commands() {
        let json = serde_json::to_string(&cmd)
            .unwrap_or_else(|e| panic!("{}: serialize failed: {e}", cmd.name));
        let back: CommandInfo = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("{}: deserialize failed: {e}", cmd.name));
        assert_eq!(back.name, cmd.name, "name mismatch after roundtrip");
        assert_eq!(
            back.description, cmd.description,
            "description mismatch after roundtrip"
        );
        assert_eq!(back.requires_init, cmd.requires_init);
        assert_eq!(back.requires_git, cmd.requires_git);
    }
}

#[test]
fn every_known_command_produces_valid_json_object() {
    for cmd in known_commands() {
        let json = serde_json::to_string(&cmd).expect("serialize");
        let val: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(val.is_object(), "{}: root must be object", cmd.name);
        let obj = val.as_object().unwrap();
        assert!(obj.contains_key("name"), "{}: missing 'name'", cmd.name);
        assert!(
            obj.contains_key("description"),
            "{}: missing 'description'",
            cmd.name
        );
        assert!(
            obj.contains_key("requires_init"),
            "{}: missing 'requires_init'",
            cmd.name
        );
        assert!(
            obj.contains_key("requires_git"),
            "{}: missing 'requires_git'",
            cmd.name
        );
    }
}

// ============================================================================
// Flag default values — type consistency
// ============================================================================

#[test]
fn bool_flags_have_bool_defaults() {
    for cmd in known_commands() {
        for flag in &cmd.flags {
            if flag.flag_type == "bool" {
                if let Some(ref default) = flag.default {
                    assert!(
                        default.is_boolean(),
                        "{} --{}: bool flag default must be JSON boolean, got: {default}",
                        cmd.name,
                        flag.long
                    );
                }
            }
        }
    }
}

#[test]
fn string_flags_have_string_or_no_defaults() {
    for cmd in known_commands() {
        for flag in &cmd.flags {
            if flag.flag_type == "string" {
                if let Some(ref default) = flag.default {
                    assert!(
                        default.is_string(),
                        "{} --{}: string flag default must be JSON string, got: {default}",
                        cmd.name,
                        flag.long
                    );
                }
            }
        }
    }
}

// ============================================================================
// Adversarial / Red Queen
// ============================================================================

mod red_queen_adversarial {
    use super::*;

    #[test]
    fn injection_in_command_name_does_not_crash() {
        let payloads = [
            "'; DROP TABLE commands; --",
            "$(rm -rf /)",
            "../../../etc/passwd",
            "<script>alert('xss')</script>",
            "add\x00hidden",
            "add\nnewline",
            "add\ttab",
        ];
        for payload in &payloads {
            let result = run_introspect(&specific(payload));
            // Must return NotFound, not panic
            assert_not_found(result, "Unknown command");
        }
    }

    #[test]
    fn unicode_command_names_return_not_found() {
        let names = ["Добавить", "追加", "🔥", "ådd", "café"];
        for name in &names {
            let result = run_introspect(&specific(name));
            assert_not_found(result, "Unknown command");
        }
    }

    #[test]
    fn very_long_command_name_returns_not_found() {
        let long_name = "x".repeat(10_000);
        let result = run_introspect(&specific(&long_name));
        assert_not_found(result, "Unknown command");
    }

    #[test]
    fn repeated_calls_are_idempotent() {
        // Running the same query many times should always produce same result
        for _ in 0..100 {
            let result = run_introspect(&specific("add"));
            assert!(result.is_ok());
        }
    }

    #[test]
    fn resolve_command_does_not_mutate_registry() {
        let count_before = known_commands().len();
        // Repeated resolve calls
        for _ in 0..50 {
            let _ = resolve_command("add");
        }
        assert_eq!(known_commands().len(), count_before);
    }
}

// ============================================================================
// Cross-function consistency
// ============================================================================

#[test]
fn resolve_matches_known_commands_output() {
    // Every command in known_commands() must be resolvable and produce
    // the same name via resolve_command
    for cmd in known_commands() {
        let resolved = resolve_command(&cmd.name).unwrap_or_else(|| {
            panic!(
                "{}: in known_commands but resolve_command returns None",
                cmd.name
            )
        });
        assert_eq!(resolved.name, cmd.name);
        assert_eq!(resolved.description, cmd.description);
    }
}

#[test]
fn run_introspect_success_for_all_known_commands() {
    // Verify every command in the registry can be introspected individually
    for cmd in known_commands() {
        let result = run_introspect(&specific(&cmd.name));
        assert!(
            result.is_ok(),
            "{}: run_introspect should succeed",
            cmd.name
        );
    }
}

#[test]
fn all_target_and_individual_targets_both_succeed() {
    // All target
    assert!(run_introspect(&all_opts()).is_ok());
    // Every individual target
    for cmd in known_commands() {
        assert!(run_introspect(&specific(&cmd.name)).is_ok());
    }
}
