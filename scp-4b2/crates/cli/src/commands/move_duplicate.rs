//! Move and Duplicate commands

use std::process::Command;

use scp_core::{Error, Result};

/// Run a jj command and return its output
fn run_jj_command(args: &[&str], cwd: &std::path::Path) -> Result<std::process::Output> {
    Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(Error::Io)
}

/// Check command output and extract stdout on success
fn process_jj_output(output: std::process::Output, command_name: &str) -> Result<String> {
    if !output.status.success() {
        return Err(Error::VcsConflict(
            command_name.to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Print stdout if it's not empty
fn print_stdout_if_present(stdout: &str) {
    if !stdout.is_empty() {
        println!("{stdout}");
    }
}

/// Move changes from one revision to another using jj move
pub fn move_changes(source: &str, dest: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    run_jj_command(&["move", "--from", source, "--to", dest], &cwd)
        .and_then(|output| process_jj_output(output, "jj move"))
        .inspect(|stdout| print_stdout_if_present(stdout))
        .map(|_| println!("✓ Moved changes from {source} to {dest}"))
        .map(|_| ())
}

/// Duplicate a revision using jj duplicate
pub fn duplicate(revision: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    run_jj_command(&["duplicate", revision], &cwd)
        .and_then(|output| process_jj_output(output, "jj duplicate"))
        .inspect(|stdout| print_stdout_if_present(stdout))
        .map(|_| println!("✓ Duplicated {revision}"))
        .map(|_| ())
}
