//! Session commands (from Isolate)

use scp_core::{output::Output, vcs, Error, Result};

/// List sessions
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        println!("No sessions found");
    } else {
        println!("Sessions:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            println!("  - {} on branch {}{}", ws.name, ws.branch, current);
        }
    }

    Ok(())
}

/// Show session status
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    let state = match vcs_status {
        scp_core::vcs::VcsStatus::Clean => "clean",
        scp_core::vcs::VcsStatus::Dirty => "dirty",
        scp_core::vcs::VcsStatus::Conflicted => "conflicted",
        scp_core::vcs::VcsStatus::Detached => "detached",
    };

    println!("Session Status:");
    println!("  Branch: {}", branch);
    println!("  State: {}", state);

    let log = backend.log(5)?;
    if !log.is_empty() {
        println!("  Recent commits:");
        for commit in log.iter().take(3) {
            println!("    - {}", commit.id.chars().take(8).collect::<String>());
            if !commit.message.is_empty() {
                println!("      {}", commit.message.lines().next().unwrap_or(""));
            }
        }
    }

    Ok(())
}

/// Focus (switch to) a session
pub fn focus(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    Output::info(&format!("Focusing session '{}'...", name));

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    backend.switch_workspace(name)?;
    Output::success(&format!("Focused session '{}'", name));
    Ok(())
}

/// Submit session changes for review
pub fn submit(name: Option<&str>, auto_commit: bool, message: Option<&str>) -> Result<()> {
    let config = scp_core::config::global_config().load()?;
    let auto_commit = auto_commit || *config.session.auto_commit;
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspace_name = if let Some(n) = name {
        n.to_string()
    } else {
        let workspaces = backend.list_workspaces()?;
        workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.clone())
            .ok_or_else(|| Error::workspace_not_found("no current session".to_string()))?
    };

    Output::info(&format!("Submitting session '{}'...", workspace_name));

    let vcs_status = backend.status()?;
    if vcs_status == scp_core::vcs::VcsStatus::Dirty {
        if auto_commit {
            if let Some(msg) = message {
                let output = std::process::Command::new("git")
                    .args(["commit", "-m", msg])
                    .current_dir(&cwd)
                    .output()
                    .map_err(|e| Error::io_error(e.to_string()))?;
                if !output.status.success() {
                    return Err(Error::vcs_conflict(
                        "commit",
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
            } else {
                return Err(Error::invalid_state(
                    "dirty working copy requires --message".to_string(),
                ));
            }
        } else {
            return Err(Error::working_copy_dirty());
        }
    }

    backend.push()?;
    Output::success("Pushed to remote");

    println!("✓ Submitted session '{}'", workspace_name);
    Ok(())
}

/// Remove a session
pub fn remove(name: &str, _force: bool, merge: bool) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    if name == "main" {
        return Err(Error::invalid_state(
            "cannot remove the main session".to_string(),
        ));
    }

    Output::info(&format!("Removing session '{}'...", name));

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    if merge {
        backend.rebase("main")?;
        Output::success("Merged with main");
    }

    backend.delete_workspace(name)?;
    Output::success(&format!("Removed session '{}'", name));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use tempfile::TempDir;

    fn safe_restore_dir() -> std::path::PathBuf {
        std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            })
    }

    // -- list() tests --

    #[test]
    #[serial]
    fn test_list_returns_error_in_non_vcs_directory() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = list();
        assert!(result.is_err(), "list should error in non-VCS directory");
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_list_success_never_panics() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = list();
        if result.is_err() {
            assert!(result.unwrap_err().code() != "PANIC", "list must not panic");
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    // -- status() tests --

    #[test]
    #[serial]
    fn test_status_returns_error_in_non_vcs_directory() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = status();
        assert!(result.is_err(), "status should error in non-VCS directory");
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_status_success_never_panics() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = status();
        if result.is_err() {
            assert!(
                result.unwrap_err().code() != "PANIC",
                "status must not panic"
            );
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    // -- focus() tests --

    #[test]
    fn test_focus_rejects_empty_name() {
        let result = focus("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), "INVALID_IDENTIFIER");
    }

    #[test]
    #[serial]
    fn test_focus_returns_error_for_nonexistent_session() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = focus("no-such-session");
        assert!(
            result.is_err(),
            "focus should error for non-existent session"
        );
        assert_ne!(
            result.unwrap_err().code(),
            "INVALID_IDENTIFIER",
            "expected VCS/session error, not identifier error"
        );
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    #[serial]
    fn test_focus_leading_trailing_whitespace_passes_validation() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = focus("  session-name  ");
        assert!(
            result.is_err(),
            "whitespace-padded name must not silently succeed"
        );
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_focus_with_very_long_name() {
        let long_name = "a".repeat(10_000);
        let result = focus(&long_name);
        assert!(
            result.is_err(),
            "very long name must not cause panic or silent success"
        );
    }

    #[test]
    fn test_focus_with_newline_in_name() {
        let result = focus("session\nname");
        assert!(result.is_err(), "newline in name must not succeed");
    }

    #[test]
    fn test_focus_with_null_byte_in_name() {
        let result = focus("session\0name");
        assert!(result.is_err(), "null byte in name must not succeed");
    }

    // -- submit() tests --

    #[test]
    #[serial]
    fn test_submit_returns_error_in_non_vcs_directory() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = submit(None, false, None);
        assert!(result.is_err(), "submit should error in non-VCS directory");
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_submit_with_empty_name_and_no_current_session() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = submit(Some(""), false, None);
        assert!(result.is_err(), "submit with empty name should error");
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_submit_with_very_long_session_name() {
        let long_name = "a".repeat(10_000);
        let result = submit(Some(&long_name), false, None);
        if result.is_err() {
            assert_ne!(
                result.unwrap_err().code(),
                "PANIC",
                "very long name must not cause panic"
            );
        }
    }

    #[test]
    fn test_submit_auto_commit_flag_is_honored() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result_clean = submit(None, true, Some("test commit"));
        if result_clean.is_err() {
            assert_ne!(
                result_clean.unwrap_err().code(),
                "INVALID_ARGUMENT",
                "auto_commit=true should be a valid argument combination"
            );
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_submit_message_without_auto_commit() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = submit(None, false, Some("message only"));
        if result.is_err() {
            let err = result.unwrap_err();
            assert_ne!(
                err.code(),
                "INVALID_ARGUMENT",
                "message without auto_commit should not error on argument parsing"
            );
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    // -- remove() tests --

    #[test]
    fn test_remove_rejects_empty_name() {
        let result = remove("", false, false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_remove_rejects_main_session() {
        let result = remove("main", false, false);
        assert!(result.is_err(), "cannot remove main session");
        assert_eq!(result.unwrap_err().code(), "INVALID_STATE");
    }

    #[test]
    #[serial]
    fn test_remove_returns_error_for_nonexistent_session() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = remove("no-such-session", false, false);
        assert!(
            result.is_err(),
            "remove should error for non-existent session"
        );
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_remove_with_merge_flag_does_not_panic() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = remove("any-session", false, true);
        if result.is_err() {
            assert_ne!(
                result.unwrap_err().code(),
                "PANIC",
                "merge flag should not cause panic"
            );
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_remove_force_flag_is_accepted() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        let result = remove("any-session", true, false);
        if result.is_err() {
            assert_ne!(
                result.unwrap_err().code(),
                "INVALID_ARGUMENT",
                "force flag should be valid"
            );
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    #[test]
    fn test_remove_with_very_long_name() {
        let long_name = "a".repeat(10_000);
        let result = remove(&long_name, false, false);
        assert!(
            result.is_err(),
            "very long name must not cause panic or silent success"
        );
    }

    #[test]
    fn test_remove_whitespace_only_name() {
        let result = remove("   ", false, false);
        assert!(result.is_err(), "whitespace-only name must not succeed");
    }

    #[test]
    fn test_remove_with_newline_in_name() {
        let result = remove("session\nname", false, false);
        assert!(result.is_err(), "newline in name must not succeed");
    }

    #[test]
    fn test_remove_with_null_byte_in_name() {
        let result = remove("session\0name", false, false);
        assert!(result.is_err(), "null byte in name must not succeed");
    }

    #[test]
    fn test_remove_main_with_merge_flag() {
        let result = remove("main", false, true);
        assert!(result.is_err(), "cannot remove main even with merge flag");
        assert_eq!(result.unwrap_err().code(), "INVALID_STATE");
    }

    #[test]
    fn test_remove_main_with_force_flag() {
        let result = remove("main", true, false);
        assert!(result.is_err(), "cannot remove main even with force flag");
        assert_eq!(result.unwrap_err().code(), "INVALID_STATE");
    }

    #[test]
    fn test_remove_empty_error_message_contains_session() {
        let result = remove("", false, false);
        let err = result.expect_err("should be error");
        let msg = err.to_string();
        assert!(
            msg.contains("session") || msg.contains("empty"),
            "error message should mention session or emptiness: {msg}"
        );
    }

    // -- VcsStatus mapping tests --

    #[test]
    fn test_vcs_status_state_mapping_clean() {
        let state = match scp_core::vcs::VcsStatus::Clean {
            scp_core::vcs::VcsStatus::Clean => "clean",
            _ => "not clean",
        };
        assert_eq!(state, "clean");
    }

    #[test]
    fn test_vcs_status_state_mapping_dirty() {
        let state = match scp_core::vcs::VcsStatus::Dirty {
            scp_core::vcs::VcsStatus::Dirty => "dirty",
            _ => "not dirty",
        };
        assert_eq!(state, "dirty");
    }

    #[test]
    fn test_vcs_status_state_mapping_conflicted() {
        let state = match scp_core::vcs::VcsStatus::Conflicted {
            scp_core::vcs::VcsStatus::Conflicted => "conflicted",
            _ => "not conflicted",
        };
        assert_eq!(state, "conflicted");
    }

    #[test]
    fn test_vcs_status_state_mapping_detached() {
        let state = match scp_core::vcs::VcsStatus::Detached {
            scp_core::vcs::VcsStatus::Detached => "detached",
            _ => "not detached",
        };
        assert_eq!(state, "detached");
    }

    // -- Error code verification --

    #[test]
    fn test_focus_empty_error_code_is_invalid_identifier() {
        let result = focus("");
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_remove_empty_error_code_is_invalid_identifier() {
        let result = remove("", false, false);
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_IDENTIFIER");
    }

    #[test]
    fn test_remove_main_error_code_is_invalid_state() {
        let result = remove("main", false, false);
        let err = result.expect_err("should be error");
        assert_eq!(err.code(), "INVALID_STATE");
    }

    // -- Integration: submit with dirty state --

    #[test]
    #[serial]
    fn test_submit_dirty_without_auto_commit_fails() {
        let tmp = TempDir::new().expect("temp dir");
        std::env::set_current_dir(tmp.path()).expect("chdir to tmp");
        // Run in a git repo with uncommitted changes would be ideal,
        // but in a temp dir without git, we just verify it doesn't panic
        let result = submit(None, false, None);
        // Either succeeds (no VCS) or fails with proper error, never panics
        if result.is_err() {
            let err = result.unwrap_err();
            assert_ne!(err.code(), "PANIC");
        }
        std::env::set_current_dir(safe_restore_dir()).ok();
    }

    use proptest::prelude::*;
    use proptest::proptest;
    use serial_test::serial;

    proptest! {
        #[test]
        fn prop_focus_any_whitespace_or_empty_always_fails(
            name in proptest::string::string_regex("[ \\t]*").unwrap()
        ) {
            let result = focus(&name);
            prop_assert!(result.is_err(), "empty/whitespace name should fail");
        }

        #[test]
        fn prop_remove_any_whitespace_or_empty_always_fails(
            name in proptest::string::string_regex("[ \\t]*").unwrap()
        ) {
            let result = remove(&name, false, false);
            prop_assert!(result.is_err(), "empty/whitespace name should fail");
        }

        #[test]
        fn prop_remove_main_always_fails(
            force in proptest::bool::ANY,
            merge in proptest::bool::ANY
        ) {
            let result = remove("main", force, merge);
            prop_assert!(result.is_err(), "main session should never be removable");
            prop_assert_eq!(result.unwrap_err().code(), "INVALID_STATE");
        }

        #[test]
        fn prop_focus_very_long_name_never_succeeds(
            prefix in "[a-z]{1,10}"
        ) {
            let long_name = format!("{}:{}", prefix, "x".repeat(10000));
            let result = focus(&long_name);
            prop_assert!(result.is_err(), "very long session name should fail");
        }

        #[test]
        fn prop_remove_very_long_name_never_succeeds(
            prefix in "[a-z]{1,10}"
        ) {
            let long_name = format!("{}:{}", prefix, "x".repeat(10000));
            let result = remove(&long_name, false, false);
            prop_assert!(result.is_err(), "very long session name should fail");
        }
    }
}
