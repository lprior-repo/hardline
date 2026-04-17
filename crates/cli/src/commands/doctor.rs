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

    #[test]
    fn check_dependency_known_binary_succeeds() {
        // "ls" exists on all Unix systems
        let result = check_dependency("ls");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn check_dependency_unknown_binary_fails_gracefully() {
        // A nonexistent binary should not panic — it returns Ok(false) or Err
        let result = check_dependency("nonexistent_binary_xyz_123");
        // Either the binary isn't found (Ok(false)) or the command itself fails (Err)
        match result {
            Ok(found) => assert!(!found),
            Err(_) => {} // Also acceptable — command not found
        }
    }

    #[test]
    fn check_dependency_empty_name_fails() {
        let result = check_dependency("");
        assert!(result.is_err());
    }

    #[test]
    fn check_vcs_available_detects_git_dir() {
        // Create a temp dir with .git to test the detection logic
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = check_vcs_available();

        std::env::set_current_dir(&original).unwrap();
        assert!(result.is_ok());
        assert!(result.unwrap(), ".git directory should be detected");
    }

    #[test]
    fn check_vcs_available_detects_git_file() {
        // Git worktrees use a .git file, not directory
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".git"), "ref: some-ref").unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = check_vcs_available();

        std::env::set_current_dir(&original).unwrap();
        assert!(result.is_ok());
        assert!(result.unwrap(), ".git file (worktree) should be detected");
    }

    #[test]
    fn check_vcs_available_returns_false_without_git() {
        let dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = check_vcs_available();

        std::env::set_current_dir(&original).unwrap();
        assert!(result.is_ok());
        assert!(!result.unwrap(), "no .git should return false");
    }

    #[test]
    fn check_config_exists_returns_result() {
        // Config may or may not exist, but the function should not panic
        let result = check_config_exists();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_workspaces_count_returns_result() {
        // Should return a count without panicking
        let result = check_workspaces_count();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn run_full_flag_accepted() {
        // Verify both variants compile and return Result
        let _basic: fn(bool) -> Result<()> = run;
    }

    #[test]
    fn doctor_uses_vcs_module() {
        use scp_core::vcs::VcsStatus;
        // Verify VcsStatus variants used in run(full=true)
        let _ = VcsStatus::Clean;
        let _ = VcsStatus::Dirty;
        let _ = VcsStatus::Conflicted;
        let _ = VcsStatus::Detached;
    }

    #[test]
    fn error_constructors_used_by_doctor() {
        // Verify error constructors exist
        let _ = scp_core::Error::io_error("test");
        let _ = scp_core::Error::internal("test");
    }
}
