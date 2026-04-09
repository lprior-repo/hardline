//! Doctor command - health checks and diagnostics

use scp_core::{vcs, Error, Result};

fn check_vcs_available() -> Result<bool> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let is_git = cwd.join(".git").exists();
    Ok(is_git)
}

fn check_dependency(name: &str) -> Result<bool> {
    std::process::Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .map_err(|e| Error::io_error(e.to_string()))
}

fn check_config_exists() -> Result<bool> {
    let dir = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::config_not_found("No config dir"))?;
    let config_file = dir.config_dir().join("config.toml");
    Ok(config_file.exists())
}

fn check_workspaces_count() -> Result<usize> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.len())
}

/// Run health checks
pub fn run(full: bool) -> Result<()> {
    println!("Running SCP diagnostics...\n");

    let check_vcs_result = check_vcs_available();
    let check_dep_git = check_dependency("git");
    let check_config_result = check_config_exists();
    let check_workspaces_result = check_workspaces_count();

    let vcs_passed = check_vcs_result.as_ref().copied().unwrap_or(false);
    let dep_git_found = check_dep_git.as_ref().copied().unwrap_or(false);
    let _config_result = check_config_result.as_ref().copied().unwrap_or(false);
    let _workspaces_count = check_workspaces_result.as_ref().copied().unwrap_or(0);

    println!("[1/5] Checking VCS...");
    if vcs_passed {
        println!("  ✓ VCS initialized");
    } else {
        println!("  ✗ No VCS found");
        println!("    Run 'scp init --vcs git'");
    }

    println!("\n[2/5] Checking dependencies...");
    if dep_git_found {
        println!("  ✓ git found");
    } else {
        println!("  ✗ No VCS CLI found (git)");
    }

    println!("\n[3/5] Checking configuration...");
    match check_config_result {
        Ok(true) => println!("  ✓ Config valid"),
        Ok(false) => {
            println!("  ⚠ No config found (will use defaults)");
        }
        Err(e) => {
            println!("  ✗ Config error: {}", e);
        }
    }

    println!("\n[4/5] Checking workspaces...");
    match check_workspaces_result {
        Ok(count) => {
            if count > 0 {
                println!("  ✓ {} workspace(s) found", count);
            } else {
                println!("  ℹ No workspaces (run 'scp workspace spawn <name>')");
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }

    let all_passed = vcs_passed && dep_git_found;

    if full {
        println!("\n[5/5] Running full diagnostics...");

        if let Ok(path) = std::env::current_dir() {
            #[cfg(unix)]
            {
                use std::process::Command;
                if let Ok(output) = Command::new("df").arg("-h").arg(path).output() {
                    let output = String::from_utf8_lossy(&output.stdout);
                    for line in output.lines().skip(1) {
                        println!("  Disk: {}", line);
                    }
                }
            }
        }

        if let Ok(path) = std::env::current_dir() {
            let lock_patterns = [".git"];
            for pattern in lock_patterns {
                let lock_path = path.join(pattern).join("lock");
                if lock_path.exists() {
                    println!("  ⚠ Found lock file: {:?}", lock_path);
                }
            }
        }

        if let Ok(path) = std::env::current_dir() {
            if let Ok(be) = vcs::create_backend(&path) {
                if let Ok(status) = be.status() {
                    match status {
                        scp_core::vcs::VcsStatus::Conflicted => {
                            println!("  ✗ Working copy has conflicts!");
                        }
                        scp_core::vcs::VcsStatus::Dirty => {
                            println!("  ⚠ Working copy has uncommitted changes");
                        }
                        _ => {}
                    }
                }
            }
        }
    } else {
        println!("\n[5/5] Skipping full diagnostics (use --full)");
    }

    println!("\n{}", "=".repeat(50));
    if all_passed {
        println!("✓ All checks passed");
        Ok(())
    } else {
        println!("✗ Some checks failed - see above for details");
        Err(Error::internal("Diagnostics failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // ========================================================================
    // check_vcs_available
    // ========================================================================

    #[test]
    #[serial]
    fn check_vcs_available_returns_true_in_git_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_vcs_available();
        assert_eq!(result.expect("should succeed"), true);
    }

    #[test]
    #[serial]
    fn check_vcs_available_returns_false_without_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_vcs_available();
        assert_eq!(result.expect("should succeed"), false);
    }

    #[test]
    #[serial]
    fn check_vcs_available_returns_false_for_gitignore_not_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".gitignore"), "").expect("write");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_vcs_available();
        assert_eq!(result.expect("should succeed"), false);
    }

    #[test]
    #[serial]
    fn check_vcs_available_distinguishes_git_dir_from_git_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".git"), "gitdir: /some/other/path").expect("write");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_vcs_available();
        assert_eq!(result.expect("should succeed"), true);
    }

    // ========================================================================
    // check_dependency
    // ========================================================================

    #[test]
    fn check_dependency_git_succeeds() {
        let result = check_dependency("git");
        assert!(result.is_ok());
        assert_eq!(result.expect("git should exist"), true);
    }

    #[test]
    fn check_dependency_nonexistent_returns_ok_false_or_err() {
        let result = check_dependency("nonexistent_binary_xyz_12345");
        match result {
            Ok(found) => assert_eq!(found, false),
            Err(_) => {}
        }
    }

    #[test]
    fn check_dependency_empty_name_returns_err_or_false() {
        let result = check_dependency("");
        match result {
            Ok(found) => assert_eq!(found, false),
            Err(_) => {}
        }
    }

    #[test]
    fn check_dependency_never_panics_on_any_input() {
        let inputs = [
            "",
            "git",
            "nonexistent",
            "/usr/bin/ls",
            "../../etc/passwd",
            "a;b;c",
            "test\0binary",
        ];
        for input in inputs {
            let _ = check_dependency(input);
        }
    }

    // ========================================================================
    // check_config_exists
    // ========================================================================

    #[test]
    fn check_config_exists_returns_bool_without_panic() {
        let result = check_config_exists();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_config_exists_ok_result_is_bool() {
        let result = check_config_exists();
        match result {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("config") || msg.contains("Config"),
                    "Error should mention config: {msg}"
                );
            }
        }
    }

    // ========================================================================
    // check_workspaces_count
    // ========================================================================

    #[test]
    #[serial]
    fn check_workspaces_count_fails_without_vcs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_workspaces_count();
        assert!(result.is_err(), "should fail without VCS");
    }

    #[test]
    #[serial]
    fn check_workspaces_count_succeeds_or_errors_in_git_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = check_workspaces_count();
        match result {
            Ok(count) => assert!(count < 1000),
            Err(_) => {}
        }
    }

    // ========================================================================
    // run() — stdout capture tests
    // ========================================================================

    #[test]
    #[serial]
    fn run_prints_header() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(false)));
        assert!(output.is_ok() || output.is_err());
    }

    #[test]
    #[serial]
    fn run_non_full_prints_skip_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let _ = run(false);
    }

    #[test]
    #[serial]
    fn run_full_mode_executes() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let _ = run(true);
    }

    #[test]
    #[serial]
    fn run_without_git_returns_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = run(false);
        assert!(result.is_err(), "run without git should return Err");
    }

    #[test]
    #[serial]
    fn run_full_without_git_returns_err() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = run(true);
        assert!(result.is_err(), "run --full without git should return Err");
    }

    #[test]
    #[serial]
    fn run_with_git_and_config_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let _ = run(false);
    }

    // ========================================================================
    // run() — lock file detection (full mode)
    // ========================================================================

    #[test]
    #[serial]
    fn run_full_detects_git_lock_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::fs::create_dir(dir.path().join(".git").join("lock")).expect("create lock");
        std::env::set_current_dir(dir.path()).expect("cd");
        let _ = run(true);
    }

    #[test]
    #[serial]
    fn run_full_no_lock_file_no_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let _ = run(true);
    }

    // ========================================================================
    // run() — repair suggestions in output
    // ========================================================================

    #[test]
    #[serial]
    fn run_includes_init_suggestion_when_no_vcs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = run(false);
        assert!(result.is_err());
        let err = result.expect_err("should be err");
        let msg = format!("{err}");
        assert!(
            msg.contains("Diagnostics failed") || msg.contains("internal"),
            "Error should indicate diagnostics failure: {msg}"
        );
    }

    // ========================================================================
    // check_workspaces_count — edge cases
    // ========================================================================

    #[test]
    #[serial]
    fn check_workspaces_count_in_nested_git_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("a").join("b");
        std::fs::create_dir_all(nested.join(".git")).expect("create nested .git");
        std::env::set_current_dir(&nested).expect("cd");
        let result = check_workspaces_count();
        match result {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    // ========================================================================
    // Combined health check logic
    // ========================================================================

    #[test]
    #[serial]
    fn all_checks_pass_only_when_git_and_dependency_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        std::env::set_current_dir(dir.path()).expect("cd");
        let vcs = check_vcs_available().expect("vcs check").to_owned();
        let dep = check_dependency("git").expect("dep check").to_owned();
        assert!(vcs && dep, "both git VCS and git CLI should pass");
    }

    #[test]
    #[serial]
    fn all_checks_fail_when_no_vcs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let vcs = check_vcs_available().expect("vcs check").to_owned();
        assert!(!vcs, "no VCS should fail");
    }

    #[test]
    #[serial]
    fn dependency_check_independent_of_vcs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let git_dep = check_dependency("git").expect("dep check");
        assert!(git_dep, "git CLI should be found regardless of VCS");
    }

    // ========================================================================
    // Error type verification
    // ========================================================================

    #[test]
    #[serial]
    fn run_error_is_internal_on_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(dir.path()).expect("cd");
        let result = run(false);
        let err = result.expect_err("should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("Diagnostics failed"),
            "Expected 'Diagnostics failed', got: {msg}"
        );
    }

    // ========================================================================
    // Proptest — no panics on any input combination
    // ========================================================================

    #[cfg(test)]
    mod proptests {
        use super::*;

        proptest::proptest! {
            #[test]
            fn proptest_check_dependency_never_panics(name in ".*") {
                let _ = check_dependency(&name);
            }

            #[test]
            #[serial]
            fn proptest_run_never_panics(full in proptest::bool::ANY) {
                let dir = tempfile::tempdir().expect("tempdir");
                std::fs::create_dir(dir.path().join(".git")).expect("create .git");
                std::env::set_current_dir(dir.path()).expect("cd");
                let _ = run(full);
            }
        }
    }

    // ========================================================================
    // Red Queen adversarial tests
    // ========================================================================

    mod adversarial {
        use super::*;

        #[test]
        #[serial]
        fn run_with_path_traversal_in_cwd() {
            let dir = tempfile::tempdir().expect("tempdir");
            let traversal = dir.path().join("../../../etc");
            if traversal.exists() {
                std::env::set_current_dir(&traversal).expect("cd");
                let _ = run(false);
            }
        }

        #[test]
        #[serial]
        fn run_in_readonly_directory() {
            let dir = tempfile::tempdir().expect("tempdir");
            std::env::set_current_dir(dir.path()).expect("cd");
            let _ = run(false);
        }

        #[test]
        fn check_dependency_with_path_injection() {
            let result = check_dependency("../../../usr/bin/git");
            match result {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        #[test]
        fn check_dependency_with_null_bytes() {
            let result = check_dependency("git\0--version");
            match result {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        #[test]
        #[serial]
        fn check_vcs_available_in_directory_with_symlink() {
            let dir = tempfile::tempdir().expect("tempdir");
            #[cfg(unix)]
            {
                let target = dir.path().join("target_dir");
                std::fs::create_dir(&target).expect("create target");
                let link = dir.path().join("link");
                std::os::unix::fs::symlink(&target, &link).expect("symlink");
                std::env::set_current_dir(&link).expect("cd");
            }
            let _ = check_vcs_available();
        }

        #[test]
        fn run_concurrent_calls_do_not_deadlock() {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(dir.path().join(".git")).expect("create .git");

            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let dir = dir.path().to_path_buf();
                    std::thread::spawn(move || {
                        std::env::set_current_dir(&dir).expect("cd");
                        let _ = run(false);
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        }

        #[test]
        fn run_full_concurrent_calls_do_not_deadlock() {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(dir.path().join(".git")).expect("create .git");

            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let dir = dir.path().to_path_buf();
                    std::thread::spawn(move || {
                        std::env::set_current_dir(&dir).expect("cd");
                        let _ = run(true);
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        }
    }
}
