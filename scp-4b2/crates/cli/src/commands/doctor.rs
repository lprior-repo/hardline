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

// Full diagnostics - pure calculation functions (Data→Calc pattern)

#[cfg(unix)]
fn disk_usage_lines(path: &std::path::Path) -> Vec<String> {
    std::process::Command::new("df")
        .arg("-h")
        .arg(path)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .skip(1)
                .map(|l| format!("  Disk: {}", l))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(unix))]
fn disk_usage_lines(_path: &std::path::Path) -> Vec<String> {
    Vec::new()
}

fn lock_files_in_path(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    [".jj", ".git"]
        .iter()
        .flat_map(|pattern| {
            let lock_path = path.join(pattern).join("lock");
            lock_path.exists().then_some(lock_path)
        })
        .collect()
}

fn working_copy_status_message(status: scp_core::vcs::VcsStatus) -> Option<String> {
    match status {
        scp_core::vcs::VcsStatus::Conflicted => Some("  ✗ Working copy has conflicts!".to_string()),
        scp_core::vcs::VcsStatus::Dirty => {
            Some("  ⚠ Working copy has uncommitted changes".to_string())
        }
        _ => None,
    }
}

fn full_diagnostics_messages(cwd: &std::path::Path) -> Vec<String> {
    let mut messages = Vec::new();

    let disk_lines = disk_usage_lines(cwd);
    messages.extend(disk_lines);

    let locks = lock_files_in_path(cwd);
    for lock_path in locks {
        messages.push(format!("  ⚠ Found lock file: {:?}", lock_path));
    }

    if let Ok(be) = vcs::create_backend(cwd) {
        if let Ok(status) = be.status() {
            if let Some(msg) = working_copy_status_message(status) {
                messages.push(msg);
            }
        }
    }

    messages
}

fn print_vcs_check(passed: bool) {
    println!("[1/5] Checking VCS...");
    if passed {
        println!("  ✓ VCS initialized");
    } else {
        println!("  ✗ No VCS found");
        println!("    Run 'scp init --vcs jj' or 'scp init --vcs git'");
    }
}

fn print_dependency_check(jj_found: bool, git_found: bool) {
    println!("\n[2/5] Checking dependencies...");
    if jj_found {
        println!("  ✓ jj found");
    } else if git_found {
        println!("  ✓ git found");
    } else {
        println!("  ✗ No VCS CLI found (jj or git)");
    }
}

fn print_config_check(result: Result<bool>) {
    println!("\n[3/5] Checking configuration...");
    match result {
        Ok(true) => println!("  ✓ Config valid"),
        Ok(false) => {
            println!("  ⚠ No config found (will use defaults)");
        }
        Err(e) => {
            println!("  ✗ Config error: {}", e);
        }
    }
}

fn print_workspaces_check(result: Result<usize>) {
    println!("\n[4/5] Checking workspaces...");
    match result {
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
}

fn print_full_diagnostics(cwd: &std::path::Path) {
    let messages = full_diagnostics_messages(cwd);
    for msg in messages {
        println!("{}", msg);
    }
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
    let _workspaces_count = check_workspaces_result.as_ref().copied().unwrap_or(0);

    print_vcs_check(vcs_passed);
    print_dependency_check(dep_jj_found, dep_git_found);
    print_config_check(check_config_result);
    print_workspaces_check(check_workspaces_result);

    let all_passed = vcs_passed && (dep_jj_found || dep_git_found);

    if full {
        println!("\n[5/5] Running full diagnostics...");
        let cwd = std::env::current_dir().map_err(Error::Io)?;
        print_full_diagnostics(&cwd);
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
