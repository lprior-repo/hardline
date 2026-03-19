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

/// Validates VCS type and returns (marker, args) pair.
/// Returns error for unknown VCS types.
fn validate_vcs(vcs_type: &str) -> Result<(&'static str, &'static [&'static str])> {
    let marker = vcs_marker(vcs_type)
        .ok_or_else(|| Error::ConfigInvalid(format!("Unknown VCS type: {}", vcs_type)))?;
    let args = vcs_init_args(vcs_type)
        .ok_or_else(|| Error::ConfigInvalid(format!("Unknown VCS type: {}", vcs_type)))?;
    Ok((marker, args))
}

/// Checks if a VCS tool is installed by running `vcs_type --version`.
fn check_vcs_installed(vcs_type: &str) -> Result<()> {
    std::process::Command::new(vcs_type)
        .arg("--version")
        .output()
        .map_err(Error::Io)
        .map(|_| ())
}

/// Determines if JJ-specific pre-init check is needed.
fn needs_jj_install_check(vcs_type: &str) -> bool {
    vcs_type == "jj"
}

/// Checks if a VCS is already initialized by looking for its marker directory.
fn is_vcs_initialized(dir: &Path, marker: &str) -> bool {
    dir.join(marker).exists()
}

/// Gets the current working directory.
fn get_cwd() -> Result<std::path::PathBuf> {
    std::env::current_dir().map_err(Error::Io)
}

/// Formats the already-initialized message for display.
fn format_already_initialized_msg(vcs_type: &str) -> String {
    format!("Already initialized with {}", vcs_type)
}

/// Formats the success message for display.
fn format_success_msg(vcs_type: &str, dir: &Path) -> String {
    format!("✓ Initialized {} in {:?}", vcs_type, dir)
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

    let (marker, args) = validate_vcs(vcs_type)?;
    let cwd = get_cwd()?;

    needs_jj_install_check(vcs_type)
        .then(|| check_vcs_installed(vcs_type))
        .transpose()?;

    if is_vcs_initialized(&cwd, marker) {
        println!("{}", format_already_initialized_msg(vcs_type));
        return Ok(());
    }

    run_vcs_init(&cwd, vcs_type, args)?;

    println!("{}", format_success_msg(vcs_type, &cwd));
    Ok(())
}
