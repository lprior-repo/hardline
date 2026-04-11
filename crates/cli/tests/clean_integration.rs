//! Integration tests for the `clean` CLI command.
//!
//! Tests the full CLI pipeline: argument parsing → handler → git operations.
//! Uses real git repositories with worktrees for end-to-end verification.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `Command` for the CLI binary.
fn scp_cli() -> Command {
    Command::cargo_bin("scp-cli").expect("failed to find scp-cli binary")
}

/// Initialize a git repo with an initial commit at the given directory.
fn init_git_repo(dir: &std::path::Path) {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init");

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

    // Initial commit so HEAD exists
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

/// Create a git worktree branch + worktree at `worktree_path` from `repo_dir`.
fn create_worktree(repo_dir: &std::path::Path, branch_name: &str, worktree_path: &std::path::Path) {
    let status = StdCommand::new("git")
        .args([
            "worktree",
            "add",
            worktree_path.to_str().unwrap(),
            "-b",
            branch_name,
        ])
        .current_dir(repo_dir)
        .status()
        .expect("git worktree add");
    assert!(status.success(), "git worktree add failed");
}

/// Remove a worktree directory from disk (simulating stale state) without
/// running `git worktree remove`, so git still tracks it.
fn remove_dir_recursive(dir: &std::path::Path) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove worktree dir");
    }
}

// ===========================================================================
// clean — Help & Argument Validation
// ===========================================================================

mod clean_help_and_args {
    use super::*;

    #[test]
    fn clean_help_shows_usage() {
        scp_cli()
            .args(["workspace", "clean"])
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Remove stale"))
            .stdout(predicate::str::contains("--dry-run"))
            .stdout(predicate::str::contains("--force"))
            .stdout(predicate::str::contains("--verbose"));
    }

    #[test]
    fn clean_no_args_in_git_repo_succeeds() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success();
    }

    #[test]
    fn clean_outside_git_repo_fails() {
        let dir = TempDir::new().expect("temp dir");
        // No git init — should fail because it's not a git repository

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// clean — No Stale Sessions (happy path, nothing to clean)
// ===========================================================================

mod clean_no_stale {
    use super::*;

    #[test]
    fn clean_repo_with_no_worktrees_reports_no_stale() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn clean_repo_with_only_main_worktree() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Only the main worktree exists — nothing stale
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn clean_verbose_no_stale() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn clean_force_no_stale() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--force")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }
}

// ===========================================================================
// clean — Dry-Run Mode
// ===========================================================================

mod clean_dry_run {
    use super::*;

    #[test]
    fn dry_run_lists_stale_without_removing() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());
        // We need a stable path — use a child of the main temp dir
        let worktree_path = dir.path().join(".worktrees").join("feature-x");
        fs::create_dir_all(&worktree_path).expect("create worktree parent");

        create_worktree(dir.path(), "feature-x", &worktree_path);

        // Verify worktree exists
        assert!(worktree_path.exists());

        // Remove the directory to make it stale
        remove_dir_recursive(&worktree_path);
        assert!(!worktree_path.exists());

        // Dry run should list the stale session
        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("dry-run"))
            .stdout(predicate::str::contains("feature-x"));

        // After dry-run, the worktree reference should still exist in git
        let output = StdCommand::new("git")
            .args(["worktree", "list"])
            .current_dir(dir.path())
            .output()
            .expect("git worktree list");
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(
            listing.contains("feature-x"),
            "worktree should still be listed after dry-run: {listing}"
        );
    }

    #[test]
    fn dry_run_no_stale_reports_none() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn dry_run_verbose_lists_sessions() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let worktree_path = dir.path().join(".worktrees").join("stale-branch");
        fs::create_dir_all(&worktree_path).expect("create parent");

        create_worktree(dir.path(), "stale-branch", &worktree_path);
        remove_dir_recursive(&worktree_path);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("dry-run"))
            .stdout(predicate::str::contains("stale-branch"));
    }
}

// ===========================================================================
// clean — Actual Cleanup (prune stale worktrees)
// ===========================================================================

mod clean_execution {
    use super::*;

    #[test]
    fn clean_removes_stale_worktree() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let worktree_path = dir.path().join(".worktrees").join("prune-me");
        fs::create_dir_all(&worktree_path).expect("create parent");

        create_worktree(dir.path(), "prune-me", &worktree_path);
        assert!(worktree_path.exists());

        // Make stale by removing directory
        remove_dir_recursive(&worktree_path);
        assert!(!worktree_path.exists());

        // Run clean
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));

        // Verify git no longer lists the stale worktree
        let output = StdCommand::new("git")
            .args(["worktree", "list"])
            .current_dir(dir.path())
            .output()
            .expect("git worktree list");
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(
            !listing.contains("prune-me"),
            "stale worktree should be gone after clean: {listing}"
        );
    }

    #[test]
    fn clean_verbose_shows_removed_sessions() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let worktree_path = dir.path().join(".worktrees").join("verbose-test");
        fs::create_dir_all(&worktree_path).expect("create parent");

        create_worktree(dir.path(), "verbose-test", &worktree_path);
        remove_dir_recursive(&worktree_path);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("verbose-test"));
    }

    #[test]
    fn clean_force_removes_stale_without_prompt() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let worktree_path = dir.path().join(".worktrees").join("force-clean");
        fs::create_dir_all(&worktree_path).expect("create parent");

        create_worktree(dir.path(), "force-clean", &worktree_path);
        remove_dir_recursive(&worktree_path);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--force")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));
    }
}

// ===========================================================================
// clean — Multiple Stale Worktrees
// ===========================================================================

mod clean_multiple_stale {
    use super::*;

    #[test]
    fn clean_removes_multiple_stale_worktrees() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let wt1 = dir.path().join(".worktrees").join("stale-1");
        let wt2 = dir.path().join(".worktrees").join("stale-2");
        let wt3 = dir.path().join(".worktrees").join("stale-3");
        fs::create_dir_all(&wt1).expect("create wt1 parent");
        fs::create_dir_all(&wt2).expect("create wt2 parent");
        fs::create_dir_all(&wt3).expect("create wt3 parent");

        create_worktree(dir.path(), "stale-1", &wt1);
        create_worktree(dir.path(), "stale-2", &wt2);
        create_worktree(dir.path(), "stale-3", &wt3);

        // Make all three stale
        remove_dir_recursive(&wt1);
        remove_dir_recursive(&wt2);
        remove_dir_recursive(&wt3);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("stale-1"))
            .stdout(predicate::str::contains("stale-2"))
            .stdout(predicate::str::contains("stale-3"));
    }

    #[test]
    fn dry_run_lists_multiple_stale_sessions() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let wt1 = dir.path().join(".worktrees").join("dry-1");
        let wt2 = dir.path().join(".worktrees").join("dry-2");
        fs::create_dir_all(&wt1).expect("create wt1 parent");
        fs::create_dir_all(&wt2).expect("create wt2 parent");

        create_worktree(dir.path(), "dry-1", &wt1);
        create_worktree(dir.path(), "dry-2", &wt2);

        remove_dir_recursive(&wt1);
        remove_dir_recursive(&wt2);

        let result = scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .current_dir(dir.path())
            .assert()
            .success();

        let output = result.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("dry-1"), "missing dry-1 in output: {stdout}");
        assert!(stdout.contains("dry-2"), "missing dry-2 in output: {stdout}");
    }
}

// ===========================================================================
// clean — Mixed State (some stale, some active)
// ===========================================================================

mod clean_mixed_state {
    use super::*;

    #[test]
    fn clean_preserves_active_worktrees() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Create an active worktree (directory still exists)
        let active_path = dir.path().join(".worktrees").join("active-ws");
        fs::create_dir_all(&active_path).expect("create active parent");
        create_worktree(dir.path(), "active-ws", &active_path);

        // Create a stale worktree (directory removed)
        let stale_path = dir.path().join(".worktrees").join("stale-ws");
        fs::create_dir_all(&stale_path).expect("create stale parent");
        create_worktree(dir.path(), "stale-ws", &stale_path);
        remove_dir_recursive(&stale_path);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("stale-ws"))
            .stdout(predicate::str::contains("Removed"));

        // Active worktree should still be listed
        let output = StdCommand::new("git")
            .args(["worktree", "list"])
            .current_dir(dir.path())
            .output()
            .expect("git worktree list");
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(
            listing.contains("active-ws"),
            "active worktree should still exist: {listing}"
        );
        assert!(
            !listing.contains("stale-ws"),
            "stale worktree should be gone: {listing}"
        );
    }

    #[test]
    fn dry_run_preserves_all_worktrees() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let active_path = dir.path().join(".worktrees").join("alive");
        let stale_path = dir.path().join(".worktrees").join("dead");
        fs::create_dir_all(&active_path).expect("create active parent");
        fs::create_dir_all(&stale_path).expect("create stale parent");

        create_worktree(dir.path(), "alive", &active_path);
        create_worktree(dir.path(), "dead", &stale_path);
        remove_dir_recursive(&stale_path);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("dead"));

        // Both worktrees still listed after dry-run
        let output = StdCommand::new("git")
            .args(["worktree", "list"])
            .current_dir(dir.path())
            .output()
            .expect("git worktree list");
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(listing.contains("alive"), "alive should remain: {listing}");
        assert!(listing.contains("dead"), "dead should remain after dry-run: {listing}");
    }
}

// ===========================================================================
// clean — Idempotency
// ===========================================================================

mod clean_idempotency {
    use super::*;

    #[test]
    fn running_clean_twice_is_idempotent() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let stale_path = dir.path().join(".worktrees").join("idem-stale");
        fs::create_dir_all(&stale_path).expect("create parent");
        create_worktree(dir.path(), "idem-stale", &stale_path);
        remove_dir_recursive(&stale_path);

        // First clean — removes stale worktree
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));

        // Second clean — no stale sessions
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn clean_after_partial_prune_succeeds() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let wt1 = dir.path().join(".worktrees").join("partial-1");
        let wt2 = dir.path().join(".worktrees").join("partial-2");
        fs::create_dir_all(&wt1).expect("create parent 1");
        fs::create_dir_all(&wt2).expect("create parent 2");

        create_worktree(dir.path(), "partial-1", &wt1);
        create_worktree(dir.path(), "partial-2", &wt2);

        // Only make one stale
        remove_dir_recursive(&wt1);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("partial-1"));

        // Second clean — nothing stale (partial-2 still active)
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }
}

// ===========================================================================
// clean — Worktree with Branch vs Detached HEAD
// ===========================================================================

mod clean_worktree_variants {
    use super::*;

    #[test]
    fn clean_stale_detached_head_worktree() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Get current HEAD hash
        let output = StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.path())
            .output()
            .expect("git rev-parse HEAD");
        let head_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Create a detached HEAD worktree
        let detached_path = dir.path().join(".worktrees").join("detached-test");
        fs::create_dir_all(&detached_path).expect("create parent");

        let status = StdCommand::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                detached_path.to_str().unwrap(),
                &head_hash,
            ])
            .current_dir(dir.path())
            .status()
            .expect("git worktree add --detach");
        assert!(status.success(), "detached worktree add failed");

        remove_dir_recursive(&detached_path);

        // Clean should handle detached HEAD worktrees
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));
    }

    #[test]
    fn clean_stale_worktree_with_long_branch_name() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let long_name = "feature/very/deeply/nested/branch/name/that-is-quite-long";
        let wt_path = dir.path().join(".worktrees").join("long-branch");
        fs::create_dir_all(&wt_path).expect("create parent");

        create_worktree(dir.path(), long_name, &wt_path);
        remove_dir_recursive(&wt_path);

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));
    }
}

// ===========================================================================
// clean — Error Scenarios
// ===========================================================================

mod clean_errors {
    use super::*;

    #[test]
    fn clean_in_non_git_directory_fails() {
        let dir = TempDir::new().expect("temp dir");
        // No git init

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    #[test]
    fn clean_in_empty_directory_fails() {
        let dir = TempDir::new().expect("temp dir");

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    #[test]
    fn dry_run_in_non_git_directory_fails() {
        let dir = TempDir::new().expect("temp dir");

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .current_dir(dir.path())
            .assert()
            .failure();
    }

    #[test]
    fn force_in_non_git_directory_fails() {
        let dir = TempDir::new().expect("temp dir");

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--force")
            .current_dir(dir.path())
            .assert()
            .failure();
    }
}

// ===========================================================================
// clean — Flag Combinations
// ===========================================================================

mod clean_flag_combinations {
    use super::*;

    #[test]
    fn clean_dry_run_and_force() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Both flags — dry-run should take precedence (no changes)
        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .arg("--force")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("No stale sessions"));
    }

    #[test]
    fn clean_dry_run_verbose() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let wt = dir.path().join(".worktrees").join("dv-test");
        fs::create_dir_all(&wt).expect("create parent");
        create_worktree(dir.path(), "dv-test", &wt);
        remove_dir_recursive(&wt);

        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("dry-run"))
            .stdout(predicate::str::contains("dv-test"));
    }

    #[test]
    fn clean_all_flags() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        let wt = dir.path().join(".worktrees").join("all-flags");
        fs::create_dir_all(&wt).expect("create parent");
        create_worktree(dir.path(), "all-flags", &wt);
        remove_dir_recursive(&wt);

        // dry-run + force + verbose: dry-run should win (no removal)
        scp_cli()
            .args(["workspace", "clean"])
            .arg("--dry-run")
            .arg("--force")
            .arg("--verbose")
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("dry-run"));
    }
}

// ===========================================================================
// clean — Integration with Other Commands
// ===========================================================================

mod clean_command_integration {
    use super::*;

    #[test]
    fn clean_after_worktree_add_and_remove() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Create worktree
        let wt = dir.path().join(".worktrees").join("integration-wt");
        fs::create_dir_all(&wt).expect("create parent");
        create_worktree(dir.path(), "integration-wt", &wt);

        // Verify it shows up
        let output = StdCommand::new("git")
            .args(["worktree", "list"])
            .current_dir(dir.path())
            .output()
            .expect("git worktree list");
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(listing.contains("integration-wt"));

        // Stale it
        remove_dir_recursive(&wt);

        // Clean
        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed"));

        // Status should still work
        scp_cli()
            .arg("status")
            .current_dir(dir.path())
            .assert()
            .success();
    }

    #[test]
    fn clean_does_not_affect_main_branch() {
        let dir = TempDir::new().expect("temp dir");
        init_git_repo(dir.path());

        // Create and stale a worktree
        let wt = dir.path().join(".worktrees").join("to-remove");
        fs::create_dir_all(&wt).expect("create parent");
        create_worktree(dir.path(), "to-remove", &wt);
        remove_dir_recursive(&wt);

        scp_cli()
            .args(["workspace", "clean"])
            .current_dir(dir.path())
            .assert()
            .success();

        // Main branch should still be intact
        let output = StdCommand::new("git")
            .args(["branch"])
            .current_dir(dir.path())
            .output()
            .expect("git branch");
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(
            branches.contains("main") || branches.contains("master"),
            "main/master branch should still exist: {branches}"
        );
    }
}
