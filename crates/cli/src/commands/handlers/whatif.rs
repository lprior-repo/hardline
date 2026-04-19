//! WhatIf command - Preview what a command would do
//!
//! Provides detailed preview of command effects without execution.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: WhatIfOptions, WhatIfResult, WhatIfStep, ResourceChange (inert, serializable)
//! - **Calculations**: preview functions for each command (pure)
//! - **Actions**: run_whatif (I/O - output)

use scp_core::{
    output::Output, validation::domain::validate_session_name, Error, OutputFormat, Result,
};
use serde::{Deserialize, Serialize};

/// Options for the whatif command
#[derive(Debug, Clone)]
pub struct WhatIfOptions {
    /// Command to preview
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Output format
    pub format: OutputFormat,
}

impl Default for WhatIfOptions {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            format: OutputFormat::Json,
        }
    }
}

/// What-if preview result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfResult {
    /// The command being previewed
    pub command: String,
    /// Arguments provided
    pub args: Vec<String>,
    /// Steps that would be executed
    pub steps: Vec<WhatIfStep>,
    /// Resources that would be created
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub creates: Vec<ResourceChange>,
    /// Resources that would be modified
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifies: Vec<ResourceChange>,
    /// Resources that would be deleted
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub deletes: Vec<ResourceChange>,
    /// Side effects
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub side_effects: Vec<String>,
    /// Whether this operation is reversible
    pub reversible: bool,
    /// Undo command if reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Potential risks or warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Prerequisites that must be met
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<PrerequisiteCheck>,
}

/// A step in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfStep {
    /// Step number
    pub order: usize,
    /// Description of what this step does
    pub description: String,
    /// Command or action being performed
    pub action: String,
    /// Whether this step can fail
    pub can_fail: bool,
    /// What happens if this step fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
}

/// A resource that would be changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    /// Type of resource (session, workspace, file, database)
    pub resource_type: String,
    /// Resource identifier or path
    pub resource: String,
    /// Description of change
    pub description: String,
}

/// A prerequisite check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteCheck {
    /// What is being checked
    pub check: String,
    /// Current status
    pub status: PrerequisiteStatus,
    /// Description
    pub description: String,
}

/// Status of a prerequisite
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrerequisiteStatus {
    /// Prerequisite is met
    Met,
    /// Prerequisite is not met
    NotMet,
    /// Status is unknown (needs checking)
    Unknown,
}

/// Run the whatif command
///
/// **Actions (Tier 3)**: I/O - outputs preview
pub fn run_whatif(options: &WhatIfOptions) -> Result<()> {
    let result = preview(options)?;

    if options.format == OutputFormat::Json {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| Error::io_error(format!("Failed to serialize whatif result: {e}")))?;
        println!("{json}");
    } else {
        Output::info(&format!(
            "Preview for: {} {}",
            options.command,
            options.args.join(" ")
        ));
        Output::info(&format!(
            "Reversible: {}",
            if result.reversible { "yes" } else { "no" }
        ));

        if !result.steps.is_empty() {
            Output::info("Steps:");
            for step in &result.steps {
                Output::info(&format!("  {}. {}", step.order, step.description));
                Output::info(&format!("     Action: {}", step.action));
            }
        }

        if !result.creates.is_empty() {
            Output::info("Creates:");
            for c in &result.creates {
                Output::info(&format!("  {} ({})", c.resource, c.resource_type));
            }
        }

        if !result.modifies.is_empty() {
            Output::info("Modifies:");
            for m in &result.modifies {
                Output::info(&format!("  {} ({})", m.resource, m.resource_type));
            }
        }

        if !result.deletes.is_empty() {
            Output::info("Deletes:");
            for d in &result.deletes {
                Output::info(&format!("  {} ({})", d.resource, d.resource_type));
            }
        }

        if !result.warnings.is_empty() {
            Output::info("Warnings:");
            for w in &result.warnings {
                Output::warn(w);
            }
        }

        if let Some(undo) = &result.undo_command {
            Output::info(&format!("Undo: {undo}"));
        }
    }

    Ok(())
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
                    validate_session_name(workspace_name)
                        .map_err(|e| Error::validation_error(e.to_string()))?;
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
        _ => Ok(WhatIfResult {
            command: options.command.clone(),
            args: args.clone(),
            steps: vec![WhatIfStep {
                order: 1,
                description: format!("Execute '{}' command", options.command),
                action: format!("scp {} {}", options.command, args.join(" ")),
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
        }),
    }
}

fn get_name_arg(args: &[String]) -> (String, bool) {
    let name = args.first().map(String::as_str).unwrap_or("<name>");
    let is_placeholder = name == "<name>";
    (name.to_string(), is_placeholder)
}
fn ensure_valid_name(name: &str, is_placeholder: bool) -> Result<()> {
    if !is_placeholder {
        validate_session_name(name).map_err(|e| Error::validation_error(e.to_string()))?;
    }
    Ok(())
}

fn preview_add(args: &[String], has_force_flag: bool) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    let mut result = WhatIfResult {
        command: "add".to_string(),
        args: args.to_vec(),
        steps: vec![
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
        ],
        creates: vec![
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
        ],
        modifies: vec![],
        deletes: vec![],
        side_effects: vec!["Changes working directory to workspace".to_string()],
        reversible: true,
        undo_command: Some(format!("scp workspace remove {name}")),
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if is_placeholder {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Session name is valid".to_string(),
            },
            PrerequisiteCheck {
                check: "git_installed".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "Git is installed".to_string(),
            },
        ],
    };

    if has_force_flag {
        result
            .warnings
            .push("--force flag will skip all confirmations".to_string());
    }

    Ok(result)
}

fn preview_work(args: &[String]) -> Result<WhatIfResult> {
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

fn preview_remove(args: &[String], has_keep_flag: bool) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    let mut result = WhatIfResult {
        command: "remove".to_string(),
        args: args.to_vec(),
        steps: vec![
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
        ],
        creates: vec![],
        modifies: vec![],
        deletes: vec![
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
        ],
        side_effects: vec![],
        reversible: false,
        undo_command: None,
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
    };

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

#[allow(clippy::too_many_lines)]
fn preview_done(
    args: &[String],
    has_workspace_flag: bool,
    has_force_flag: bool,
    has_keep_flag: bool,
) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");
    let is_placeholder = workspace == "<current>";

    if !is_placeholder {
        validate_session_name(workspace).map_err(|e| Error::validation_error(e.to_string()))?;
    }

    let mut result = WhatIfResult {
        command: "done".to_string(),
        args: args.to_vec(),
        steps: vec![
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
        ],
        creates: vec![
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
        ],
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
        prerequisites: vec![
            PrerequisiteCheck {
                check: "in_workspace".to_string(),
                status: if is_placeholder {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Must be in a workspace".to_string(),
            },
            PrerequisiteCheck {
                check: "no_conflicts".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "No merge conflicts with main".to_string(),
            },
        ],
    };

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

    Ok(result)
}

fn preview_abort(args: &[String], has_workspace_flag: bool) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");
    let is_placeholder = workspace == "<current>";

    if !is_placeholder {
        validate_session_name(workspace).map_err(|e| Error::validation_error(e.to_string()))?;
    }

    let mut result = WhatIfResult {
        command: "abort".to_string(),
        args: args.to_vec(),
        steps: vec![
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
        ],
        creates: vec![],
        modifies: vec![],
        deletes: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".scp/workspaces/{workspace}"),
                description: "Workspace directory".to_string(),
            },
            ResourceChange {
                resource_type: "database_record".to_string(),
                resource: format!("session:{workspace}"),
                description: "Session tracking record".to_string(),
            },
        ],
        side_effects: vec![],
        reversible: false,
        undo_command: None,
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "in_workspace".to_string(),
                status: if is_placeholder {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Must be in a workspace".to_string(),
            },
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if is_placeholder {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Workspace name is valid".to_string(),
            },
        ],
    };

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

fn preview_sync(args: &[String]) -> WhatIfResult {
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

fn preview_spawn(args: &[String]) -> Result<WhatIfResult> {
    let (name, is_placeholder) = get_name_arg(args);
    ensure_valid_name(&name, is_placeholder)?;

    Ok(WhatIfResult {
        command: "spawn".to_string(),
        args: args.to_vec(),
        steps: vec![
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
        ],
        creates: vec![
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
        ],
        modifies: vec![],
        deletes: vec![],
        side_effects: vec![
            "Changes working directory to new workspace".to_string(),
            "Sets up agent environment".to_string(),
        ],
        reversible: true,
        undo_command: Some(format!("scp workspace abort --workspace {name}")),
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if is_placeholder {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Task ID is valid".to_string(),
            },
            PrerequisiteCheck {
                check: "task_exists".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "Task exists in database".to_string(),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatif_default_options() {
        let opts = WhatIfOptions::default();
        assert!(opts.command.is_empty());
        assert!(opts.args.is_empty());
    }

    #[test]
    fn test_whatif_result_structure() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec!["test-session".to_string()],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![],
        };
        assert_eq!(result.command, "add");
        assert_eq!(result.args.len(), 1);
        assert!(!result.reversible);
    }

    #[test]
    fn test_whatif_step_structure() {
        let step = WhatIfStep {
            order: 1,
            description: "Test step".to_string(),
            action: "Do something".to_string(),
            can_fail: true,
            on_failure: Some("Handle failure".to_string()),
        };
        assert_eq!(step.order, 1);
        assert!(step.can_fail);
    }

    #[test]
    fn test_whatif_resource_change_structure() {
        let change = ResourceChange {
            resource_type: "test".to_string(),
            resource: "resource".to_string(),
            description: "Test resource".to_string(),
        };
        assert_eq!(change.resource_type, "test");
    }

    #[test]
    fn test_whatif_prerequisite_status_serialization() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec![],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![
                PrerequisiteCheck {
                    check: "valid_name".to_string(),
                    status: PrerequisiteStatus::Met,
                    description: "Name is valid".to_string(),
                },
                PrerequisiteCheck {
                    check: "workspace_exists".to_string(),
                    status: PrerequisiteStatus::Unknown,
                    description: "Workspace exists".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let prereqs = parsed.get("prerequisites").unwrap().as_array().unwrap();
        assert_eq!(prereqs.len(), 2);
        assert_eq!(prereqs[0].get("status").unwrap().as_str(), Some("met"));
        assert_eq!(prereqs[1].get("status").unwrap().as_str(), Some("unknown"));
    }

    #[test]
    fn test_preview_unknown_command() {
        let opts = WhatIfOptions {
            command: "unknown".to_string(),
            args: vec!["arg1".to_string()],
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--force")));
    }

    #[test]
    fn test_preview_done_with_workspace_flag() {
        let opts = WhatIfOptions {
            command: "done".to_string(),
            args: vec!["--workspace".to_string(), "feature-x".to_string()],
            format: OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--workspace")));
    }

    #[test]
    fn test_preview_remove_with_keep_flag() {
        let opts = WhatIfOptions {
            command: "remove".to_string(),
            args: vec!["--keep-workspace".to_string(), "test-session".to_string()],
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
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
            format: OutputFormat::Json,
        };
        let result = preview(&opts).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("--workspace")));
    }
}
