//! Action functions for the can-i command handler (Tier 3).
//!
//! I/O operations that check whether a given action is permitted.
//! Delegates permission checks to pure data helpers for known actions
//! and performs filesystem/workspace state checks as needed.

use std::path::Path;

use scp_core::{output::Output, Error, Result};

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
    display_result(&output);
    Ok(())
}

/// Display the permission check result to the user.
fn display_result(output: &CanIOutput) {
    if output.permitted {
        Output::success(&format!("Yes, you can: {}", output.action));
    } else {
        Output::info(&format!("No, you cannot: {}", output.action));
    }
    if let Some(resource) = &output.resource {
        Output::info(&format!("  Resource: {resource}"));
    }
    Output::info(&format!("  Reason: {}", output.reason));

    if !output.permitted {
        display_prerequisites(&output.prerequisites);
        display_fix_commands(&output.fix_commands);
    }
}

/// Display prerequisite check results.
fn display_prerequisites(prerequisites: &[Prerequisite]) {
    if prerequisites.is_empty() {
        return;
    }
    Output::info("");
    Output::info("Prerequisites:");
    for prereq in prerequisites {
        let icon = if prereq.passed { "PASS" } else { "FAIL" };
        Output::info(&format!(
            "  [{icon}] {}: {}",
            prereq.check, prereq.description
        ));
    }
}

/// Display suggested fix commands.
fn display_fix_commands(fix_commands: &[String]) {
    if fix_commands.is_empty() {
        return;
    }
    Output::info("");
    Output::info("To fix, run:");
    for cmd in fix_commands {
        Output::info(&format!("  {cmd}"));
    }
}

/// Check whether the requested action is permitted.
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

// ============================================================================
// Prerequisite Builder Helpers
// ============================================================================

/// Probe current filesystem state once and return cached results.
struct FsState {
    in_git_repo: bool,
    in_worktree: bool,
}

impl FsState {
    /// Probe filesystem state from the current working directory.
    fn probe() -> Self {
        let cwd = std::env::current_dir();
        let in_git_repo = cwd.as_ref().is_ok_and(|d| d.join(".git").exists());
        let in_worktree = cwd.is_ok_and(|d| {
            let git_path = d.join(".git");
            git_path.exists() && git_path.is_file()
        });
        Self {
            in_git_repo,
            in_worktree,
        }
    }
}

/// Build a git-repo prerequisite from cached state.
fn git_repo_prereq(in_git_repo: bool) -> Prerequisite {
    Prerequisite {
        check: "git_repo".to_string(),
        passed: in_git_repo,
        description: if in_git_repo {
            "Inside a git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
    }
}

/// Arguments for [`build_output`].
struct BuildOutputArgs<'a> {
    action: &'a str,
    resource: Option<&'a str>,
    prerequisites: Vec<Prerequisite>,
    permitted: bool,
    reason: String,
    fix_commands: Vec<String>,
}

/// Build an output from prerequisites, computing reason and fix commands.
fn build_output(args: BuildOutputArgs<'_>) -> CanIOutput {
    CanIOutput {
        permitted: args.permitted,
        action: args.action.to_string(),
        resource: args.resource.map(String::from),
        reason: args.reason,
        prerequisites: args.prerequisites,
        fix_commands: args.fix_commands,
    }
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
            count > 1
        })
}

// ============================================================================
// Action-Specific Check Functions
// ============================================================================

/// Check if the user can add a workspace.
fn check_can_add(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let name_available = !resource.is_some_and(workspace_exists);
    let permitted = fs.in_git_repo && name_available;

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        name_available_prereq(name_available),
    ];
    let reason = add_reason(permitted, fs.in_git_repo);
    let fix_commands = add_fix_commands(fs.in_git_repo, name_available, resource);
    build_output(BuildOutputArgs {
        action: "add",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands,
    })
}

/// Build the name-availability prerequisite.
fn name_available_prereq(available: bool) -> Prerequisite {
    Prerequisite {
        check: "name_available".to_string(),
        passed: available,
        description: if available {
            "Workspace name is available".to_string()
        } else {
            "Workspace name already exists".to_string()
        },
    }
}

/// Compute the reason string for the add check.
fn add_reason(permitted: bool, in_git_repo: bool) -> String {
    if permitted {
        "Can create workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Workspace name already exists".to_string()
    }
}

/// Compute fix commands for the add check.
fn add_fix_commands(
    in_git_repo: bool,
    name_available: bool,
    resource: Option<&str>,
) -> Vec<String> {
    if !in_git_repo {
        vec!["git init".to_string()]
    } else if !name_available {
        resource.map_or_else(Vec::new, |name| {
            vec![format!("scp workspace remove {name}")]
        })
    } else {
        vec![]
    }
}

/// Check if the user can remove a workspace.
fn check_can_remove(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let ws_exists = resource.is_some_and(workspace_exists);

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        Prerequisite {
            check: "workspace_exists".to_string(),
            passed: ws_exists,
            description: if ws_exists {
                "Workspace exists".to_string()
            } else {
                "Workspace does not exist".to_string()
            },
        },
    ];

    let permitted = fs.in_git_repo && (resource.is_none() || ws_exists);
    let reason = remove_reason(permitted, fs.in_git_repo);
    build_output(BuildOutputArgs {
        action: "remove",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands: vec![],
    })
}

/// Compute reason for the remove check.
fn remove_reason(permitted: bool, in_git_repo: bool) -> String {
    if permitted {
        "Can remove workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Workspace does not exist".to_string()
    }
}

/// Check if the user can complete (done) a workspace.
fn check_can_done(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let resource_specified = resource.is_some();
    let has_target = fs.in_worktree || resource_specified;

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        Prerequisite {
            check: "in_workspace_or_specified".to_string(),
            passed: has_target,
            description: done_target_desc(fs.in_worktree, resource_specified),
        },
    ];

    let permitted = fs.in_git_repo && has_target;
    let reason = done_reason(permitted, fs.in_git_repo);
    build_output(BuildOutputArgs {
        action: "done",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands: vec![],
    })
}

/// Description for the workspace-or-specified prerequisite.
fn done_target_desc(in_worktree: bool, resource_specified: bool) -> String {
    if in_worktree {
        "Inside a worktree".to_string()
    } else if resource_specified {
        "Workspace specified".to_string()
    } else {
        "Not in a worktree and no workspace specified".to_string()
    }
}

/// Reason string for the done check.
fn done_reason(permitted: bool, in_git_repo: bool) -> String {
    if permitted {
        "Can complete and merge workspace".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Not in a worktree - specify workspace or cd to worktree".to_string()
    }
}

/// Check if the user can merge a workspace (same prerequisites as done).
fn check_can_merge(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let resource_specified = resource.is_some();
    let has_target = fs.in_worktree || resource_specified;

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        Prerequisite {
            check: "in_workspace_or_specified".to_string(),
            passed: has_target,
            description: done_target_desc(fs.in_worktree, resource_specified),
        },
    ];

    let permitted = fs.in_git_repo && has_target;
    let reason = done_reason(permitted, fs.in_git_repo);
    build_output(BuildOutputArgs {
        action: "merge",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands: vec![],
    })
}

/// Check if the user can undo the last operation.
fn check_can_undo() -> CanIOutput {
    let log_exists = undo_log_exists();
    let prerequisites = vec![Prerequisite {
        check: "undo_log_exists".to_string(),
        passed: log_exists,
        description: if log_exists {
            "Undo log exists".to_string()
        } else {
            "No undo log available".to_string()
        },
    }];

    let reason = if log_exists {
        "Can undo last operation".to_string()
    } else {
        "No undo log - nothing to undo".to_string()
    };
    build_output(BuildOutputArgs {
        action: "undo",
        resource: None,
        prerequisites,
        permitted: log_exists,
        reason,
        fix_commands: vec![],
    })
}

/// Check if the user can sync.
fn check_can_sync(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let worktrees_present = has_worktrees();
    let has_target = worktrees_present || resource.is_some();

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        Prerequisite {
            check: "has_worktrees".to_string(),
            passed: has_target,
            description: sync_target_desc(worktrees_present, resource.is_some()),
        },
    ];

    let permitted = fs.in_git_repo && has_target;
    let reason = sync_reason(permitted, fs.in_git_repo);
    build_output(BuildOutputArgs {
        action: "sync",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands: vec![],
    })
}

/// Description for the has-worktrees prerequisite.
fn sync_target_desc(has_worktrees: bool, resource_specified: bool) -> String {
    if has_worktrees {
        "Worktrees available to sync".to_string()
    } else if resource_specified {
        "Workspace specified".to_string()
    } else {
        "No worktrees to sync".to_string()
    }
}

/// Reason string for the sync check.
fn sync_reason(permitted: bool, in_git_repo: bool) -> String {
    if permitted {
        "Can sync workspaces".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "No worktrees to sync".to_string()
    }
}

/// Check if the user can spawn an agent session.
fn check_can_spawn(resource: Option<&str>) -> CanIOutput {
    let fs = FsState::probe();
    let bead_provided = resource.is_some();

    let prerequisites = vec![
        git_repo_prereq(fs.in_git_repo),
        Prerequisite {
            check: "bead_provided".to_string(),
            passed: bead_provided,
            description: if bead_provided {
                "Bead ID provided".to_string()
            } else {
                "No bead ID specified".to_string()
            },
        },
    ];

    let permitted = fs.in_git_repo && bead_provided;
    let reason = spawn_reason(permitted, fs.in_git_repo);
    build_output(BuildOutputArgs {
        action: "spawn",
        resource,
        prerequisites,
        permitted,
        reason,
        fix_commands: vec![],
    })
}

/// Reason string for the spawn check.
fn spawn_reason(permitted: bool, in_git_repo: bool) -> String {
    if permitted {
        "Can spawn agent session".to_string()
    } else if !in_git_repo {
        "Not inside a git repository".to_string()
    } else {
        "Bead ID required".to_string()
    }
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
