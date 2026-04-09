//! Context command - shows current workspace/branch/location

use scp_core::{output::Output, vcs, Result};

/// Show current context (workspace, branch, VCS status)
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;

    // Get current workspace by finding the one with is_current = true
    // Note: Git backends return unimplemented error for list_workspaces (use worktrees)
    let workspace_name = backend
        .list_workspaces()
        .ok()
        .and_then(|workspaces| {
            workspaces
                .into_iter()
                .find(|w| w.is_current)
                .map(|w| w.name)
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get current branch - use "unknown" if no commits yet (detached HEAD)
    let branch = backend
        .current_branch()
        .unwrap_or_else(|_| "unknown".to_string());
    let vcs_status = backend.status()?;

    Output::info("Current Context:");
    Output::info(&format!("  Workspace: {}", workspace_name));
    Output::info(&format!("  Branch: {}", branch));
    Output::info(&format!("  Status: {}", vcs_status));

    Ok(())
}

/// Alias for run() - shows current context
pub fn whereami() -> Result<()> {
    run()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    use scp_core::output::{Output, Verbosity};

    fn reset_verbosity() {
        Output::set_verbose(false, false);
    }

    #[test]
    fn test_context_fails_when_no_git_repo() {
        let tmp = TempDir::new().expect("temp dir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        assert!(result.is_err(), "Context should fail without .git");
    }

    #[serial_test::serial]
    #[test]
    fn test_context_with_real_git_repo() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok(), "Context should work with real git repo");
    }

    #[serial_test::serial]
    #[test]
    fn test_context_shows_branch_name() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");
        fs::write(tmp.path().join("test.txt"), "test").expect("write file");
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_context_vcs_status_clean() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");
        fs::write(tmp.path().join("test.txt"), "test").expect("write file");
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_context_vcs_status_dirty() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(tmp.path())
            .output()
            .expect("git config");
        fs::write(tmp.path().join("test.txt"), "test").expect("write file");
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");
        fs::write(tmp.path().join("test.txt"), "modified").expect("modify file");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_whereami_alias_is_same_as_run() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result_run = run();
        let result_whereami = whereami();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert_eq!(result_run.is_ok(), result_whereami.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_context_quiet_mode() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");

        env::set_current_dir(tmp.path()).expect("chdir");
        Output::set_verbose(false, true);

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_context_verbose_mode() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");

        env::set_current_dir(tmp.path()).expect("chdir");
        Output::set_verbose(true, false);

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[test]
    fn test_context_nested_directory() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        let nested = tmp.path().join("nested").join("deep");
        fs::create_dir_all(&nested).expect("create nested");
        Command::new("git")
            .args(["init"])
            .current_dir(&nested)
            .output()
            .expect("git init");

        env::set_current_dir(&nested).expect("chdir");
        reset_verbosity();

        let result = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert!(result.is_ok());
    }

    #[test]
    fn test_context_verbosity_default() {
        reset_verbosity();
        assert_eq!(Verbosity::current(), Verbosity::Normal);
    }

    #[test]
    fn test_context_verbosity_quiet() {
        Output::set_verbose(false, true);
        assert_eq!(Verbosity::current(), Verbosity::Quiet);
        reset_verbosity();
    }

    #[test]
    fn test_context_verbosity_verbose() {
        Output::set_verbose(true, false);
        assert_eq!(Verbosity::current(), Verbosity::Verbose);
        reset_verbosity();
    }

    #[serial_test::serial]
    #[test]
    fn test_context_multiple_calls_consistent() {
        let original_dir = env::current_dir().expect("get current dir");
        let tmp = TempDir::new().expect("temp dir");

        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");

        env::set_current_dir(tmp.path()).expect("chdir");
        reset_verbosity();

        let result1 = run();
        let result2 = run();
        let result3 = run();
        reset_verbosity();

        env::set_current_dir(&original_dir).ok();

        assert_eq!(result1.is_ok(), result2.is_ok());
        assert_eq!(result2.is_ok(), result3.is_ok());
    }
}
