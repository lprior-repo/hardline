//! Next action suggestions based on command context.

use super::types::{ActionRisk, CommandContext, NextAction};

/// Generate next action suggestions based on command context.
#[must_use]
pub fn next_actions_for_command(context: &CommandContext) -> Vec<NextAction> {
    if !context.success {
        return next_actions_for_error(context);
    }

    match context.command.as_str() {
        "init" => next_after_init(),
        "create" => next_after_create(context),
        "destroy" => next_after_destroy(context),
        "list" => next_after_list(context),
        "status" => next_after_status(context),
        "checkpoint" => next_after_checkpoint(context),
        "restore" => next_after_restore(context),
        "merge" => next_after_merge(context),
        _ => Vec::new(),
    }
}

fn next_after_init() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "Create your first workspace".to_string(),
            commands: vec!["isolate create <name>".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Start working in an isolated workspace".to_string()),
        },
        NextAction {
            action: "Check system health".to_string(),
            commands: vec!["isolate doctor".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
    ]
}

fn next_after_create(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if let Some(name) = &context.workspace_name {
        actions.push(NextAction {
            action: "Check workspace status".to_string(),
            commands: vec![format!("isolate status {name}")],
            risk: ActionRisk::Safe,
            description: Some("Verify workspace was created correctly".to_string()),
        });
    }

    actions.push(NextAction {
        action: "List all workspaces".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

fn next_after_destroy(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![NextAction {
        action: "List remaining workspaces".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    }];

    if context.workspace_count > 1 {
        actions.push(NextAction {
            action: "Clean up stale workspaces".to_string(),
            commands: vec!["isolate clean --dry-run".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Preview which workspaces would be cleaned".to_string()),
        });
    }

    actions
}

fn next_after_list(context: &CommandContext) -> Vec<NextAction> {
    if context.workspace_count == 0 {
        return vec![NextAction {
            action: "Create your first workspace".to_string(),
            commands: vec!["isolate create <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        }];
    }

    vec![
        NextAction {
            action: "Check workspace status".to_string(),
            commands: vec!["isolate status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Create another workspace".to_string(),
            commands: vec!["isolate create <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
    ]
}

fn next_after_status(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if let Some(name) = &context.workspace_name {
        actions.push(NextAction {
            action: "Create checkpoint".to_string(),
            commands: vec![format!("isolate checkpoint {name}")],
            risk: ActionRisk::Safe,
            description: Some("Save current state before making changes".to_string()),
        });
    }

    actions.push(NextAction {
        action: "List all workspaces".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

fn next_after_checkpoint(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if let Some(name) = &context.workspace_name {
        actions.push(NextAction {
            action: "Restore from checkpoint".to_string(),
            commands: vec![format!("isolate restore {name}")],
            risk: ActionRisk::Medium,
            description: Some("Restore to a previous checkpoint if needed".to_string()),
        });
    }

    actions.push(NextAction {
        action: "List checkpoints".to_string(),
        commands: vec!["isolate checkpoint list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

fn next_after_restore(_context: &CommandContext) -> Vec<NextAction> {
    vec![NextAction {
        action: "Check restored status".to_string(),
        commands: vec!["isolate status".to_string()],
        risk: ActionRisk::Safe,
        description: Some("Verify restore completed correctly".to_string()),
    }]
}

fn next_after_merge(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if let Some(name) = &context.workspace_name {
        actions.push(NextAction {
            action: "Check merge status".to_string(),
            commands: vec![format!("isolate status {name}")],
            risk: ActionRisk::Safe,
            description: Some("Verify merge result".to_string()),
        });
    }

    actions.push(NextAction {
        action: "List workspaces".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

fn next_actions_for_error(context: &CommandContext) -> Vec<NextAction> {
    match context.command.as_str() {
        "init" => vec![NextAction {
            action: "Check system prerequisites".to_string(),
            commands: vec!["isolate doctor".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Diagnose what's missing".to_string()),
        }],
        "create" => vec![
            NextAction {
                action: "List existing workspaces".to_string(),
                commands: vec!["isolate list".to_string()],
                risk: ActionRisk::Safe,
                description: Some("Check if workspace name is already taken".to_string()),
            },
            NextAction {
                action: "Check system health".to_string(),
                commands: vec!["isolate doctor".to_string()],
                risk: ActionRisk::Safe,
                description: None,
            },
        ],
        "destroy" | "status" | "checkpoint" | "restore" | "merge" => vec![NextAction {
            action: "List available workspaces".to_string(),
            commands: vec!["isolate list".to_string()],
            risk: ActionRisk::Safe,
            description: Some("See which workspaces exist".to_string()),
        }],
        _ => Vec::new(),
    }
}
