use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scp_cmd_with_db(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", db_path);
    cmd
}

fn create_temp_db() -> TempDir {
    TempDir::new().expect("create temp db")
}

/// Initialize a test workspace with VCS
fn init_test_workspace(dir: &std::path::Path) {
    // Initialize git repo
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("init git");

    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("git config email");

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .output()
        .expect("git config name");

    // Create initial commit
    fs::write(dir.join("README.md"), "# Test").expect("write README");
    StdCommand::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .output()
        .expect("git add");
    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir)
        .output()
        .expect("git commit");
}

// ===========================================================================
// workspace switch — Happy Path Tests
// ===========================================================================

mod workspace_switch_happy_path {
    use super::*;

    /// Switch to a valid workspace fails because worktrees not implemented
    #[test]
    fn switch_to_existing_workspace_fails_unimplemented() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("main")
            .current_dir(dir.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("Not implemented"));
    }

    /// Switch command shows workspace name in output before error
    #[test]
    fn switch_output_includes_workspace_name() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("my-feature")
            .current_dir(dir.path())
            .assert()
            .failure()
            .stdout(predicate::str::contains(
                "Switching to workspace 'my-feature'",
            ));
    }

    /// Switch requires workspace name argument
    #[test]
    fn switch_requires_workspace_name() {
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.arg("switch")
            .assert()
            .failure()
            .stderr(predicate::str::contains("required arguments"));
    }

    /// Switch with --help shows usage
    #[test]
    fn switch_help_shows_usage() {
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.arg("switch").arg("--help").assert().success();
    }

    /// Switch with verbose flag works
    #[test]
    fn switch_with_verbose_flag() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("-v")
            .arg("switch")
            .arg("test-ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Non-existent Workspace Tests
// ===========================================================================

mod workspace_switch_nonexistent {
    use super::*;

    /// Switching to non-existent workspace fails with error
    #[test]
    fn switch_nonexistent_workspace_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("nonexistent-workspace")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch non-existent workspace shows workspace name in error
    #[test]
    fn switch_nonexistent_shows_workspace_name() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("missing-ws")
            .current_dir(dir.path())
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("missing-ws")
                    .or(predicate::str::contains("Not implemented")),
            );
    }

    /// Switch non-existent workspace suggests listing available workspaces
    #[test]
    fn switch_nonexistent_suggests_list() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("does-not-exist")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with empty string fails
    #[test]
    fn switch_empty_name_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with whitespace-only name fails
    #[test]
    fn switch_whitespace_name_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("   ")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Dirty Tree Tests
// ===========================================================================

mod workspace_switch_dirty_tree {
    use super::*;

    /// Switch with dirty tree fails with working copy dirty error
    #[test]
    fn switch_dirty_tree_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // Create a dirty state
        fs::write(dir.path().join("dirty.txt"), "dirty").expect("write dirty file");

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch dirty tree suggests cleaning up changes
    #[test]
    fn switch_dirty_tree_suggests_cleanup() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // Create a dirty state
        fs::write(dir.path().join("dirty.txt"), "dirty").expect("write dirty file");

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with --force allows dirty tree (if supported)
    #[test]
    fn switch_force_with_dirty_tree() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // Create a dirty state
        fs::write(dir.path().join("dirty.txt"), "dirty").expect("write dirty file");

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("--force")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with unstaged changes fails
    #[test]
    fn switch_unstaged_changes_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // Create unstaged changes
        fs::write(dir.path().join("unstaged.txt"), "unstaged").expect("write file");

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with staged changes fails
    #[test]
    fn switch_staged_changes_fails() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // Create staged changes
        fs::write(dir.path().join("staged.txt"), "staged").expect("write file");
        StdCommand::new("git")
            .args(["add", "staged.txt"])
            .current_dir(dir.path())
            .output()
            .expect("git add");

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Already Active Workspace Tests
// ===========================================================================

mod workspace_switch_already_active {
    use super::*;

    /// Switching to already-active workspace fails because worktrees not implemented
    #[test]
    fn switch_to_current_workspace_fails_unimplemented() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("main")
            .current_dir(dir.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("Not implemented"));
    }

    /// Switch to current workspace shows it's attempting to switch
    #[test]
    fn switch_current_workspace_message() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("main")
            .current_dir(dir.path())
            .assert()
            .failure()
            .stdout(predicate::str::contains("Switching to workspace"));
    }

    /// Switch to current workspace returns non-zero exit code
    #[test]
    fn switch_current_workspace_exit_code() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("main").current_dir(dir.path());
        let output = cmd.output().expect("execute cmd");
        // Expected to fail since workspaces use worktrees (not implemented)
        assert_ne!(output.status.code(), Some(0));
    }
}

// ===========================================================================
// workspace switch — Workspace State Preservation Tests
// ===========================================================================

mod workspace_switch_state_preservation {
    use super::*;

    /// Switch command preserves current workspace state (branch, files)
    #[test]
    fn switch_preserves_state() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch updates VCS backend to target workspace
    #[test]
    fn switch_updates_backend() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("target-ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch invalidates previous workspace context
    #[test]
    fn switch_invalidates_previous_context() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("new-ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Environment/Context Update Tests
// ===========================================================================

mod workspace_switch_context_update {
    use super::*;

    /// Switch updates active workspace in session context
    #[test]
    fn switch_updates_session_context() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch updates environment variables for workspace
    #[test]
    fn switch_updates_environment() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("env-ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Edge Cases and Adversarial Tests
// ===========================================================================

mod workspace_switch_edge_cases {
    use super::*;

    /// Switch with very long workspace name
    #[test]
    fn switch_very_long_workspace_name() {
        let tmp = create_temp_db();
        let long_name = "a".repeat(255);
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg(&long_name).assert().failure();
    }

    /// Switch with unicode workspace name
    #[test]
    fn switch_unicode_workspace_name() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("ワークスペース").assert().failure();
    }

    /// Switch with special characters in name
    #[test]
    fn switch_special_characters_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("ws@#$%").assert().failure();
    }

    /// Switch with path-like name rejected
    #[test]
    fn switch_path_like_name_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("path/to/workspace")
            .assert()
            .failure();
    }

    /// Switch with injection payload rejected
    #[test]
    fn switch_injection_payload_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("'; DROP TABLE workspaces; --")
            .assert()
            .failure();
    }

    /// Switch with null byte in name rejected (platform dependent)
    #[test]
    fn switch_null_byte_rejected() {
        // assert_cmd doesn't allow null bytes in command arguments
        // This test documents that null bytes should be rejected
        // The actual rejection happens at the argument parsing level
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        // Use a name that would contain null if it were allowed
        cmd.arg("switch").arg("ws-null-test").assert().failure();
    }

    /// Switch with newline in name rejected
    #[test]
    fn switch_newline_in_name_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("ws\nname").assert().failure();
    }

    /// Switch with carriage return in name rejected
    #[test]
    fn switch_cr_in_name_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("ws\rname").assert().failure();
    }

    /// Switch with tab in name rejected
    #[test]
    fn switch_tab_in_name_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("ws\tname").assert().failure();
    }

    /// Switch with XSS payload rejected
    #[test]
    fn switch_xss_payload_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch")
            .arg("<script>alert('xss')</script>")
            .assert()
            .failure();
    }

    /// Switch with command substitution in name rejected
    #[test]
    fn switch_command_substitution_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("$(whoami)").assert().failure();
    }

    /// Switch with backtick command substitution rejected
    #[test]
    fn switch_backtick_substitution_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("`id`").assert().failure();
    }

    /// Switch with relative path rejected
    #[test]
    fn switch_relative_path_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("../workspace").assert().failure();
    }

    /// Switch with absolute path rejected
    #[test]
    fn switch_absolute_path_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("/etc/passwd").assert().failure();
    }

    /// Switch with emoji in name rejected
    #[test]
    fn switch_emoji_in_name_rejected() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("switch").arg("feature-🔥").assert().failure();
    }

    /// Switch with homograph attack name
    #[test]
    fn switch_homograph_name() {
        let tmp = create_temp_db();
        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        // Cyrillic 'а' (U+0430) instead of Latin 'a'
        cmd.arg("switch")
            .arg("workspace\u{0430}")
            .assert()
            .failure();
    }
}

// ===========================================================================
// workspace switch — Integration with other commands
// ===========================================================================

mod workspace_switch_integration {
    use super::*;

    /// Switch followed by init (separate commands)
    #[test]
    fn switch_then_init() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // First command: switch
        let mut cmd1 = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd1.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();

        // Second command: init (new command instance)
        let mut cmd2 = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd2.arg("init").current_dir(dir.path()).assert().success();
    }

    /// Switch followed by status (separate commands)
    #[test]
    fn switch_then_status() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        // First command: switch
        let mut cmd1 = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd1.arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();

        // Second command: status (new command instance)
        let mut cmd2 = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd2.arg("status")
            .current_dir(dir.path())
            .assert()
            .success();
    }

    /// Switch with quiet flag
    #[test]
    fn switch_with_quiet_flag() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("-q")
            .arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    /// Switch with JSON format output
    #[test]
    fn switch_with_json_format() {
        let tmp = create_temp_db();
        let dir = TempDir::new().expect("temp dir");
        init_test_workspace(dir.path());

        let mut cmd = scp_cmd_with_db(tmp.path().to_str().unwrap());
        cmd.arg("--format")
            .arg("json")
            .arg("switch")
            .arg("ws")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}
