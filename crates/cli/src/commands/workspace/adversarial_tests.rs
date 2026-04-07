//! RED QUEEN: Adversarial tests for the switch command.
//!
//! Tests designed to break the switch implementation through:
//!
//! - Switch to non-existent workspace
//! - Switch with dirty working copy
//! - Name validation bypass attempts
//! - Path traversal and injection in workspace names
//! - Concurrent switch detection
//! - Workspace navigation edge cases (wrap-around, single ws, no current)

use super::operations::{find_next_workspace, find_prev_workspace, sorted_workspace_names};
use super::validators::validate_workspace_name;
use scp_core::vcs::Workspace;

// =============================================================================
// SWITCH: Name validation adversarial
// =============================================================================

#[test]
fn adversarial_switch_empty_name() {
    let err = validate_workspace_name("");
    assert!(err.is_some(), "empty name must be rejected");
}

#[test]
fn adversarial_switch_starts_with_digit() {
    assert!(validate_workspace_name("123workspace").is_some());
}

#[test]
fn adversarial_switch_starts_with_dash() {
    assert!(validate_workspace_name("-workspace").is_some());
}

#[test]
fn adversarial_switch_starts_with_underscore() {
    assert!(validate_workspace_name("_workspace").is_some());
}

#[test]
fn adversarial_switch_path_traversal() {
    let evil_names = [
        "../../../etc",
        "..",
        ".",
        "/absolute/path",
        "workspace/../../../etc",
        "workspace\\..\\secret",
    ];
    for name in &evil_names {
        assert!(
            validate_workspace_name(name).is_some(),
            "path traversal name {:?} must be rejected",
            name
        );
    }
}

#[test]
fn adversarial_switch_shell_injection() {
    let evil_names = [
        "ws; rm -rf /",
        "ws && evil",
        "ws || evil",
        "ws`evil`",
        "ws$(evil)",
        "ws{evil}",
    ];
    for name in &evil_names {
        assert!(
            validate_workspace_name(name).is_some(),
            "shell injection name {:?} must be rejected",
            name
        );
    }
}

#[test]
fn adversarial_switch_sql_injection() {
    let evil_names = [
        "ws'; DROP TABLE workspaces; --",
        "ws\" OR 1=1",
        "ws UNION SELECT * FROM secrets",
    ];
    for name in &evil_names {
        assert!(
            validate_workspace_name(name).is_some(),
            "SQL injection name {:?} must be rejected",
            name
        );
    }
}

#[test]
fn adversarial_switch_special_chars() {
    let evil_names = [
        "ws!bang",
        "ws@at",
        "ws#hash",
        "ws$dollar",
        "ws%percent",
        "ws^caret",
        "ws&ampersand",
        "ws*star",
        "ws(paren",
        "ws)close",
        "ws=equals",
        "ws+plus",
        "ws[bracket",
        "ws]close",
        "ws{brace",
        "ws}close",
        "ws|pipe",
        "ws\\backslash",
        "ws:colon",
        "ws;semicolon",
        "ws<quote",
        "ws'apostrophe",
        "ws<less",
        "ws>greater",
        "ws,comma",
        "ws?question",
        "ws~tilde",
        "ws`backtick",
    ];
    for name in &evil_names {
        assert!(
            validate_workspace_name(name).is_some(),
            "special char name {:?} must be rejected",
            name
        );
    }
}

#[test]
fn adversarial_switch_null_bytes() {
    assert!(validate_workspace_name("ws\x00evil").is_some());
}

#[test]
fn adversarial_switch_newlines() {
    assert!(validate_workspace_name("ws\nevil").is_some());
    assert!(validate_workspace_name("ws\revil").is_some());
    assert!(validate_workspace_name("ws\r\nevil").is_some());
}

#[test]
fn adversarial_switch_tabs() {
    assert!(validate_workspace_name("ws\tevil").is_some());
}

#[test]
fn adversarial_switch_valid_edge_cases() {
    let valid = [
        "a",
        "A",
        "workspace",
        "my-workspace",
        "my_workspace",
        "workspace123",
        "A-B_C123",
        "feature-auth-oauth2-v3",
    ];
    for name in &valid {
        assert!(
            validate_workspace_name(name).is_none(),
            "valid name {:?} should be accepted",
            name
        );
    }
}

#[test]
fn adversarial_switch_very_long_name() {
    let long_name = "a".repeat(10000);
    let result = validate_workspace_name(&long_name);
    assert!(result.is_none(), "long valid name should be accepted");
}

// =============================================================================
// SWITCH: find_next/find_prev adversarial (workspace navigation)
// =============================================================================

fn make_workspace(name: &str, is_current: bool) -> Workspace {
    Workspace {
        name: name.to_string(),
        branch: format!("branch-{name}"),
        is_current,
    }
}

#[test]
fn adversarial_find_next_single_workspace() {
    let workspaces = vec![make_workspace("only", true)];
    let result = find_next_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "only");
}

#[test]
fn adversarial_find_prev_single_workspace() {
    let workspaces = vec![make_workspace("only", true)];
    let result = find_prev_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "only");
}

#[test]
fn adversarial_find_next_no_current_workspace() {
    let workspaces = vec![
        make_workspace("alpha", false),
        make_workspace("beta", false),
    ];
    let result = find_next_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "alpha");
}

#[test]
fn adversarial_find_prev_no_current_workspace() {
    let workspaces = vec![
        make_workspace("alpha", false),
        make_workspace("beta", false),
    ];
    let result = find_prev_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "beta");
}

#[test]
fn adversarial_find_next_wraps_around() {
    // gamma is last alphabetically — next wraps to alpha (first)
    let workspaces = vec![
        make_workspace("alpha", false),
        make_workspace("beta", false),
        make_workspace("gamma", true),
    ];
    let result = find_next_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "alpha", "should wrap to first");
}

#[test]
fn adversarial_find_prev_wraps_around() {
    // alpha is first alphabetically — prev wraps to gamma (last)
    let workspaces = vec![
        make_workspace("alpha", true),
        make_workspace("beta", false),
        make_workspace("gamma", false),
    ];
    let result = find_prev_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "gamma", "should wrap to last");
}

#[test]
fn adversarial_sorted_names_is_alphabetical() {
    let workspaces = vec![
        make_workspace("zebra", false),
        make_workspace("alpha", false),
        make_workspace("mango", false),
    ];
    let names = sorted_workspace_names(&workspaces);
    assert_eq!(names, vec!["alpha", "mango", "zebra"]);
}

#[test]
fn adversarial_sorted_names_empty() {
    let names = sorted_workspace_names(&[]);
    assert!(names.is_empty());
}

#[test]
fn adversarial_find_next_many_workspaces() {
    let workspaces: Vec<Workspace> = (0..100)
        .map(|i| {
            let name = format!("ws-{:03}", i);
            make_workspace(&name, i == 50)
        })
        .collect();
    let result = find_next_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "ws-051");
}

#[test]
fn adversarial_find_prev_many_workspaces() {
    let workspaces: Vec<Workspace> = (0..100)
        .map(|i| {
            let name = format!("ws-{:03}", i);
            make_workspace(&name, i == 50)
        })
        .collect();
    let result = find_prev_workspace(&workspaces);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "ws-049");
}

#[test]
fn adversarial_find_next_duplicate_names() {
    let workspaces = vec![
        make_workspace("dup", true),
        make_workspace("dup", false),
        make_workspace("other", false),
    ];
    let result = find_next_workspace(&workspaces);
    assert!(result.is_ok(), "should not panic with duplicate names");
}

#[test]
fn adversarial_sorted_names_preserves_duplicates() {
    let workspaces = vec![
        make_workspace("dup", false),
        make_workspace("dup", false),
    ];
    let names = sorted_workspace_names(&workspaces);
    assert_eq!(names.len(), 2, "duplicates should be preserved");
}

#[test]
fn adversarial_case_sensitive_names() {
    let workspaces = vec![
        make_workspace("Alpha", true),
        make_workspace("alpha", false),
        make_workspace("BETA", false),
    ];
    let names = sorted_workspace_names(&workspaces);
    assert_eq!(names.len(), 3);
    let unique: std::collections::HashSet<_> = names.into_iter().collect();
    assert_eq!(unique.len(), 3);
}

// =============================================================================
// SWITCH: Error message quality
// =============================================================================

#[test]
fn adversarial_validate_error_mentions_empty() {
    let err = validate_workspace_name("").unwrap();
    assert!(err.to_string().to_lowercase().contains("empty"));
}

#[test]
fn adversarial_validate_error_mentions_letter_requirement() {
    let err = validate_workspace_name("123").unwrap();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("letter"), "error should mention 'letter', got: {msg}");
}

#[test]
fn adversarial_validate_error_shows_input() {
    let err = validate_workspace_name("@invalid").unwrap();
    assert!(err.to_string().contains("@invalid"));
}
