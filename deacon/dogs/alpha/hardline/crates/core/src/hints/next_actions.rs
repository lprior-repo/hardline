//! Next action suggestions based on command context
//!
//! Provides suggestions for what to do after various commands

use super::types::{ActionRisk, CommandContext, NextAction};

/// Generate next action suggestions based on command context.
///
/// Returns 0-5 suggestions with copy-pastable commands.
#[must_use]
pub fn next_actions_for_command(context: &CommandContext) -> Vec<NextAction> {
    if !context.success {
        return next_actions_for_error(context);
    }

    match context.command.as_str() {
        "init" => next_after_init(),
        "add" => next_after_add(context),
        "remove" => next_after_remove(context),
        "list" => next_after_list(context),
        "focus" => next_after_focus(context),
        "status" => next_after_status(context),
        "sync" => next_after_sync(context),
        "doctor" => next_after_doctor(),
        "clean" => next_after_clean(),
        _ => vec![],
    }
}

fn next_after_init() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "Create your first session".to_string(),
            commands: vec!["scp session add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Start a parallel workspace".to_string()),
        },
        NextAction {
            action: "Check system health".to_string(),
            commands: vec!["scp doctor".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
    ]
}

fn next_after_add(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![];
    if let Some(name) = &context.session_name {
        actions.push(NextAction {
            action: "Switch to new session".to_string(),
            commands: vec![format!("scp session focus {name}")],
            risk: ActionRisk::Safe,
            description: Some("Switch to the new session workspace".to_string()),
        });
        actions.push(NextAction {
            action: "Check session status".to_string(),
            commands: vec![format!("scp session status {name}")],
            risk: ActionRisk::Safe,
            description: None,
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["scp session list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_remove(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![NextAction {
        action: "List remaining sessions".to_string(),
        commands: vec!["scp session list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    }];
    if context.session_count > 1 {
        actions.push(NextAction {
            action: "Clean up stale sessions".to_string(),
            commands: vec!["scp session clean --dry-run".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Preview which sessions would be cleaned".to_string()),
        });
    }
    actions.push(NextAction {
        action: "Create a new session".to_string(),
        commands: vec!["scp session add <name>".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_list(context: &CommandContext) -> Vec<NextAction> {
    if context.session_count == 0 {
        return vec![NextAction {
            action: "Create your first session".to_string(),
            commands: vec!["scp session add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        }];
    }
    vec![
        NextAction {
            action: "Check session status".to_string(),
            commands: vec!["scp session status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Create another session".to_string(),
            commands: vec!["scp session add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
    ]
}

fn next_after_focus(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![];
    if let Some(name) = &context.session_name {
        actions.push(NextAction {
            action: "Check session status".to_string(),
            commands: vec![format!("scp session status {name}")],
            risk: ActionRisk::Safe,
            description: None,
        });
        actions.push(NextAction {
            action: "Sync session with main".to_string(),
            commands: vec![format!("scp session sync {name}")],
            risk: ActionRisk::Medium,
            description: Some("Rebase session onto latest main".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["scp session list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_status(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![];
    if let Some(name) = &context.session_name {
        actions.push(NextAction {
            action: "Sync session".to_string(),
            commands: vec![format!("scp session sync {name}")],
            risk: ActionRisk::Medium,
            description: Some("Rebase onto latest main".to_string()),
        });
        actions.push(NextAction {
            action: "Remove session".to_string(),
            commands: vec![format!("scp session remove {name}")],
            risk: ActionRisk::High,
            description: Some("Delete session and its workspace".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["scp session list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_sync(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![];
    if let Some(name) = &context.session_name {
        actions.push(NextAction {
            action: "Check session status".to_string(),
            commands: vec![format!("scp session status {name}")],
            risk: ActionRisk::Safe,
            description: Some("Verify sync result".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["scp session list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_doctor() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "List sessions".to_string(),
            commands: vec!["scp session list".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Clean stale sessions".to_string(),
            commands: vec!["scp session clean --dry-run".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Preview cleanup before applying".to_string()),
        },
    ]
}

fn next_after_clean() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "List remaining sessions".to_string(),
            commands: vec!["scp session list".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Run doctor check".to_string(),
            commands: vec!["scp doctor".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Verify system health after cleanup".to_string()),
        },
    ]
}

/// Generate next actions for failed commands
fn next_actions_for_error(context: &CommandContext) -> Vec<NextAction> {
    match context.command.as_str() {
        "init" => vec![NextAction {
            action: "Check system prerequisites".to_string(),
            commands: vec!["scp doctor".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Diagnose what's missing".to_string()),
        }],
        "add" => vec![
            NextAction {
                action: "List existing sessions".to_string(),
                commands: vec!["scp session list".to_string()],
                risk: ActionRisk::Safe,
                description: Some("Check if session name is already taken".to_string()),
            },
            NextAction {
                action: "Check system health".to_string(),
                commands: vec!["scp doctor".to_string()],
                risk: ActionRisk::Safe,
                description: None,
            },
        ],
        "focus" | "status" | "sync" | "remove" => vec![NextAction {
            action: "List available sessions".to_string(),
            commands: vec!["scp session list".to_string()],
            risk: ActionRisk::Safe,
            description: Some("See which sessions exist".to_string()),
        }],
        _ => vec![],
    }
}
