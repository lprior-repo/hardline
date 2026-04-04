//! Action functions for the can-i command handler (Tier 3).
//!
//! I/O operations that check whether a given action is permitted.
//! Delegates permission checks to pure data helpers for known actions
//! and performs filesystem/workspace state checks as needed.

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{default_allowed_output, CanIOptions, CanIOutput, Prerequisite};

/// Execute the can-i command with the given options.
///
/// # Errors
///
/// Returns an error if the action string is empty.
pub fn run_can_i(options: &CanIOptions) -> Result<()> {
    if options.action.is_empty() {
        return Err(Error::validation_error("Action must not be empty"));
    }

    let output = check_permission(options)?;

    if output.permitted {
        Output::success(&format!("Yes, you can: {}", options.action));
        if let Some(resource) = &output.resource {
            Output::info(&format!("  Resource: {resource}"));
        }
        Output::info(&format!("  Reason: {}", output.reason));
    } else {
        Output::info(&format!("No, you cannot: {}", options.action));
        if let Some(resource) = &output.resource {
            Output::info(&format!("  Resource: {resource}"));
        }
        Output::info(&format!("  Reason: {}", output.reason));

        if !output.prerequisites.is_empty() {
            Output::info("");
            Output::info("Prerequisites:");
            for prereq in &output.prerequisites {
                let icon = if prereq.passed { "PASS" } else { "FAIL" };
                Output::info(&format!(
                    "  [{icon}] {}: {}",
                    prereq.check, prereq.description
                ));
            }
        }

        if !output.fix_commands.is_empty() {
            Output::info("");
            Output::info("To fix, run:");
            for cmd in &output.fix_commands {
                Output::info(&format!("  {cmd}"));
            }
        }
    }

    Ok(())
}

/// Check whether the requested action is permitted.
///
/// Dispatches to specific check functions for known actions, or returns
/// a default "generally allowed" result for unknown actions.
fn check_permission(options: &CanIOptions) -> Result<CanIOutput> {
    let action = options.action.as_str();
    let resource = options.resource.as_deref();

    match action {
        "add" | "work" => Ok(check_can_add(resource)),
        "remove" => Ok(check_can_remove(resource)),
        "done" => Ok(check_can_done(resource)),
        "merge" => Ok(check_can_merge(resource)),
        "undo" => Ok(check_can_undo()),
        "sync" => Ok(check_can_sync(resource)),
        "spawn" => Ok(check_can_spawn(resource)),
        _ => Ok(default_allowed_output(action, resource)),
    }
}

/// Check if the user can add a workspace.
fn check_can_add(resource: Option<&str>) -> CanIOutput {
    let mut prerequisites = Vec::new();

    // Check if we are inside a git repository
    let in_git_repo = is_git_repo();
    prerequisites.push(Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    });

    // Check if the workspace name is available
    let name_available = match resource {
        Some(name) => !workspace_exists(name),
        None => true,
    };
    if resource.is_some() {
        prerequisites.push(Prerequisite {
            check: "name_available".to_string(),
            passed: name_available,
            description: if name_available {
                "Workspace name is available".to_string()
            } else {
                "Workspace name already exists".to_string()
            },
        });
    }

    let permitted = in_git_repo && name_available;
    let reason = if permitted {
        "Can create workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Workspace name already exists".to_string()
    };

    let fix_commands = if !in_git_repo {
        vec!["git init".to_string()]
    } else if !name_available {
        resource.map_or_else(Vec::new, |name| {
            vec![format!("scp workspace remove {name}")]
        })
    } else {
        vec![]
    };

    CanIOutput {
        permitted,
        action: "add".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands,
    }
}

/// Check if the user can remove a workspace.
fn check_can_remove(resource: Option<&str>) -> CanIOutput {
    let mut prerequisites = Vec::new();

    let in_git_repo = is_git_repo();
    prerequisites.push(Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    });

    // Check if the workspace exists
    let workspace_exists = match resource {
        Some(name) => workspace_exists(name),
        None => false,
    };
    if resource.is_some() {
        prerequisites.push(Prerequisite {
            check: "workspace_exists".to_string(),
            passed: workspace_exists,
            description: if workspace_exists {
                "Workspace exists".to_string()
            } else {
                "Workspace does not exist".to_string()
            },
        });
    }

    let permitted = in_git_repo && (resource.is_none() || workspace_exists);
    let reason = if permitted {
        "Can remove workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Workspace does not exist".to_string()
    };

    CanIOutput {
        permitted,
        action: "remove".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

/// Check if the user can complete (done) a workspace.
fn check_can_done(resource: Option<&str>) -> CanIOutput {
    let mut prerequisites = Vec::new();

    let in_git_repo = is_git_repo();
    prerequisites.push(Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    });

    // Check if on main branch or in a workspace
    let in_workspace = is_in_worktree();
    let resource_specified = resource.is_some();
    prerequisites.push(Prerequisite {
        check: "in_workspace_or_specified".to_string(),
        passed: in_workspace || resource_specified,
        description: if in_workspace {
            "Inside a worktree".to_string()
        } else if resource_specified {
            "Workspace specified".to_string()
        } else {
            "Not in a worktree and no workspace specified".to_string()
        },
    });

    let permitted = in_git_repo && (in_workspace || resource_specified);
    let reason = if permitted {
        "Can complete and merge workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Not in a worktree - specify workspace or cd to worktree".to_string()
    };

    CanIOutput {
        permitted,
        action: "done".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

/// Check if the user can merge a workspace (same as done).
fn check_can_merge(resource: Option<&str>) -> CanIOutput {
    let mut output = check_can_done(resource);
    output.action = "merge".to_string();
    output
}

/// Check if the user can undo the last operation.
fn check_can_undo() -> CanIOutput {
    let mut prerequisites = Vec::new();

    // Check if undo log exists
    let undo_log_exists = undo_log_exists();
    prerequisites.push(Prerequisite {
        check: "undo_log_exists".to_string(),
        passed: undo_log_exists,
        description: if undo_log_exists {
            "Undo log exists".to_string()
        } else {
            "No undo log available".to_string()
        },
    });

    let permitted = undo_log_exists;
    let reason = if permitted {
        "Can undo last operation".to_string()
    } else {
        "No undo log - nothing to undo".to_string()
    };

    CanIOutput {
        permitted,
        action: "undo".to_string(),
        resource: None,
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

/// Check if the user can sync.
fn check_can_sync(resource: Option<&str>) -> CanIOutput {
    let mut prerequisites = Vec::new();

    let in_git_repo = is_git_repo();
    prerequisites.push(Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    });

    // Check if there are worktrees to sync
    let has_worktrees = has_worktrees();
    prerequisites.push(Prerequisite {
        check: "has_worktrees".to_string(),
        passed: has_worktrees || resource.is_some(),
        description: if has_worktrees {
            "Worktrees available to sync".to_string()
        } else if resource.is_some() {
            "Workspace specified".to_string()
        } else {
            "No worktrees to sync".to_string()
        },
    });

    let permitted = in_git_repo && (has_worktrees || resource.is_some());
    let reason = if permitted {
        "Can sync workspaces".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "No worktrees to sync".to_string()
    };

    CanIOutput {
        permitted,
        action: "sync".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

/// Check if the user can spawn an agent session.
fn check_can_spawn(resource: Option<&str>) -> CanIOutput {
    let mut prerequisites = Vec::new();

    let in_git_repo = is_git_repo();
    prerequisites.push(Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    });

    // Check if bead ID is provided
    let bead_provided = resource.is_some();
    prerequisites.push(Prerequisite {
        check: "bead_provided".to_string(),
        passed: bead_provided,
        description: if bead_provided {
            "Bead ID provided".to_string()
        } else {
            "No bead ID specified".to_string()
        },
    });

    let permitted = in_git_repo && bead_provided;
    let reason = if permitted {
        "Can spawn agent session".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Bead ID required".to_string()
    };

    CanIOutput {
        permitted,
        action: "spawn".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

// ============================================================================
// Filesystem / State Helpers (I/O, but lightweight)
// ============================================================================

/// Check if the current directory is inside a git repository.
fn is_git_repo() -> bool {
    std::env::current_dir().is_ok_and(|d| d.join(".git").exists())
}

/// Check if the current directory is inside a git worktree.
fn is_in_worktree() -> bool {
    std::env::current_dir().is_ok_and(|d| {
        // A worktree has a .git file (not directory) pointing to the main repo
        let git_path = d.join(".git");
        git_path.exists() && git_path.is_file()
    })
}

/// Check if a workspace directory exists in the parent's worktree area.
fn workspace_exists(name: &str) -> bool {
    std::env::current_dir().is_ok_and(|d| {
        let workspace_path = d
            .parent()
            .map(|p| p.join(name))
            .unwrap_or_else(|| d.join(name));
        workspace_path.exists()
    })
}

/// Check if the undo log file exists.
fn undo_log_exists() -> bool {
    std::env::current_dir().is_ok_and(|d| {
        let undo_path = Path::new(&d).join(".scp").join("undo.log");
        undo_path.exists()
    })
}

/// Check if there are any git worktrees present.
fn has_worktrees() -> bool {
    std::process::Command::new("git")
        .args(["worktree", "list"])
        .output()
        .is_ok_and(|out| {
            let count = String::from_utf8_lossy(&out.stdout).lines().count();
            count > 1 // More than just the main worktree
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::can_i::data::CanIOptions;

    // -- run_can_i with empty action should error --

    #[test]
    fn run_can_i_empty_action_returns_error() {
        let options = CanIOptions {
            action: String::new(),
            resource: None,
        };
        let result = run_can_i(&options);
        assert!(result.is_err());
    }

    // -- run_can_i with known actions should succeed --

    #[test]
    fn run_can_i_unknown_action_succeeds() {
        // Unknown actions are generally allowed
        let options = CanIOptions {
            action: "custom-action".to_string(),
            resource: None,
        };
        let result = run_can_i(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_can_i_undo_succeeds() {
        // undo always succeeds (just reports status), even if no undo log
        let options = CanIOptions {
            action: "undo".to_string(),
            resource: None,
        };
        let result = run_can_i(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_can_i_spawn_no_resource_succeeds() {
        // spawn without resource reports not-permitted but does not error
        let options = CanIOptions {
            action: "spawn".to_string(),
            resource: None,
        };
        let result = run_can_i(&options);
        assert!(result.is_ok());
    }

    // -- check_permission dispatches correctly --

    #[test]
    fn check_permission_unknown_action_is_allowed() {
        let options = CanIOptions {
            action: "teleport".to_string(),
            resource: None,
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert!(output.permitted);
        assert_eq!(output.action, "teleport");
    }

    #[test]
    fn check_permission_add_dispatches() {
        let options = CanIOptions {
            action: "add".to_string(),
            resource: Some("test-ws".to_string()),
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "add");
        assert_eq!(output.resource.as_deref(), Some("test-ws"));
    }

    #[test]
    fn check_permission_work_dispatches_as_add() {
        let options = CanIOptions {
            action: "work".to_string(),
            resource: None,
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "add");
    }

    #[test]
    fn check_permission_remove_dispatches() {
        let options = CanIOptions {
            action: "remove".to_string(),
            resource: Some("some-ws".to_string()),
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "remove");
    }

    #[test]
    fn check_permission_done_dispatches() {
        let options = CanIOptions {
            action: "done".to_string(),
            resource: None,
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "done");
    }

    #[test]
    fn check_permission_merge_dispatches_as_merge() {
        let options = CanIOptions {
            action: "merge".to_string(),
            resource: None,
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "merge");
    }

    #[test]
    fn check_permission_sync_dispatches() {
        let options = CanIOptions {
            action: "sync".to_string(),
            resource: None,
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "sync");
    }

    #[test]
    fn check_permission_spawn_dispatches() {
        let options = CanIOptions {
            action: "spawn".to_string(),
            resource: Some("bead-123".to_string()),
        };
        let output = check_permission(&options).expect("check_permission should not fail");
        assert_eq!(output.action, "spawn");
        assert_eq!(output.resource.as_deref(), Some("bead-123"));
    }

    // -- Prerequisite construction --

    #[test]
    fn prerequisite_fields_match() {
        let prereq = Prerequisite {
            check: "test_check".to_string(),
            passed: false,
            description: "It failed".to_string(),
        };
        assert_eq!(prereq.check, "test_check");
        assert!(!prereq.passed);
        assert_eq!(prereq.description, "It failed");
    }

    // -- CanIOutput construction --

    #[test]
    fn can_i_output_denied() {
        let output = CanIOutput {
            permitted: false,
            action: "spawn".to_string(),
            resource: None,
            reason: "No bead ID".to_string(),
            prerequisites: vec![Prerequisite {
                check: "bead_provided".to_string(),
                passed: false,
                description: "No bead ID specified".to_string(),
            }],
            fix_commands: vec!["Provide a bead ID".to_string()],
        };
        assert!(!output.permitted);
        assert_eq!(output.prerequisites.len(), 1);
        assert_eq!(output.fix_commands.len(), 1);
    }
}
