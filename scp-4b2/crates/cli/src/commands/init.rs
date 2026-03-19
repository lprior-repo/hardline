//! Initialize command

use scp_core::{Error, Result};
use std::path::Path;

/// Returns the marker directory name for a given VCS type.
/// Returns None for unknown VCS types.
fn vcs_marker(vcs_type: &str) -> Option<&'static str> {
    match vcs_type {
        "jj" => Some(".jj"),
        "git" => Some(".git"),
        _ => None,
    }
}

/// Returns the init arguments for a given VCS type.
/// Returns None for unknown VCS types.
fn vcs_init_args(vcs_type: &str) -> Option<&'static [&'static str]> {
    match vcs_type {
        "jj" => Some(&["init", "--name", "main"][..]),
        "git" => Some(&["init"][..]),
        _ => None,
    }
}

/// Checks if a VCS tool is installed by running `vcs_type --version`.
fn check_vcs_installed(vcs_type: &str) -> Result<()> {
    std::process::Command::new(vcs_type)
        .arg("--version")
        .output()
        .map_err(Error::Io)
        .map(|_| ())
}

/// Checks if a VCS is already initialized by looking for its marker directory.
fn is_vcs_initialized(dir: &Path, marker: &str) -> bool {
    dir.join(marker).exists()
}

/// Gets the current working directory.
fn get_cwd() -> Result<std::path::PathBuf> {
    std::env::current_dir().map_err(Error::Io)
}

/// Runs the VCS init command and validates success.
fn run_vcs_init(dir: &Path, vcs_type: &str, args: &[&str]) -> Result<()> {
    std::process::Command::new(vcs_type)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(Error::Io)
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::Internal(format!(
                    "Failed to init {}: {}",
                    vcs_type,
                    String::from_utf8_lossy(&output.stderr)
                )))
            }
        })
}

/// Initialize SCP in current directory
pub fn run(vcs_type: &str) -> Result<()> {
    println!("Initializing Source Control Plane...");

    let marker = vcs_marker(vcs_type)
        .ok_or_else(|| Error::ConfigInvalid(format!("Unknown VCS type: {}", vcs_type)))?;

    let args = vcs_init_args(vcs_type)
        .ok_or_else(|| Error::ConfigInvalid(format!("Unknown VCS type: {}", vcs_type)))?;

    let cwd = get_cwd()?;

    if vcs_type == "jj" {
        check_vcs_installed(vcs_type)?;
    }

    if is_vcs_initialized(&cwd, marker) {
        println!("Already initialized with JJ");
        return Ok(());
    }

    run_vcs_init(&cwd, vcs_type, args)?;

    println!("✓ Initialized {} in {:?}", vcs_type, cwd);
    Ok(())
}
