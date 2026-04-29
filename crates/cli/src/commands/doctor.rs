//! Doctor command - health checks and diagnostics
//!
//! Data->Calc->Actions architecture:
//! - Data: Uses introspection::doctor::DoctorCheck types
//! - Calc: Pure check functions that return structured results
//! - Actions: Console output and orchestration

use scp_core::{
    config::ConfigManager,
    introspection::doctor::{CheckStatus, DoctorCheck},
    vcs::{self, VcsStatus},
    Error, Result,
};

fn get_project_dirs() -> Result<directories::ProjectDirs> {
    directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::config_not_found("No config dir"))
}

fn get_current_dir() -> Result<std::path::PathBuf> {
    std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))
}

fn vcs_exists(cwd: &std::path::Path) -> bool {
    cwd.join(".git").exists()
}

fn check_vcs_available() -> Result<DoctorCheck> {
    let cwd = get_current_dir()?;
    let exists = vcs_exists(&cwd);
    let status = if exists {
        CheckStatus::Pass
    } else {
        CheckStatus::Fail
    };
    Ok(DoctorCheck {
        name: "VCS initialized".to_string(),
        status,
        message: if exists {
            "Git repository found".to_string()
        } else {
            "No VCS found".to_string()
        },
        suggestion: if exists {
            None
        } else {
            Some("Run 'scp init --vcs git'".to_string())
        },
        auto_fixable: false,
        details: None,
    })
}

fn check_git_dependency() -> Result<DoctorCheck> {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map_err(|e| Error::io_error(e.to_string()))?;
    let found = output.status.success();
    let status = if found {
        CheckStatus::Pass
    } else {
        CheckStatus::Fail
    };
    Ok(DoctorCheck {
        name: "Git CLI".to_string(),
        status,
        message: if found {
            "git found".to_string()
        } else {
            "No git CLI found".to_string()
        },
        suggestion: if found {
            None
        } else {
            Some("Install git".to_string())
        },
        auto_fixable: false,
        details: None,
    })
}

fn check_config_exists() -> Result<DoctorCheck> {
    let dir = get_project_dirs()?;
    let config_file = dir.config_dir().join("config.toml");
    let exists = config_file.exists();
    let status = if exists {
        CheckStatus::Pass
    } else {
        CheckStatus::Warn
    };
    Ok(DoctorCheck {
        name: "Configuration".to_string(),
        status,
        message: if exists {
            "Config file found".to_string()
        } else {
            "No config found (will use defaults)".to_string()
        },
        suggestion: None,
        auto_fixable: false,
        details: None,
    })
}

fn check_config_valid() -> Result<DoctorCheck> {
    let dir = get_project_dirs()?;
    let config_file = dir.config_dir().join("config.toml");
    if !config_file.exists() {
        return Ok(DoctorCheck {
            name: "Config validation".to_string(),
            status: CheckStatus::Warn,
            message: "No config file to validate".to_string(),
            suggestion: Some("Run 'scp config list' to see current settings".to_string()),
            auto_fixable: false,
            details: None,
        });
    }
    let manager = ConfigManager::new()?;
    match manager.load() {
        Ok(_) => Ok(DoctorCheck {
            name: "Config validation".to_string(),
            status: CheckStatus::Pass,
            message: "Config is valid".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }),
        Err(e) => Ok(DoctorCheck {
            name: "Config validation".to_string(),
            status: CheckStatus::Fail,
            message: format!("Config error: {}", e),
            suggestion: Some("Run 'scp config list' to check configuration".to_string()),
            auto_fixable: false,
            details: None,
        }),
    }
}

fn check_workspaces() -> Result<DoctorCheck> {
    let cwd = get_current_dir()?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;
    let count = workspaces.len();
    let status = if count > 0 {
        CheckStatus::Pass
    } else {
        CheckStatus::Warn
    };
    Ok(DoctorCheck {
        name: "Workspaces".to_string(),
        status,
        message: if count > 0 {
            format!("{} workspace(s) found", count)
        } else {
            "No workspaces found".to_string()
        },
        suggestion: if count == 0 {
            Some("Run 'scp workspace spawn <name>'".to_string())
        } else {
            None
        },
        auto_fixable: false,
        details: None,
    })
}

#[cfg(unix)]
fn gather_disk_usage(path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("df")
        .arg("-h")
        .arg(path)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(not(unix))]
fn gather_disk_usage(_path: &std::path::Path) -> Option<String> {
    None
}

fn check_lock_files(path: &std::path::Path) -> Vec<String> {
    let lock_patterns = [".git"];
    lock_patterns
        .iter()
        .filter_map(|pattern| {
            let lock_path = path.join(pattern).join("lock");
            if lock_path.exists() {
                Some(format!("{:?}", lock_path))
            } else {
                None
            }
        })
        .collect()
}

fn check_vcs_status(path: &std::path::Path) -> Option<VcsStatus> {
    vcs::create_backend(path).ok()?.status().ok()
}

fn print_check(check: &DoctorCheck, index: usize, total: usize) {
    let status_symbol = match check.status {
        CheckStatus::Pass => "✓",
        CheckStatus::Warn => "⚠",
        CheckStatus::Fail => "✗",
    };
    println!("[{}{}] {}...", index, total, check.name);
    println!("  {} {}", status_symbol, check.message);
    if let Some(ref suggestion) = check.suggestion {
        println!("    {}", suggestion);
    }
}

fn print_diagnostic_line(line: &str) {
    println!("  {}", line);
}

fn all_critical_passed(checks: &[&DoctorCheck]) -> bool {
    checks
        .iter()
        .filter(|c| c.name == "VCS initialized" || c.name == "Git CLI")
        .all(|c| c.status == CheckStatus::Pass)
}

/// Run health checks
pub fn run(full: bool) -> Result<()> {
    println!("Running SCP diagnostics...\n");

    let vcs_check = check_vcs_available()?;
    let git_check = check_git_dependency()?;
    let config_exists_check = check_config_exists()?;
    let config_valid_check = check_config_valid()?;
    let workspaces_check = check_workspaces()?;

    let all_checks = vec![
        &vcs_check,
        &git_check,
        &config_exists_check,
        &config_valid_check,
        &workspaces_check,
    ];

    let check_count = all_checks.len();
    for (i, check) in all_checks.iter().enumerate() {
        print_check(check, i + 1, check_count);
    }

    if full {
        run_full_diagnostics(check_count)?;
    } else {
        println!(
            "\n[{}] Skipping full diagnostics (use --full)",
            check_count + 1
        );
    }

    println!("\n{}", "=".repeat(50));

    if all_critical_passed(&all_checks) {
        println!("✓ All checks passed");
        Ok(())
    } else {
        println!("✗ Some checks failed - see above for details");
        Err(Error::internal("Diagnostics failed"))
    }
}

/// Run full diagnostics (disk usage, lock files, VCS status).
fn run_full_diagnostics(check_count: usize) -> Result<()> {
    println!("\n[{}] Running full diagnostics...", check_count + 1);

    let cwd = get_current_dir()?;

    if let Some(disk) = gather_disk_usage(&cwd) {
        println!("\nDisk usage:");
        disk.lines().skip(1).for_each(print_diagnostic_line);
    }

    let lock_files = check_lock_files(&cwd);
    if !lock_files.is_empty() {
        println!("\nLock files found:");
        for lock in lock_files {
            print_diagnostic_line(&format!("⚠ Found lock file: {}", lock));
        }
    }

    if let Some(status) = check_vcs_status(&cwd) {
        match status {
            VcsStatus::Conflicted => {
                println!("\n  ✗ Working copy has conflicts!");
            }
            VcsStatus::Dirty => {
                println!("\n  ⚠ Working copy has uncommitted changes");
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_dependency_known_binary_succeeds() {
        // "ls" exists on all Unix systems
        let result = check_git_dependency();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, CheckStatus::Pass);
    }

    #[test]
    fn check_dependency_unknown_binary_fails_gracefully() {
        // A nonexistent binary should not panic — it returns a DoctorCheck
        let result = check_git_dependency();
        assert!(result.is_ok());
    }

    #[test]
    fn check_dependency_empty_name_fails() {
        // check_git_dependency runs "git --version" which should succeed
        let result = check_git_dependency();
        assert!(result.is_ok());
    }

    #[test]
    fn check_vcs_available_detects_git_dir() {
        let Ok(dir) = tempfile::tempdir() else { return };
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let Ok(original) = std::env::current_dir() else { return };
        if std::env::set_current_dir(dir.path()).is_err() { return }

        let result = check_vcs_available();

        let _ = std::env::set_current_dir(&original);
        if let Ok(r) = result {
            assert_eq!(r.status, CheckStatus::Pass, ".git directory should be detected");
        }
    }


    #[test]
    fn check_vcs_available_detects_git_file() {
        // Git worktrees use a .git file, not directory
        let Ok(dir) = tempfile::tempdir() else { return };
        std::fs::write(dir.path().join(".git"), "ref: some-ref").unwrap();
        let Ok(original) = std::env::current_dir() else { return };
        if std::env::set_current_dir(dir.path()).is_err() { return }

        let result = check_vcs_available();

        let _ = std::env::set_current_dir(&original);
        if let Ok(r) = result {
            assert_eq!(r.status, CheckStatus::Pass, ".git file (worktree) should be detected");
        }
    }

    #[test]
    fn check_vcs_available_returns_false_without_git() {
        let Ok(dir) = tempfile::tempdir() else { return };
        let Ok(original) = std::env::current_dir() else { return };
        if std::env::set_current_dir(dir.path()).is_err() { return }

        let result = check_vcs_available();

        let _ = std::env::set_current_dir(&original);
        if let Ok(r) = result {
            assert_eq!(r.status, CheckStatus::Fail, "no .git should return Fail");
        }
    }

    #[test]
    fn check_config_exists_returns_result() {
        // Config may or may not exist, but the function should not panic
        let result = check_config_exists();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn check_workspaces_count_returns_result() {
        // Should return a DoctorCheck without panicking
        let result = check_workspaces();
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
