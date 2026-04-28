//! `WhatIf` command - Preview what a command would do
//!
//! Provides detailed preview of command effects without execution.
//!
//! Enhanced to handle command flags properly, especially --workspace for done/abort commands.

use isolate_core::OutputFormat;
use serde::{Deserialize, Serialize};

use crate::{session::validate_session_name, Result};

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
pub fn run(options: &WhatIfOptions) -> Result<WhatIfResult> {
    // Enhanced flag detection for workspace commands
    let args = options.args.clone();
    let has_workspace_flag = args.contains(&"--workspace".to_string());
    let has_force_flag = args.contains(&"--force".to_string());
    let has_keep_flag = args.contains(&"--keep-workspace".to_string());
    let _has_dry_run_flag = args.contains(&"--dry-run".to_string());

    // Special handling for workspace commands with --workspace flag
    if has_workspace_flag {
        // Find the workspace name (should be after --workspace flag)
        if let Some(pos) = args.iter().position(|arg| arg == "--workspace") {
            if pos + 1 < args.len() {
                let workspace_name = &args[pos + 1];
                // Validate workspace name if it's not a flag itself
                if !workspace_name.starts_with("--") {
                    validate_session_name(workspace_name)?;
                }
            }
        }
    }

    // Enhanced command routing with flag-aware previews
    let result = match options.command.as_str() {
        "add" => preview_add_with_flags(&args, has_force_flag),
        "work" => preview_work_with_flags(&args),
        "remove" => preview_remove_with_flags(&args, has_keep_flag),
        "done" => preview_done_with_flags(&args, has_workspace_flag, has_force_flag, has_keep_flag),
        "abort" => preview_abort_with_flags(&args, has_workspace_flag),
        "sync" => Ok(preview_sync_with_flags(&args)),
        "spawn" => preview_spawn_with_flags(&args),
        _ => Ok(WhatIfResult {
            command: options.command.clone(),
            args: args.clone(),
            steps: vec![WhatIfStep {
                order: 1,
                description: format!("Execute \'{}\' command", options.command),
                action: format!("isolate {} {}", options.command, args.join(" ")),
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
                "No specific preview available for \'{}\'",
                options.command
            )],
            prerequisites: vec![],
        }),
    }?;

    Ok(result)
}

// Enhanced preview functions with flag awareness

fn preview_add_with_flags(args: &[String], has_force_flag: bool) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).map_or("<name>", |s| s);

    if name != "<name>" {
        validate_session_name(name)?;
    }

    let mut result = WhatIfResult {
        command: "add".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate session name".to_string(),
                action: format!("Check \'{}\' is valid and doesn't exist", name),
                can_fail: true,
                on_failure: Some("Error if name invalid or already exists".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Create JJ workspace".to_string(),
                action: format!("jj workspace add {name}"),
                can_fail: true,
                on_failure: Some("Rollback: nothing created yet".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Save to database".to_string(),
                action: "INSERT session into .isolate/state.db".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".isolate/workspaces/{name}"),
                description: "JJ workspace directory".to_string(),
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
        undo_command: Some(format!("isolate remove {name}")),
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if name == "<name>" {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Session name is valid".to_string(),
            },
            PrerequisiteCheck {
                check: "jj_installed".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "JJ is installed".to_string(),
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

fn preview_work_with_flags(args: &[String]) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).map_or("<name>", |s| s);

    if name != "<name>" {
        validate_session_name(name)?;
    }

    let result = WhatIfResult {
        command: "work".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate session name".to_string(),
                action: format!("Check \'{}\' is valid", name),
                can_fail: true,
                on_failure: Some("Error if name invalid".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Register as agent".to_string(),
                action: "Set Isolate_AGENT_ID environment variable".to_string(),
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
        side_effects: vec!["Sets Isolate_AGENT_ID in environment".to_string()],
        reversible: true,
        undo_command: Some(format!("isolate abort --workspace {name}")),
        warnings: vec![],
        prerequisites: vec![PrerequisiteCheck {
            check: "valid_name".to_string(),
            status: if name == "<name>" {
                PrerequisiteStatus::Unknown
            } else {
                PrerequisiteStatus::Met
            },
            description: "Session name is valid".to_string(),
        }],
    };

    Ok(result)
}

fn preview_remove_with_flags(args: &[String], has_keep_flag: bool) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).map_or("<name>", |s| s);

    if name != "<name>" {
        validate_session_name(name)?;
    }

    let mut result = WhatIfResult {
        command: "remove".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Check session exists".to_string(),
                action: format!("Verify \'{}\' exists in database", name),
                can_fail: true,
                on_failure: Some("Error if session not found".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Remove JJ workspace".to_string(),
                action: format!("jj workspace forget {name}"),
                can_fail: true,
                on_failure: Some("Log warning, continue".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Delete workspace files".to_string(),
                action: format!("rm -rf .isolate/workspaces/{name}"),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 4,
                description: "Remove from database".to_string(),
                action: "DELETE session from .isolate/state.db".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![],
        modifies: vec![],
        deletes: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".isolate/workspaces/{name}"),
                description: "JJ workspace directory".to_string(),
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
            status: if name == "<name>" {
                PrerequisiteStatus::Unknown
            } else {
                PrerequisiteStatus::Met
            },
            description: "Session name is valid".to_string(),
        }],
    };

    if has_keep_flag {
        result.steps[2].description = "Keep workspace files".to_string();
        result.steps[2].action = format!("Preserve .isolate/workspaces/{name}");
        result.steps[2].can_fail = false;
        result.deletes[0].description = "Workspace directory (unless --keep-workspace)".to_string();
        result
            .warnings
            .push("--keep-workspace flag will preserve workspace files".to_string());
    }

    Ok(result)
}

#[allow(clippy::too_many_lines)]
fn preview_done_with_flags(
    args: &[String],
    has_workspace_flag: bool,
    has_force_flag: bool,
    has_keep_flag: bool,
) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).map_or("<current>", |s| s);

    if workspace != "<current>" {
        validate_session_name(workspace)?;
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
                action: "jj commit -m <auto-message>".to_string(),
                can_fail: true,
                on_failure: Some("Error if commit fails".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Switch to main".to_string(),
                action: "jj edit main".to_string(),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 4,
                description: "Merge workspace".to_string(),
                action: format!("jj merge {workspace}"),
                can_fail: true,
                on_failure: Some("Error if merge conflicts".to_string()),
            },
            WhatIfStep {
                order: 5,
                description: "Log undo history".to_string(),
                action: "Write to .isolate/undo.jsonl".to_string(),
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
            WhatIfStep {
                order: 7,
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
                resource: ".isolate/undo.jsonl".to_string(),
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
            resource: format!(".isolate/workspaces/{workspace}"),
            description: "Workspace directory (unless --keep-workspace)".to_string(),
        }],
        side_effects: vec![
            "Changes working directory to main".to_string(),
            "Updates bead status to closed".to_string(),
        ],
        reversible: true,
        undo_command: Some("isolate undo".to_string()),
        warnings: vec![
            "Make sure all changes are committed".to_string(),
            "Use --dry-run to preview merge".to_string(),
        ],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "in_workspace".to_string(),
                status: if workspace == "<current>" {
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
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if workspace == "<current>" {
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
            .push("--workspace flag specifies workspace to close".to_string());
    }

    if has_force_flag {
        result
            .warnings
            .push("--force flag will skip confirmations".to_string());
    }

    if has_keep_flag {
        result.steps[6].description = "Keep workspace files".to_string();
        result.steps[6].action = format!("Preserve .isolate/workspaces/{workspace}");
        result.deletes[0].description = "Workspace directory (unless --keep-workspace)".to_string();
        result
            .warnings
            .push("--keep-workspace flag will preserve workspace files".to_string());
    }

    Ok(result)
}

fn preview_abort_with_flags(args: &[String], has_workspace_flag: bool) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).map_or("<current>", |s| s);

    if workspace != "<current>" {
        validate_session_name(workspace)?;
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
                description: "Remove JJ workspace".to_string(),
                action: format!("jj workspace forget {workspace}"),
                can_fail: true,
                on_failure: Some("Log warning, continue".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Delete workspace files".to_string(),
                action: format!("rm -rf .isolate/workspaces/{workspace}"),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 4,
                description: "Remove from database".to_string(),
                action: "DELETE session from .isolate/state.db".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![],
        modifies: vec![],
        deletes: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".isolate/workspaces/{workspace}"),
                description: "JJ workspace directory".to_string(),
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
                status: if workspace == "<current>" {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Must be in a workspace".to_string(),
            },
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if workspace == "<current>" {
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

fn preview_sync_with_flags(args: &[String]) -> WhatIfResult {
    WhatIfResult {
        command: "sync".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Check prerequisites".to_string(),
                action: "Verify JJ installed".to_string(),
                can_fail: true,
                on_failure: Some("Error if prerequisites not met".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Sync workspace".to_string(),
                action: "Update workspace state from JJ".to_string(),
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
            check: "jj_installed".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "JJ is installed".to_string(),
        }],
    }
}

fn preview_spawn_with_flags(args: &[String]) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).map_or("<name>", |s| s);

    if name != "<name>" {
        validate_session_name(name)?;
    }

    Ok(WhatIfResult {
        command: "spawn".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate bead ID".to_string(),
                action: format!("Check \'{}\' is valid bead ID", name),
                can_fail: true,
                on_failure: Some("Error if bead ID invalid".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Find bead definition".to_string(),
                action: format!("Lookup bead {name} in database"),
                can_fail: true,
                on_failure: Some("Error if bead not found".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Create workspace".to_string(),
                action: format!("Create workspace for bead {name}"),
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
                resource: format!(".isolate/workspaces/{name}"),
                description: "Workspace directory for bead".to_string(),
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
        undo_command: Some(format!("isolate abort --workspace {name}")),
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: if name == "<name>" {
                    PrerequisiteStatus::Unknown
                } else {
                    PrerequisiteStatus::Met
                },
                description: "Bead ID is valid".to_string(),
            },
            PrerequisiteCheck {
                check: "bead_exists".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "Bead exists in database".to_string(),
            },
        ],
    })
}

/// Detect common flags in argument list
#[cfg(test)]
fn detect_flags(args: &[String]) -> (bool, bool, bool, bool) {
    let has_workspace = args.contains(&"--workspace".to_string());
    let has_force = args.contains(&"--force".to_string());
    let has_keep = args.contains(&"--keep-workspace".to_string());
    let has_dry_run = args.contains(&"--dry-run".to_string());
    (has_workspace, has_force, has_keep, has_dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatif_default_options() {
        let opts = WhatIfOptions::default();
        assert!(opts.command.is_empty());
        assert!(opts.args.is_empty());
        assert!(opts.format.is_json());
    }

    #[test]
    fn test_whatif_result_default() {
        let result = WhatIfResult {
            command: "test".to_string(),
            args: vec![],
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
        assert_eq!(result.command, "test");
        assert!(result.args.is_empty());
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
        assert_eq!(step.description, "Test step");
        assert_eq!(step.action, "Do something");
        assert!(step.can_fail);
        assert_eq!(step.on_failure, Some("Handle failure".to_string()));
    }

    #[test]
    fn test_whatif_resource_change_structure() {
        let change = ResourceChange {
            resource_type: "test".to_string(),
            resource: "resource".to_string(),
            description: "Test resource".to_string(),
        };
        assert_eq!(change.resource_type, "test");
        assert_eq!(change.resource, "resource");
        assert_eq!(change.description, "Test resource");
    }

    #[test]
    fn test_whatif_prerequisite_structure() {
        let prereq = PrerequisiteCheck {
            check: "test_check".to_string(),
            status: PrerequisiteStatus::Met,
            description: "Test description".to_string(),
        };
        assert_eq!(prereq.check, "test_check");
        assert_eq!(prereq.status, PrerequisiteStatus::Met);
        assert_eq!(prereq.description, "Test description");
    }

    #[test]
    fn test_detect_flags_basic() {
        let args = vec![
            "done".to_string(),
            "--workspace".to_string(),
            "feature-x".to_string(),
        ];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(has_workspace);
        assert!(!has_force);
        assert!(!has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_detect_flags_multiple() {
        let args = vec![
            "done".to_string(),
            "--workspace".to_string(),
            "feature-x".to_string(),
            "--force".to_string(),
            "--keep-workspace".to_string(),
        ];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(has_workspace);
        assert!(has_force);
        assert!(has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_detect_flags_no_flags() {
        let args = vec!["done".to_string(), "feature-x".to_string()];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(!has_workspace);
        assert!(!has_force);
        assert!(!has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_detect_flags_empty() {
        let args: Vec<String> = Vec::new();
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(!has_workspace);
        assert!(!has_force);
        assert!(!has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_detect_flags_all_flags() {
        let args = vec![
            "--workspace".to_string(),
            "feature-x".to_string(),
            "--force".to_string(),
            "--keep-workspace".to_string(),
            "--dry-run".to_string(),
            "done".to_string(),
        ];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(has_workspace);
        assert!(has_force);
        assert!(has_keep);
        assert!(has_dry_run);
    }

    #[test]
    fn test_detect_flags_special_chars() {
        let args = vec![
            "done".to_string(),
            "--workspace".to_string(),
            "feature-x".to_string(),
            "--force".to_string(),
            "--keep-workspace".to_string(),
            "--with-dash".to_string(),
            "--with_underscore".to_string(),
            "--flag123".to_string(),
        ];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(has_workspace);
        assert!(has_force);
        assert!(has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_detect_flags_unicode() {
        let args = vec![
            "done".to_string(),
            "--workspace".to_string(),
            "feature-äöü".to_string(),
            "--force".to_string(),
            "--keep-workspace".to_string(),
            "--unicode-flag".to_string(),
        ];
        let (has_workspace, has_force, has_keep, has_dry_run) = detect_flags(&args);
        assert!(has_workspace);
        assert!(has_force);
        assert!(has_keep);
        assert!(!has_dry_run);
    }

    #[test]
    fn test_preview_add_with_force_flag() {
        let args = vec![
            "add".to_string(),
            "--force".to_string(),
            "test-session".to_string(),
        ];
        let result = preview_add_with_flags(&args, true).unwrap();
        assert_eq!(result.command, "add");
        assert_eq!(result.args, args);
        assert!(!result.warnings.is_empty()); // --force adds a warning
        assert!(result.warnings.iter().any(|w| w.contains("--force")));
    }

    #[test]
    fn test_preview_done_with_workspace_flag() {
        let args = vec![
            "done".to_string(),
            "--workspace".to_string(),
            "feature-x".to_string(),
        ];
        let result = preview_done_with_flags(&args, true, false, false).unwrap();
        assert_eq!(result.command, "done");
        assert_eq!(result.args, args);
        assert!(!result.warnings.is_empty()); // Should have workspace warning
    }

    #[test]
    fn test_preview_done_with_keep_flag() {
        let args = vec![
            "done".to_string(),
            "--keep-workspace".to_string(),
            "feature-x".to_string(),
        ];
        let result = preview_done_with_flags(&args, false, false, true).unwrap();
        assert_eq!(result.command, "done");
        assert_eq!(result.args, args);
        assert!(!result.warnings.is_empty()); // Should have keep-workspace warning
    }

    #[test]
    fn test_preview_remove_with_keep_flag() {
        let args = vec![
            "remove".to_string(),
            "--keep-workspace".to_string(),
            "test-session".to_string(),
        ];
        let result = preview_remove_with_flags(&args, true).unwrap();
        assert_eq!(result.command, "remove");
        assert_eq!(result.args, args);
        assert!(!result.warnings.is_empty()); // Should have keep-workspace warning
    }

    #[test]
    fn test_preview_add_with_all_flags() {
        let args = vec![
            "add".to_string(),
            "--force".to_string(),
            "--workspace".to_string(),
            "test-session".to_string(),
        ];
        let result = preview_add_with_flags(&args, true).unwrap();
        assert_eq!(result.command, "add");
        assert_eq!(result.args, args);
        // Should handle extra flags gracefully
    }

    #[test]
    fn test_preview_unknown_command() {
        let opts = WhatIfOptions {
            command: "unknown".to_string(),
            args: vec!["arg1".to_string()],
            format: OutputFormat::Json,
        };
        let result = run(&opts).unwrap();
        assert_eq!(result.command, "unknown");
        assert_eq!(result.args, vec!["arg1".to_string()]);
        assert_eq!(result.steps.len(), 1);
        assert!(result.reversible);
        assert!(result.warnings.len() > 0);
    }
}
