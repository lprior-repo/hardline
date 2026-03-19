//! Stash commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use tap::tap::Tap;

fn build_git_stash_save_command(
    cwd: &std::path::Path,
    message: Option<&str>,
    include_untracked: bool,
    patch: bool,
) -> Command {
    Command::new("git")
        .tap(|cmd| cmd.arg("stash").arg("push"))
        .tap(|cmd| {
            if let Some(msg) = message {
                cmd.arg("-m").arg(msg);
            }
        })
        .tap(|cmd| {
            if include_untracked {
                cmd.arg("-u");
            }
        })
        .tap(|cmd| {
            if patch {
                cmd.arg("-p");
            }
        })
        .current_dir(cwd)
}

fn build_git_stash_pop_command(
    cwd: &std::path::Path,
    stash: Option<&str>,
    restore_index: bool,
) -> Command {
    Command::new("git")
        .tap(|cmd| cmd.arg("stash").arg("pop"))
        .tap(|cmd| {
            if let Some(s) = stash {
                cmd.arg(s);
            }
        })
        .tap(|cmd| {
            if restore_index {
                cmd.arg("--index");
            }
        })
        .current_dir(cwd)
}

fn build_git_stash_list_command(cwd: &std::path::Path) -> Command {
    Command::new("git").args(["stash", "list"]).current_dir(cwd)
}

fn build_git_stash_drop_command(cwd: &std::path::Path, stash: &str, force: bool) -> Command {
    Command::new("git")
        .tap(|cmd| cmd.arg("stash").arg("drop"))
        .tap(|cmd| {
            if force {
                cmd.arg("-f");
            }
        })
        .arg(stash)
        .current_dir(cwd)
}

fn build_git_stash_show_command(cwd: &std::path::Path, stash_ref: &str, stat: bool) -> Command {
    Command::new("git")
        .tap(|cmd| cmd.arg("stash").arg("show"))
        .tap(|cmd| {
            if stat {
                cmd.arg("--stat");
            }
        })
        .arg(stash_ref)
        .current_dir(cwd)
}

// Pure calculation functions (Data→Calc→Actions)

fn validate_git_vcs(cwd: &std::path::Path) -> Result<()> {
    detect_vcs(cwd)
        .filter(|&vcs_type| vcs_type == scp_core::vcs::VcsType::Git)
        .ok_or(Error::VcsNotInitialized)
        .and_then(|_| {
            Err(Error::InvalidState(
                "stash is only supported for Git repositories".to_string(),
            ))
        })
}

fn execute_git_command(mut cmd: Command) -> Result<std::process::Output> {
    cmd.output().map_err(Error::Io)
}

fn handle_command_success(output: &std::process::Output) -> bool {
    output.status.success()
}

fn get_stderr_as_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn get_stdout_as_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn create_vcs_conflict_error(command: &str, stderr: &str) -> Error {
    Error::VcsConflict(command.to_string(), stderr.to_string())
}

fn get_current_working_dir() -> Result<std::path::PathBuf> {
    std::env::current_dir().map_err(Error::Io)
}

fn output_success_message(msg: &str) {
    Output::success(msg);
}

fn output_info_message(msg: &str) {
    Output::info(msg);
}

fn print_content(content: &str) {
    print!("{}", content);
}

fn is_output_empty(output: &std::process::Output) -> bool {
    get_stdout_as_string(output).trim().is_empty()
}

// Action functions that compose pure calculations

pub fn save(message: Option<&str>, include_untracked: bool, patch: bool) -> Result<()> {
    get_current_working_dir()
        .and_then(|cwd| validate_git_vcs(&cwd).map(|_| cwd))
        .and_then(|cwd| {
            execute_git_command(build_git_stash_save_command(
                &cwd,
                message,
                include_untracked,
                patch,
            ))
        })
        .and_then(|output| {
            if handle_command_success(&output) {
                let msg = message.unwrap_or("changes");
                output_success_message(&format!("Stashed: {}", msg));
                Ok(())
            } else {
                Err(create_vcs_conflict_error(
                    "git stash",
                    &get_stderr_as_string(&output),
                ))
            }
        })
}

pub fn pop(stash: Option<&str>, restore_index: bool) -> Result<()> {
    let cwd = get_current_working_dir()?;
    validate_git_vcs(&cwd)?;

    let output = execute_git_command(build_git_stash_pop_command(&cwd, stash, restore_index))?;

    if handle_command_success(&output) {
        output_success_message("Applied stash and removed from stash list");
        Ok(())
    } else {
        Err(create_vcs_conflict_error(
            "git stash pop",
            &get_stderr_as_string(&output),
        ))
    }
}

pub fn list() -> Result<()> {
    let cwd = get_current_working_dir()?;
    validate_git_vcs(&cwd)?;

    let output = execute_git_command(build_git_stash_list_command(&cwd))?;

    if handle_command_success(&output) {
        if is_output_empty(&output) {
            output_info_message("No stashed changes");
        } else {
            print_content(&get_stdout_as_string(&output));
        }
        Ok(())
    } else {
        Err(create_vcs_conflict_error(
            "git stash list",
            &get_stderr_as_string(&output),
        ))
    }
}

pub fn drop(stash: &str, force: bool) -> Result<()> {
    let cwd = get_current_working_dir()?;
    validate_git_vcs(&cwd)?;

    let output = execute_git_command(build_git_stash_drop_command(&cwd, stash, force))?;

    if handle_command_success(&output) {
        output_success_message(&format!("Dropped stash: {}", stash));
        Ok(())
    } else {
        Err(create_vcs_conflict_error(
            "git stash drop",
            &get_stderr_as_string(&output),
        ))
    }
}

pub fn show(stash: Option<&str>, stat: bool) -> Result<()> {
    let cwd = get_current_working_dir()?;
    validate_git_vcs(&cwd)?;

    let stash_ref = stash.unwrap_or("stash@{0}");

    let output = execute_git_command(build_git_stash_show_command(&cwd, stash_ref, stat))?;

    if handle_command_success(&output) {
        print_content(&get_stdout_as_string(&output));
        Ok(())
    } else {
        Err(create_vcs_conflict_error(
            "git stash show",
            &get_stderr_as_string(&output),
        ))
    }
}
