//! Doctor command - health checks and diagnostics

use scp_core::{vcs, Error, Result};

fn check_vcs_available() -> Result<bool> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let is_jj = cwd.join(".jj").exists();
    let is_git = cwd.join(".git").exists();
    Ok(is_jj || is_git)
}

fn check_dependency(name: &str) -> Result<bool> {
    std::process::Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .map_err(|e| Error::Io(e))
}

fn check_config_exists() -> Result<bool> {
    let dir = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::ConfigNotFound("No config dir".into()))?;
    let config_file = dir.config_dir().join("config.toml");
    Ok(config_file.exists())
}

fn check_workspaces_count() -> Result<usize> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.len())
}

/// Run health checks
pub fn run(full: bool) -> Result<()> {
    println!("Running SCP diagnostics...\n");

    let check_vcs_result = check_vcs_available();
    let check_dep_jj = check_dependency("jj");
    let check_dep_git = check_dependency("git");
    let check_config_result = check_config_exists();
    let check_workspaces_result = check_workspaces_count();

    let vcs_passed = check_vcs_result.as_ref().copied().unwrap_or(false);
    let dep_jj_found = check_dep_jj.as_ref().copied().unwrap_or(false);
    let dep_git_found = check_dep_git.as_ref().copied().unwrap_or(false);
    let config_result = check_config_result.as_ref().copied().unwrap_or(false);
    let workspaces_count = check_workspaces_result.as_ref().copied().unwrap_or(0);

    println!("[1/5] Checking VCS...");
    if vcs_passed {
        println!("  ✓ VCS initialized");
    } else {
        println!("  ✗ No VCS found");
        println!("    Run 'scp init --vcs jj' or 'scp init --vcs git'");
    }

    println!("\n[2/5] Checking dependencies...");
    if dep_jj_found {
        println!("  ✓ jj found");
    } else if dep_git_found {
        println!("  ✓ git found");
    } else {
        println!("  ✗ No VCS CLI found (jj or git)");
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

    let all_passed = vcs_passed && (dep_jj_found || dep_git_found);

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
            let lock_patterns = [".jj", ".git"];
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
        Err(Error::Internal("Diagnostics failed".into()))
    }
}
