//! Simulation/Calculation layer for WhatIf preview
//!
//! Pure functions that generate preview results without I/O.

use scp_core::Result;

use crate::commands::handlers::whatif::{
    analysis::{ensure_valid_name, get_name_arg},
    PrerequisiteCheck, PrerequisiteStatus, ResourceChange, WhatIfOptions, WhatIfResult, WhatIfStep,
};

/// Compute prerequisite status based on whether the name is a placeholder
fn prereq_status(is_placeholder: bool) -> PrerequisiteStatus {
    if is_placeholder {
        PrerequisiteStatus::Unknown
    } else {
        PrerequisiteStatus::Met
    }
}

/// Resource changes for deleting a workspace directory and its database record
fn workspace_deletion_changes(name: &str) -> Vec<ResourceChange> {
    vec![
        ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".scp/workspaces/{name}"),
            description: "Workspace directory".to_string(),
        },
        ResourceChange {
            resource_type: "database_record".to_string(),
            resource: format!("session:{name}"),
            description: "Session tracking record".to_string(),
        },
    ]
}

/// Generate a preview for the given command
///
/// **Calculations (Tier 2)**: Pure function, no I/O
pub fn preview(options: &WhatIfOptions) -> Result<WhatIfResult> {
    let args = &options.args;
    let has_workspace_flag = args.contains(&"--workspace".to_string());
    let has_force_flag = args.contains(&"--force".to_string());
    let has_keep_flag = args.contains(&"--keep-workspace".to_string());

    if has_workspace_flag {
        if let Some(pos) = args.iter().position(|arg| arg == "--workspace") {
            if pos + 1 < args.len() {
                let workspace_name = &args[pos + 1];
                if !workspace_name.starts_with("--") {
                    scp_core::validation::domain::validate_session_name(workspace_name)
                        .map_err(|e| scp_core::Error::validation_error(e.to_string()))?;
                }
            }
        }
    }

    match options.command.as_str() {
        "add" | "workspace add" => preview_add(args, has_force_flag),
        "work" => preview_work(args),
        "remove" => preview_remove(args, has_keep_flag),
        "done" => preview_done(args, has_workspace_flag, has_force_flag, has_keep_flag),
        "abort" => preview_abort(args, has_workspace_flag),
        "sync" => Ok(preview_sync(args)),
        "spawn" => preview_spawn(args),
        _ => Ok(preview_unknown(options)),
    }
}

fn preview_unknown(options: &WhatIfOptions) -> WhatIfResult {
    WhatIfResult {
        command: options.command.clone(),
        args: options.args.clone(),
        steps: vec![WhatIfStep {
            order: 1,
            description: format!("Execute '{}' command", options.command),
            action: format!("scp {} {}", options.command, options.args.join(" ")),
            can_fail: true,
            on_failure: Some("Error message will be shown".to_string()),
        }],
        creates: vec![],
        modifies: vec![],
        deletes: vec![],
        side_effects: vec![],
        reversible: true,
        undo_command: None,
        warnings: vec![format!(
            "No specific preview available for '{}'",
            options.command
        )],
        prerequisites: vec![],
    }
}

pub fn preview_add(args: &[String], has_force_flag: bool) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    let mut result = build_add_result(&name, is_placeholder, args);

    if has_force_flag {
        result
            .warnings
            .push("--force flag will skip all confirmations".to_string());
    }

    Ok(result)
}

fn add_steps(name: &str) -> Vec<WhatIfStep> {
    vec![
        WhatIfStep {
            order: 1,
            description: "Validate session name".to_string(),
            action: format!("Check '{name}' is valid and doesn't exist"),
            can_fail: true,
            on_failure: Some("Error if name invalid or already exists".to_string()),
        },
        WhatIfStep {
            order: 2,
            description: "Create workspace".to_string(),
            action: format!("git worktree add .scp/workspaces/{name}"),
            can_fail: true,
            on_failure: Some("Rollback: nothing created yet".to_string()),
        },
        WhatIfStep {
            order: 3,
            description: "Save to database".to_string(),
            action: "INSERT session into .scp/state.db".to_string(),
            can_fail: false,
            on_failure: None,
        },
    ]
}

fn add_creates(name: &str) -> Vec<ResourceChange> {
    vec![
        ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".scp/workspaces/{name}"),
            description: "Workspace directory".to_string(),
        },
        ResourceChange {
            resource_type: "database_record".to_string(),
            resource: format!("session:{name}"),
            description: "Session tracking record".to_string(),
        },
    ]
}

fn add_prerequisites(is_placeholder: bool) -> Vec<PrerequisiteCheck> {
    vec![
        PrerequisiteCheck {
            check: "valid_name".to_string(),
            status: prereq_status(is_placeholder),
            description: "Session name is valid".to_string(),
        },
        PrerequisiteCheck {
            check: "git_installed".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "Git is installed".to_string(),
        },
    ]
}

fn build_add_result(name: &str, is_placeholder: bool, args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "add".to_string(),
        args: args.to_vec(),
        steps: add_steps(name),
        creates: add_creates(name),
        modifies: vec![],
        deletes: vec![],
        side_effects: vec!["Changes working directory to workspace".to_string()],
        reversible: true,
        undo_command: Some(format!("scp workspace remove {name}")),
        warnings: vec![],
        prerequisites: add_prerequisites(is_placeholder),
    }
}

pub fn preview_work(args: &[String]) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    Ok(WhatIfResult {
        command: "work".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate session name".to_string(),
                action: format!("Check '{name}' is valid"),
                can_fail: true,
                on_failure: Some("Error if name invalid".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Register as agent".to_string(),
                action: "Set SCP_AGENT_ID environment variable".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![ResourceChange {
            resource_type: "agent_registration".to_string(),
            resource: format!("agent:{name}"),
            description: "Agent registration in database".to_string(),
        }],
        modifies: vec![],
        deletes: vec![],
        side_effects: vec!["Sets SCP_AGENT_ID in environment".to_string()],
        reversible: true,
        undo_command: Some(format!("scp workspace abort --workspace {name}")),
        warnings: vec![],
        prerequisites: vec![PrerequisiteCheck {
            check: "valid_name".to_string(),
            status: if is_placeholder {
                PrerequisiteStatus::Unknown
            } else {
                PrerequisiteStatus::Met
            },
            description: "Session name is valid".to_string(),
        }],
    })
}

pub fn preview_remove(args: &[String], has_keep_flag: bool) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    let mut result = build_remove_result(&name, is_placeholder, args);

    if has_keep_flag {
        result.steps[2].description = "Keep workspace files".to_string();
        result.steps[2].action = format!("Preserve .scp/workspaces/{name}");
        result.steps[2].can_fail = false;
        result.deletes[0].description = "Workspace directory (unless --keep-workspace)".to_string();
        result
            .warnings
            .push("--keep-workspace flag will preserve workspace files".to_string());
    }

    Ok(result)
}

fn remove_steps(name: &str) -> Vec<WhatIfStep> {
    vec![
        WhatIfStep {
            order: 1,
            description: "Check session exists".to_string(),
            action: format!("Verify '{name}' exists in database"),
            can_fail: true,
            on_failure: Some("Error if session not found".to_string()),
        },
        WhatIfStep {
            order: 2,
            description: "Remove workspace".to_string(),
            action: format!("git worktree remove .scp/workspaces/{name}"),
            can_fail: true,
            on_failure: Some("Log warning, continue".to_string()),
        },
        WhatIfStep {
            order: 3,
            description: "Delete workspace files".to_string(),
            action: format!("rm -rf .scp/workspaces/{name}"),
            can_fail: false,
            on_failure: None,
        },
        WhatIfStep {
            order: 4,
            description: "Remove from database".to_string(),
            action: "DELETE session from .scp/state.db".to_string(),
            can_fail: false,
            on_failure: None,
        },
    ]
}

fn remove_prerequisites(is_placeholder: bool) -> Vec<PrerequisiteCheck> {
    vec![PrerequisiteCheck {
        check: "valid_name".to_string(),
        status: prereq_status(is_placeholder),
        description: "Session name is valid".to_string(),
    }]
}

fn build_remove_result(name: &str, is_placeholder: bool, args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "remove".to_string(),
        args: args.to_vec(),
        steps: remove_steps(name),
        creates: vec![],
        modifies: vec![],
        deletes: workspace_deletion_changes(name),
        side_effects: vec![],
        reversible: false,
        undo_command: None,
        warnings: vec![],
        prerequisites: remove_prerequisites(is_placeholder),
    }
}

pub fn preview_done(
    args: &[String],
    has_workspace_flag: bool,
    has_force_flag: bool,
    has_keep_flag: bool,
) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");
    let is_placeholder = workspace == "<current>";

    if !is_placeholder {
        scp_core::validation::domain::validate_session_name(workspace)
            .map_err(|e| scp_core::Error::validation_error(e.to_string()))?;
    }

    let mut result = build_done_result(workspace, is_placeholder, args);

    apply_done_flags(
        &mut result,
        workspace,
        has_workspace_flag,
        has_force_flag,
        has_keep_flag,
    );

    Ok(result)
}

fn done_steps(workspace: &str) -> Vec<WhatIfStep> {
    vec![
        WhatIfStep {
            order: 1,
            description: "Validate location".to_string(),
            action: "Check we're in a workspace".to_string(),
            can_fail: true,
            on_failure: Some("Error: not in workspace".to_string()),
        },
        WhatIfStep {
            order: 2,
            description: "Commit any uncommitted changes".to_string(),
            action: "git commit -m <auto-message>".to_string(),
            can_fail: true,
            on_failure: Some("Error if commit fails".to_string()),
        },
        WhatIfStep {
            order: 3,
            description: "Switch to main".to_string(),
            action: "git checkout main".to_string(),
            can_fail: false,
            on_failure: None,
        },
        WhatIfStep {
            order: 4,
            description: "Merge workspace".to_string(),
            action: format!("git merge {workspace}"),
            can_fail: true,
            on_failure: Some("Error if merge conflicts".to_string()),
        },
        WhatIfStep {
            order: 5,
            description: "Log undo history".to_string(),
            action: "Write to .scp/undo.jsonl".to_string(),
            can_fail: false,
            on_failure: None,
        },
        WhatIfStep {
            order: 6,
            description: "Cleanup workspace".to_string(),
            action: format!("Remove workspace {workspace}"),
            can_fail: false,
            on_failure: None,
        },
    ]
}

fn done_creates() -> Vec<ResourceChange> {
    vec![
        ResourceChange {
            resource_type: "commit".to_string(),
            resource: "main".to_string(),
            description: "Merge commit on main".to_string(),
        },
        ResourceChange {
            resource_type: "undo_entry".to_string(),
            resource: ".scp/undo.jsonl".to_string(),
            description: "Undo history entry".to_string(),
        },
    ]
}

fn done_prerequisites(is_placeholder: bool) -> Vec<PrerequisiteCheck> {
    vec![
        PrerequisiteCheck {
            check: "in_workspace".to_string(),
            status: prereq_status(is_placeholder),
            description: "Must be in a workspace".to_string(),
        },
        PrerequisiteCheck {
            check: "no_conflicts".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "No merge conflicts with main".to_string(),
        },
    ]
}

fn build_done_result(workspace: &str, is_placeholder: bool, args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "done".to_string(),
        args: args.to_vec(),
        steps: done_steps(workspace),
        creates: done_creates(),
        modifies: vec![ResourceChange {
            resource_type: "branch".to_string(),
            resource: "main".to_string(),
            description: "Advances main with merge".to_string(),
        }],
        deletes: vec![ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".scp/workspaces/{workspace}"),
            description: "Workspace directory".to_string(),
        }],
        side_effects: vec![
            "Changes working directory to main".to_string(),
            "Updates task status to closed".to_string(),
        ],
        reversible: true,
        undo_command: Some("scp workspace undo".to_string()),
        warnings: vec![
            "Make sure all changes are committed".to_string(),
            "Use --dry-run to preview merge".to_string(),
        ],
        prerequisites: done_prerequisites(is_placeholder),
    }
}

fn apply_done_flags(
    result: &mut WhatIfResult,
    workspace: &str,
    has_workspace_flag: bool,
    has_force_flag: bool,
    has_keep_flag: bool,
) {
    if has_workspace_flag {
        result.steps[0].description = "Validate workspace location".to_string();
        result.steps[0].action = format!("Check --workspace {workspace} exists");
        result.prerequisites[0].description = "Workspace exists".to_string();
        result
            .warnings
            .push("--workspace flag specifies workspace to close".to_string());
    }

    if has_force_flag {
        result
            .warnings
            .push("--force flag will skip confirmations".to_string());
    }

    if has_keep_flag {
        result.steps[5].description = "Keep workspace files".to_string();
        result.steps[5].action = format!("Preserve .scp/workspaces/{workspace}");
        result.deletes[0].description = "Workspace directory (unless --keep-workspace)".to_string();
        result
            .warnings
            .push("--keep-workspace flag will preserve workspace files".to_string());
    }
}

pub fn preview_abort(args: &[String], has_workspace_flag: bool) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");
    let is_placeholder = workspace == "<current>";

    if !is_placeholder {
        scp_core::validation::domain::validate_session_name(workspace)
            .map_err(|e| scp_core::Error::validation_error(e.to_string()))?;
    }

    let mut result = build_abort_result(workspace, is_placeholder, args);

    if has_workspace_flag {
        result.steps[0].description = "Validate workspace location".to_string();
        result.steps[0].action = format!("Check --workspace {workspace} exists");
        result.prerequisites[0].description = "Workspace exists".to_string();
        result
            .warnings
            .push("--workspace flag specifies workspace to abort".to_string());
    }

    Ok(result)
}

fn abort_steps(workspace: &str) -> Vec<WhatIfStep> {
    vec![
        WhatIfStep {
            order: 1,
            description: "Validate location".to_string(),
            action: "Check we're in a workspace".to_string(),
            can_fail: true,
            on_failure: Some("Error: not in workspace".to_string()),
        },
        WhatIfStep {
            order: 2,
            description: "Remove workspace".to_string(),
            action: format!("git worktree remove .scp/workspaces/{workspace}"),
            can_fail: true,
            on_failure: Some("Log warning, continue".to_string()),
        },
        WhatIfStep {
            order: 3,
            description: "Delete workspace files".to_string(),
            action: format!("rm -rf .scp/workspaces/{workspace}"),
            can_fail: false,
            on_failure: None,
        },
        WhatIfStep {
            order: 4,
            description: "Remove from database".to_string(),
            action: "DELETE session from .scp/state.db".to_string(),
            can_fail: false,
            on_failure: None,
        },
    ]
}

fn abort_prerequisites(is_placeholder: bool) -> Vec<PrerequisiteCheck> {
    vec![
        PrerequisiteCheck {
            check: "in_workspace".to_string(),
            status: prereq_status(is_placeholder),
            description: "Must be in a workspace".to_string(),
        },
        PrerequisiteCheck {
            check: "valid_name".to_string(),
            status: prereq_status(is_placeholder),
            description: "Workspace name is valid".to_string(),
        },
    ]
}

fn build_abort_result(workspace: &str, is_placeholder: bool, args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "abort".to_string(),
        args: args.to_vec(),
        steps: abort_steps(workspace),
        creates: vec![],
        modifies: vec![],
        deletes: workspace_deletion_changes(workspace),
        side_effects: vec![],
        reversible: false,
        undo_command: None,
        warnings: vec![],
        prerequisites: abort_prerequisites(is_placeholder),
    }
}

pub fn preview_sync(args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "sync".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Check prerequisites".to_string(),
                action: "Verify Git installed".to_string(),
                can_fail: true,
                on_failure: Some("Error if prerequisites not met".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Sync workspace".to_string(),
                action: "Update workspace state from Git".to_string(),
                can_fail: true,
                on_failure: Some("Error if sync fails".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Update database".to_string(),
                action: "UPDATE session records".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![],
        modifies: vec![ResourceChange {
            resource_type: "database_record".to_string(),
            resource: "session:<all>".to_string(),
            description: "Update session states".to_string(),
        }],
        deletes: vec![],
        side_effects: vec![
            "Updates workspace states".to_string(),
            "May change working directory".to_string(),
        ],
        reversible: false,
        undo_command: None,
        warnings: vec![],
        prerequisites: vec![PrerequisiteCheck {
            check: "git_installed".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "Git is installed".to_string(),
        }],
    }
}

pub fn preview_spawn(args: &[String]) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;
    Ok(build_spawn_result(&name, is_placeholder, args))
}

fn spawn_steps(name: &str) -> Vec<WhatIfStep> {
    vec![
        WhatIfStep {
            order: 1,
            description: "Validate task ID".to_string(),
            action: format!("Check '{name}' is valid task ID"),
            can_fail: true,
            on_failure: Some("Error if task ID invalid".to_string()),
        },
        WhatIfStep {
            order: 2,
            description: "Find task definition".to_string(),
            action: format!("Lookup task {name} in database"),
            can_fail: true,
            on_failure: Some("Error if task not found".to_string()),
        },
        WhatIfStep {
            order: 3,
            description: "Create workspace".to_string(),
            action: format!("Create workspace for task {name}"),
            can_fail: true,
            on_failure: Some("Error if workspace creation fails".to_string()),
        },
        WhatIfStep {
            order: 4,
            description: "Initialize agent".to_string(),
            action: "Set up agent environment".to_string(),
            can_fail: false,
            on_failure: None,
        },
    ]
}

fn spawn_creates(name: &str) -> Vec<ResourceChange> {
    vec![
        ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".scp/workspaces/{name}"),
            description: "Workspace directory for task".to_string(),
        },
        ResourceChange {
            resource_type: "session".to_string(),
            resource: format!("session:{name}"),
            description: "Session tracking record".to_string(),
        },
    ]
}

fn spawn_prerequisites(is_placeholder: bool) -> Vec<PrerequisiteCheck> {
    vec![
        PrerequisiteCheck {
            check: "valid_name".to_string(),
            status: prereq_status(is_placeholder),
            description: "Task ID is valid".to_string(),
        },
        PrerequisiteCheck {
            check: "task_exists".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "Task exists in database".to_string(),
        },
    ]
}

fn build_spawn_result(name: &str, is_placeholder: bool, args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "spawn".to_string(),
        args: args.to_vec(),
        steps: spawn_steps(name),
        creates: spawn_creates(name),
        modifies: vec![],
        deletes: vec![],
        side_effects: vec![
            "Changes working directory to new workspace".to_string(),
            "Sets up agent environment".to_string(),
        ],
        reversible: true,
        undo_command: Some(format!("scp workspace abort --workspace {name}")),
        warnings: vec![],
        prerequisites: spawn_prerequisites(is_placeholder),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_unknown_command() {
        let opts = WhatIfOptions {
            command: "unknown".to_string(),
            args: vec!["arg1".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert_eq!(result.command, "unknown");
        assert_eq!(result.steps.len(), 1);
        assert!(result.reversible);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_preview_add_with_force_flag() {
        let opts = WhatIfOptions {
            command: "add".to_string(),
            args: vec!["--force".to_string(), "test-session".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--force")));
    }

    #[test]
    fn test_preview_done_with_workspace_flag() {
        let opts = WhatIfOptions {
            command: "done".to_string(),
            args: vec!["--workspace".to_string(), "feature-x".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--workspace")));
    }

    #[test]
    fn test_preview_remove_with_keep_flag() {
        let opts = WhatIfOptions {
            command: "remove".to_string(),
            args: vec!["--keep-workspace".to_string(), "test-session".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("--keep-workspace")));
    }

    #[test]
    fn test_preview_sync() {
        let opts = WhatIfOptions {
            command: "sync".to_string(),
            args: vec![],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert_eq!(result.command, "sync");
        assert!(!result.reversible);
    }

    #[test]
    fn test_preview_abort() {
        let opts = WhatIfOptions {
            command: "abort".to_string(),
            args: vec![],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert_eq!(result.command, "abort");
        assert!(!result.reversible);
        assert!(result.undo_command.is_none());
    }

    #[test]
    fn test_preview_work() {
        let opts = WhatIfOptions {
            command: "work".to_string(),
            args: vec!["test-session".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert_eq!(result.command, "work");
        assert!(result.reversible);
    }

    #[test]
    fn test_preview_spawn() {
        let opts = WhatIfOptions {
            command: "spawn".to_string(),
            args: vec!["test-session".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert_eq!(result.command, "spawn");
        assert!(result.reversible);
    }

    #[test]
    fn test_preview_done_with_keep_flag() {
        let opts = WhatIfOptions {
            command: "done".to_string(),
            args: vec!["--keep-workspace".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("--keep-workspace")));
    }

    #[test]
    fn test_preview_abort_with_workspace_flag() {
        let opts = WhatIfOptions {
            command: "abort".to_string(),
            args: vec!["--workspace".to_string(), "feature-x".to_string()],
            format: scp_core::OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--workspace")));
    }
}
