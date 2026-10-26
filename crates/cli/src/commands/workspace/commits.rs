//! Workspace diff and commit commands

use std::process::Command;

use scp_core::{output::Output, Error};

/// Detail for a failed `git commit`.
///
/// Git writes failure text such as "nothing to commit, working tree clean"
/// to stdout, not stderr, so stderr alone is often empty. Prefer stderr,
/// fall back to stdout, and only then to a short generic sentence.
fn git_commit_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    "git commit failed".to_string()
}

/// Fail cleanly when the cwd is not inside a git work tree.
///
/// Without this guard, `git diff` outside a repo falls into --no-index
/// mode and dumps ~200 lines of usage text to stderr, which callers
/// would otherwise surface verbatim as the error.
fn ensure_git_repo(cwd: &std::path::Path) -> Result<(), Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::vcs_conflict(
            "diff",
            "not a git repository (or any of the parent directories)",
        ))
    }
}

/// Show diff
pub fn diff(path: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    ensure_git_repo(&cwd)?;

    let mut cmd = super::operations::build_git_diff_command(&cwd, path);
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("diff", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if Output::json_output() {
        let mut value = serde_json::json!({
            "diff": stdout,
        });
        if let Some(p) = path {
            value["path"] = serde_json::json!(p);
        }
        Output::json(&value);
    } else {
        Output::info(&stdout);
    }

    Ok(())
}

/// Show uncommitted changes
pub fn uncommitted() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    ensure_git_repo(&cwd)?;

    let output = Command::new("git")
        .args(["diff"])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("diff", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "diff": stdout,
        }));
    } else {
        Output::info(&stdout);
    }

    Ok(())
}

/// Commit changes
pub fn commit(message: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        return Err(Error::vcs_conflict("commit", git_commit_detail(&output)));
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "result": "committed",
            "message": message,
        }));
    } else {
        Output::success("Committed changes");
    }

    Ok(())
}

/// Show workspace log
pub fn log(limit: Option<usize>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let mut args = vec!["log"];
    let limit_str = limit.map(|l| l.to_string());
    #[allow(unused_assignments)]
    if let Some(ref s) = limit_str {
        args.push("-n");
        args.push(s);
    }

    let output = Command::new("git").args(&args).current_dir(&cwd).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("log", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "log": stdout,
            "limit": limit,
        }));
    } else {
        Output::info(&stdout);
    }

    Ok(())
}
