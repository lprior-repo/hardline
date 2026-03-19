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
        .map_err(Error::Io)
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
    let disk_lines = disk_usage_lines(cwd);

    let lock_messages = lock_files_in_path(cwd)
        .iter()
        .map(|p| format!("  ⚠ Found lock file: {:?}", p))
        .collect::<Vec<_>>();

    let status_message = vcs::create_backend(cwd)
        .ok()
        .and_then(|be| be.status().ok())
        .and_then(working_copy_status_message);

    [
        disk_lines,
        lock_messages,
        status_message.into_iter().collect(),
    ]
    .concat()
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

fn print_config_check(result: &Result<bool>) {
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

fn print_workspaces_check(result: &Result<usize>) {
    println!("\n[4/5] Checking workspaces...");
    match result {
        Ok(count) => {
            if *count > 0 {
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

/// Diagnostic check results - pure data structure
struct DiagnosticResults {
    vcs_available: bool,
    jj_found: bool,
    git_found: bool,
    config_result: Result<bool>,
    workspaces_result: Result<usize>,
}

/// Gather all diagnostic results - pure calculation
fn gather_diagnostic_results() -> DiagnosticResults {
    DiagnosticResults {
        vcs_available: check_vcs_available().as_ref().copied().unwrap_or(false),
        jj_found: check_dependency("jj").as_ref().copied().unwrap_or(false),
        git_found: check_dependency("git").as_ref().copied().unwrap_or(false),
        config_result: check_config_exists(),
        workspaces_result: check_workspaces_count(),
    }
}

/// Compute overall pass/fail - pure calculation
fn compute_all_passed(results: &DiagnosticResults) -> bool {
    results.vcs_available && (results.jj_found || results.git_found)
}

/// Run health checks
pub fn run(full: bool) -> Result<()> {
    println!("Running SCP diagnostics...\n");

    let results = gather_diagnostic_results();

    print_vcs_check(results.vcs_available);
    print_dependency_check(results.jj_found, results.git_found);
    print_config_check(&results.config_result);
    print_workspaces_check(&results.workspaces_result);

    let all_passed = compute_all_passed(&results);

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
