//! Session commands (from Isolate)

use scp_core::{output::Output, vcs, Error, Result};

/// List sessions
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if Output::json_output() {
        Output::json(&serde_json::json!({ "sessions": workspaces }));
    } else if workspaces.is_empty() {
        println!("No sessions found");
    } else {
        println!("Sessions:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            println!("  - {} on branch {}{}", ws.name, ws.branch, current);
        }
    }

    Ok(())
}

/// Show session status
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let backend = vcs::create_backend(&cwd)?;

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    let state = match vcs_status {
        scp_core::vcs::VcsStatus::Clean => "clean",
        scp_core::vcs::VcsStatus::Dirty => "dirty",
        scp_core::vcs::VcsStatus::Conflicted => "conflicted",
        scp_core::vcs::VcsStatus::Detached => "detached",
    };

    let log = backend.log(5)?;
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "branch": branch,
            "state": state,
            "recent_commits": log.iter().take(3).collect::<Vec<_>>(),
        }));
    } else {
        println!("Session Status:");
        println!("  Branch: {}", branch);
        println!("  State: {}", state);

        if !log.is_empty() {
            println!("  Recent commits:");
            for commit in log.iter().take(3) {
                println!("    - {}", commit.id.chars().take(8).collect::<String>());
                if !commit.message.is_empty() {
                    println!("      {}", commit.message.lines().next().unwrap_or(""));
                }
            }
        }
    }

    Ok(())
}

/// Focus (switch to) a session
pub fn focus(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    if !Output::json_output() {
        Output::info(&format!("Focusing session '{}'...", name));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    let path = backend.switch_workspace(name)?;
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "session": name,
            "focused": true,
            "path": path.display().to_string(),
        }));
    } else {
        Output::success(&format!("Focused session '{}'", name));
        Output::info(&format!("Path: {}", path.display()));
        Output::info(&format!(
            "Run `cd {}` to move your shell there.",
            path.display()
        ));
    }
    Ok(())
}

/// Submit session changes for review
pub fn submit(name: Option<&str>, auto_commit: bool, message: Option<&str>) -> Result<()> {
    let config = scp_core::config::global_config().load()?;
    let auto_commit = auto_commit || *config.session.auto_commit;
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspace_name = if let Some(n) = name {
        n.to_string()
    } else {
        let workspaces = backend.list_workspaces()?;
        workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.clone())
            .ok_or_else(|| Error::workspace_not_found("no current session".to_string()))?
    };

    if !Output::json_output() {
        Output::info(&format!("Submitting session '{}'...", workspace_name));
    }

    let vcs_status = backend.status()?;
    if vcs_status == scp_core::vcs::VcsStatus::Dirty {
        if auto_commit {
            if let Some(msg) = message {
                let output = std::process::Command::new("git")
                    .args(["commit", "-a", "-m", msg])
                    .current_dir(&cwd)
                    .output()
                    .map_err(|e| Error::io_error(e.to_string()))?;
                if !output.status.success() {
                    // Git writes commit failure text (e.g. "nothing to
                    // commit, working tree clean") to stdout, not stderr.
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let detail = if !stderr.is_empty() {
                        stderr
                    } else {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !stdout.is_empty() {
                            stdout
                        } else {
                            "git commit failed".to_string()
                        }
                    };
                    return Err(Error::vcs_conflict("commit", detail));
                }
            } else {
                return Err(Error::invalid_state(
                    "dirty working copy requires --message".to_string(),
                ));
            }
        } else {
            return Err(Error::working_copy_dirty());
        }
    }

    backend.push()?;
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "session": workspace_name,
            "submitted": true,
            "pushed": true,
        }));
    } else {
        Output::success("Pushed to remote");
        println!("✓ Submitted session '{}'", workspace_name);
    }
    Ok(())
}

/// Remove a session
pub fn remove(name: &str, _force: bool, merge: bool) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    if name == "main" {
        return Err(Error::invalid_state(
            "cannot remove the main session".to_string(),
        ));
    }

    if !Output::json_output() {
        Output::info(&format!("Removing session '{}'...", name));
    }

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    if merge {
        backend.rebase("main")?;
        if !Output::json_output() {
            Output::success("Merged with main");
        }
    }

    backend.delete_workspace(name)?;
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "session": name,
            "removed": true,
            "merged": merge,
        }));
    } else {
        Output::success(&format!("Removed session '{}'", name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_rejects_empty_name() {
        let result = focus("");
        assert!(result.is_err());
    }

    #[test]
    fn focus_returns_invalid_identifier_for_empty() {
        let result = focus("");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("empty"), "error should mention empty: {}", msg);
    }

    #[test]
    fn remove_rejects_empty_name() {
        let result = remove("", false, false);
        assert!(result.is_err());
    }

    #[test]
    fn remove_main_session_rejected() {
        let result = remove("main", false, false);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.to_lowercase().contains("main"),
            "error should mention main: {}",
            msg
        );
    }

    #[test]
    fn list_function_exists_and_returns_result() {
        let _fn_list: fn() -> Result<()> = list;
    }

    #[test]
    fn status_function_exists_and_returns_result() {
        let _fn_status: fn() -> Result<()> = status;
    }

    #[test]
    fn submit_function_accepts_all_params() {
        let _fn_submit: fn(Option<&str>, bool, Option<&str>) -> Result<()> = submit;
    }

    #[test]
    fn remove_accepts_force_and_merge_flags() {
        let _fn_remove: fn(&str, bool, bool) -> Result<()> = remove;
    }

    #[test]
    fn vcs_status_variants_used_in_session() {
        use scp_core::vcs::VcsStatus;
        // Used in status() function
        let _ = VcsStatus::Clean;
        let _ = VcsStatus::Dirty;
        let _ = VcsStatus::Conflicted;
        let _ = VcsStatus::Detached;
    }

    #[test]
    fn error_constructors_used_in_session() {
        let _ = Error::invalid_identifier("test");
        let _ = Error::workspace_not_found("test");
        let _ = Error::invalid_state("test");
        let _ = Error::working_copy_dirty();
        let _ = Error::vcs_conflict("repo", "msg");
    }
}
