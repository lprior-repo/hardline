//! Rebase commands - restack, rebase, move, and duplicate

use std::process::Command;

use scp_core::{Error, Result};

/// Pure validation: ensures identifier is not empty
fn validate_identifier(name: &str, description: &str) -> Result<()> {
    if name.is_empty() {
        Err(Error::InvalidIdentifier(format!(
            "{} cannot be empty",
            description
        )))
    } else {
        Ok(())
    }
}

/// Action: runs jj command with given args, returns output
fn run_jj_command(args: &[&str]) -> Result<std::process::Output> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(Error::Io)
}

/// Pure calculation: processes jj output, returns success or error
fn process_jj_output(output: &std::process::Output) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(Error::VcsRebaseFailed(stderr))
    }
}

/// Pure calculation: extracts stdout as trimmed string
fn extract_stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Action: prints stdout if non-empty
fn print_stdout_if_present(output: &std::process::Output) {
    let stdout = extract_stdout(output);
    if !stdout.is_empty() {
        println!("{}", stdout);
    }
}

pub fn restack() -> Result<()> {
    run_jj_command(&["restack"]).and_then(|output| {
        process_jj_output(&output)?;
        print_stdout_if_present(&output);
        println!("✓ Restacked successfully");
        Ok(())
    })
}

pub fn rebase(dest: &str) -> Result<()> {
    validate_identifier(dest, "destination")?;

    run_jj_command(&["rebase", "-d", dest]).and_then(|output| {
        process_jj_output(&output)?;
        print_stdout_if_present(&output);
        println!("✓ Rebased onto {}", dest);
        Ok(())
    })
}

pub fn mv(source: &str, dest: &str) -> Result<()> {
    validate_identifier(source, "source")?;
    validate_identifier(dest, "destination")?;

    run_jj_command(&["move", "--from", source, "--to", dest]).and_then(|output| {
        process_jj_output(&output)?;
        print_stdout_if_present(&output);
        println!("✓ Moved changes from {} to {}", source, dest);
        Ok(())
    })
}

pub fn duplicate(revision: &str) -> Result<()> {
    validate_identifier(revision, "revision")?;

    run_jj_command(&["duplicate", revision]).and_then(|output| {
        process_jj_output(&output)?;
        print_stdout_if_present(&output);
        println!("✓ Duplicated {}", revision);
        Ok(())
    })
}
