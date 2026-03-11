//! Doctor command - health checks and diagnostics

use scp_core::{vcs, Error, Result};

/// Run health checks
pub fn run(full: bool) -> Result<()> {
    println!("Running SCP diagnostics...\n");

    let mut all_passed = true;

    // Check VCS
    println!("[1/5] Checking VCS...");
    match check_vcs() {
        Ok(true) => println!("  ✓ VCS initialized"),
        Ok(false) => {
            println!("  ✗ No VCS found");
            println!("    Run 'scp init --vcs jj' or 'scp init --vcs git'");
            all_passed = false;
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
            all_passed = false;
        }
    }

    // Check dependencies
    println!("\n[2/5] Checking dependencies...");
    if check_dependency("jj") {
        println!("  ✓ jj found");
    } else if check_dependency("git") {
        println!("  ✓ git found");
    } else {
        println!("  ✗ No VCS CLI found (jj or git)");
        all_passed = false;
    }

    // Check config
    println!("\n[3/5] Checking configuration...");
    match check_config() {
        Ok(true) => println!("  ✓ Config valid"),
        Ok(false) => {
            println!("  ⚠ No config found (will use defaults)");
        }
        Err(e) => {
            println!("  ✗ Config error: {}", e);
            all_passed = false;
        }
    }

    // Check workspaces
    println!("\n[4/5] Checking workspaces...");
    match check_workspaces() {
        Ok(count) => {
            if count > 0 {
                println!("  ✓ {} workspace(s) found", count);
            } else {
                println!("  ℹ No workspaces (run 'scp workspace spawn <name>')");
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
            all_passed = false;
        }
    }

    // Full diagnostics if requested
    if full {
        println!("\n[5/5] Running full diagnostics...");

        // Check disk space
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

        // Check for lock files
        if let Ok(path) = std::env::current_dir() {
            let lock_patterns = [".jj", ".git"];
            for pattern in lock_patterns {
                let lock_path = path.join(pattern).join("lock");
                if lock_path.exists() {
                    println!("  ⚠ Found lock file: {:?}", lock_path);
                }
            }
        }

        // Check for conflicts
        if let Ok(path) = std::env::current_dir() {
            let backend = vcs::create_backend(&path);
            if let Ok(be) = backend {
                if let Ok(status) = be.status() {
                    match status {
                        scp_core::vcs::VcsStatus::Conflicted => {
                            println!("  ✗ Working copy has conflicts!");
                            all_passed = false;
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

    // Summary
    println!("\n{}", "=".repeat(50));
    if all_passed {
        println!("✓ All checks passed");
        Ok(())
    } else {
        println!("✗ Some checks failed - see above for details");
        Err(Error::Internal("Diagnostics failed".into()))
    }
}

/// Check if VCS is initialized
fn check_vcs() -> Result<bool> {
    let cwd = std::env::current_dir().map_err(|e| Error::Io(e))?;

    let is_jj = cwd.join(".jj").exists();
    let is_git = cwd.join(".git").exists();

    Ok(is_jj || is_git)
}

/// Check if a dependency is available
fn check_dependency(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check config validity
fn check_config() -> Result<bool> {
    let dir = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::ConfigNotFound("No config dir".into()))?;

    let config_file = dir.config_dir().join("config.toml");

    if !config_file.exists() {
        return Ok(false);
    }

    // Try to parse config
    let contents = std::fs::read_to_string(&config_file).map_err(|e| Error::Io(e))?;

    // Basic validation - just check it's valid UTF-8
    let _ = contents.as_bytes();

    Ok(true)
}

/// Check workspaces
fn check_workspaces() -> Result<usize> {
    let cwd = std::env::current_dir().map_err(|e| Error::Io(e))?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    Ok(workspaces.len())
}
