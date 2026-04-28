//! Hint generation functions.

use super::types::{ActionRisk, Hint, NextAction, SystemState};
use crate::domain::WorkspaceState;

/// Generate hints based on system state.
#[must_use]
pub fn generate_hints(state: &SystemState) -> Vec<Hint> {
    let mut hints = Vec::new();

    if state.workspaces.is_empty() {
        hints.push(
            Hint::suggestion("No workspaces yet. Create your first isolated workspace!")
                .with_command("isolate create <name>")
                .with_rationale("Isolated workspaces enable parallel development"),
        );
        return hints;
    }

    state
        .workspaces
        .iter()
        .filter(|w| w.state == WorkspaceState::Working)
        .for_each(|w| {
            hints.push(
                Hint::info(format!("Workspace '{}' is active", w.name))
                    .with_command(format!("isolate status {}", w.name))
                    .with_rationale("Monitor active workspace status regularly"),
            );
        });

    state
        .workspaces
        .iter()
        .filter(|w| w.state == WorkspaceState::Conflict)
        .for_each(|w| {
            hints.push(
                Hint::warning(format!("Workspace '{}' has conflicts", w.name))
                    .with_command(format!("isolate resolve {}", w.name))
                    .with_rationale("Resolve conflicts before merging"),
            );
        });

    state
        .workspaces
        .iter()
        .filter(|w| w.state == WorkspaceState::Merged)
        .for_each(|w| {
            hints.push(
                Hint::tip(format!(
                    "Workspace '{}' is merged. Consider cleaning it up.",
                    w.name
                ))
                .with_command(format!("isolate destroy {}", w.name))
                .with_rationale("Remove merged workspaces to keep system clean"),
            );
        });

    hints
}

/// Generate hints for a specific error.
#[must_use]
pub fn hints_for_error(error_code: &str, error_msg: &str) -> Vec<Hint> {
    match error_code {
        "WORKSPACE_EXISTS" => {
            let name = extract_workspace_name(error_msg).unwrap_or("workspace");
            vec![
                Hint::suggestion("Use a different name for the new workspace")
                    .with_command(format!("isolate create {name}-v2"))
                    .with_rationale("Append version or date to differentiate"),
                Hint::suggestion("Destroy the existing workspace first")
                    .with_command(format!("isolate destroy {name}"))
                    .with_rationale("Clean up old workspace before creating new one"),
            ]
        }

        "NOT_INITIALIZED" => vec![
            Hint::suggestion("Initialize isolate in this repository")
                .with_command("isolate init")
                .with_rationale("Creates isolate configuration"),
            Hint::tip("After init, you can configure workspace paths in .isolate/config.toml")
                .with_rationale("Customize workspace locations"),
        ],

        "WORKSPACE_NOT_FOUND" => vec![
            Hint::suggestion("List all workspaces to see available ones")
                .with_command("isolate list")
                .with_rationale("Check workspace names and states"),
            Hint::tip("Workspace names are case-sensitive")
                .with_rationale("Ensure exact match when referencing workspaces"),
        ],

        "CHECKPOINT_NOT_FOUND" => vec![Hint::suggestion("List available checkpoints")
            .with_command("isolate checkpoint list")
            .with_rationale("See which checkpoints exist")],

        "INVALID_STATE_TRANSITION" => vec![
            Hint::warning("Invalid state transition attempted")
                .with_rationale("Check workspace state before performing operation"),
            Hint::suggestion("Check workspace status")
                .with_command("isolate status")
                .with_rationale("See current workspace state"),
        ],

        _ => vec![],
    }
}

/// Generate suggested next actions based on state.
#[must_use]
pub fn suggest_next_actions(state: &SystemState) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if !state.initialized {
        actions.push(NextAction {
            action: "Initialize isolate".to_string(),
            commands: vec!["isolate init".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    if state.workspaces.is_empty() {
        actions.push(NextAction {
            action: "Create first workspace".to_string(),
            commands: vec!["isolate create <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    let has_active = state
        .workspaces
        .iter()
        .any(|w| w.state == WorkspaceState::Working);

    if has_active {
        actions.push(NextAction {
            action: "Review workspace status".to_string(),
            commands: vec!["isolate status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
    }

    let has_conflict = state
        .workspaces
        .iter()
        .any(|w| w.state == WorkspaceState::Conflict);

    if has_conflict {
        if let Some(name) = state
            .workspaces
            .iter()
            .find(|w| w.state == WorkspaceState::Conflict)
            .map(|w| w.name.as_str())
        {
            actions.push(NextAction {
                action: "Resolve workspace conflicts".to_string(),
                commands: vec![format!("isolate resolve {name}")],
                risk: ActionRisk::Medium,
                description: Some("Handle merge conflicts in workspace".to_string()),
            });
        }
    }

    actions.push(NextAction {
        action: "Create new workspace".to_string(),
        commands: vec!["isolate create <name>".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

/// Extract workspace name from error message.
fn extract_workspace_name(error_msg: &str) -> Option<&str> {
    error_msg.split('\'').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{HintType, WorkspaceInfo};

    #[test]
    fn test_extract_workspace_name() {
        let msg = "Workspace 'test-workspace' already exists";
        assert_eq!(extract_workspace_name(msg), Some("test-workspace"));
    }

    #[test]
    fn test_extract_workspace_name_no_quotes() {
        let msg = "Workspace already exists";
        assert_eq!(extract_workspace_name(msg), None);
    }

    #[test]
    fn test_hints_for_error_workspace_exists() {
        let hints = hints_for_error("WORKSPACE_EXISTS", "Workspace 'test' already exists");
        assert!(!hints.is_empty());
        assert_eq!(hints[0].hint_type, HintType::Suggestion);
    }

    #[test]
    fn test_hints_for_error_unknown() {
        let hints = hints_for_error("UNKNOWN_ERROR", "Some error");
        assert!(hints.is_empty());
    }

    #[test]
    fn test_generate_hints_empty() {
        let state = SystemState {
            workspaces: vec![],
            initialized: true,
        };
        let hints = generate_hints(&state);
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_generate_hints_with_workspaces() {
        let state = SystemState {
            workspaces: vec![WorkspaceInfo {
                id: "ws-1".to_string(),
                name: "test".to_string(),
                state: WorkspaceState::Working,
            }],
            initialized: true,
        };
        let hints = generate_hints(&state);
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_suggest_next_actions_uninitialized() {
        let state = SystemState {
            workspaces: vec![],
            initialized: false,
        };
        let actions = suggest_next_actions(&state);
        assert!(!actions.is_empty());
        assert!(actions[0].commands[0].contains("init"));
    }

    #[test]
    fn test_suggest_next_actions_with_workspaces() {
        let state = SystemState {
            workspaces: vec![WorkspaceInfo {
                id: "ws-1".to_string(),
                name: "test".to_string(),
                state: WorkspaceState::Working,
            }],
            initialized: true,
        };
        let actions = suggest_next_actions(&state);
        assert!(!actions.is_empty());
    }
}
