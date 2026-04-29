//! Hint generation functions
//!
//! Core functions for generating contextual hints

use chrono::Utc;

use super::{
    helpers::extract_session_name,
    response::{HintsResponse, SystemContext},
    types::{ActionRisk, Hint, NextAction, SystemState},
};
use crate::{error::Result, types::SessionStatus};

/// Generate contextual hints based on system state
///
/// # Errors
///
/// Returns error if unable to analyze state
pub fn generate_hints(state: &SystemState) -> Result<Vec<Hint>> {
    let mut hints = Vec::new();

    if state.sessions.is_empty() {
        hints.push(
            Hint::suggestion("No sessions yet. Create your first parallel workspace!")
                .with_command("scp session add <name>")
                .with_rationale("Sessions enable parallel work on multiple features"),
        );
        return Ok(hints);
    }

    hints.extend(hints_for_active_sessions(state));
    hints.extend(hints_for_stale_completed_sessions(state));
    hints.extend(hints_for_failed_sessions(state));

    Ok(hints)
}

/// Generate hints for active sessions.
fn hints_for_active_sessions(state: &SystemState) -> Vec<Hint> {
    state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .map(|session| {
            Hint::info(format!("Session '{}' is active", session.name.as_str()))
                .with_command(format!("scp session status {}", session.name.as_str()))
                .with_rationale("Review session status regularly")
        })
        .collect()
}

/// Generate hints for completed sessions older than one day.
fn hints_for_stale_completed_sessions(state: &SystemState) -> Vec<Hint> {
    state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Completed)
        .filter_map(|session| {
            let duration = Utc::now() - session.updated_at;
            let age = duration.num_days();
            if age > 1 {
                Some(
                    Hint::suggestion(format!(
                        "Session '{}' completed {} day(s) ago, consider removing",
                        session.name.as_str(),
                        age
                    ))
                    .with_command(format!(
                        "scp session remove {} --merge",
                        session.name.as_str()
                    ))
                    .with_rationale("Clean up completed work")
                    .with_context(serde_json::json!({
                        "session": session.name.as_str(),
                        "age_days": age,
                    })),
                )
            } else {
                None
            }
        })
        .collect()
}

/// Generate hints for failed sessions.
fn hints_for_failed_sessions(state: &SystemState) -> Vec<Hint> {
    state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Failed)
        .map(|session| {
            Hint::warning(format!(
                "Session '{}' failed during creation",
                session.name.as_str()
            ))
            .with_command(format!("scp session remove {}", session.name.as_str()))
            .with_rationale("Clean up failed session and retry")
        })
        .collect()
}

/// Generate hints for a specific error
///
/// # Returns
///
/// Returns a vector of hints for the given error. The result should be used
/// as this performs error analysis and generates contextual help.
#[must_use]
pub fn hints_for_error(error_code: &str, error_msg: &str) -> Vec<Hint> {
    match error_code {
        "SESSION_ALREADY_EXISTS" => {
            let session_name = extract_session_name(error_msg).unwrap_or("session");
            vec![
                Hint::suggestion("Use a different name for the new session")
                    .with_command(format!("scp session add {session_name}-v2"))
                    .with_rationale("Append version or date to differentiate"),
                Hint::suggestion("Switch to the existing session")
                    .with_command(format!("scp session focus {session_name}"))
                    .with_rationale("Continue work in existing session"),
                Hint::suggestion("Remove the existing session first")
                    .with_command(format!("scp session remove {session_name}"))
                    .with_rationale("Clean up old session before creating new one"),
            ]
        }

        "NOT_INITIALIZED" => {
            vec![
                Hint::suggestion("Initialize scp in this repository")
                    .with_command("scp init")
                    .with_rationale("Creates .scp directory with configuration"),
                Hint::tip("After init, you can configure scp in .scp/config.toml")
                    .with_rationale("Customize workspace paths, hooks, and layouts"),
            ]
        }
        "GIT_NOT_FOUND" => {
            vec![
                Hint::warning("Git is not installed or not in PATH")
                    .with_rationale("scp requires Git for workspace management"),
                Hint::suggestion("Install Git from https://git-scm.com")
                    .with_rationale("Follow installation instructions for your platform"),
            ]
        }
        "SESSION_NOT_FOUND" => {
            vec![
                Hint::suggestion("List all sessions to see available ones")
                    .with_command("scp session list")
                    .with_rationale("Check session names and status"),
                Hint::tip("Session names are case-sensitive")
                    .with_rationale("Ensure exact match when referencing sessions"),
            ]
        }
        _ => vec![],
    }
}

/// Generate suggested next actions based on state
///
/// # Returns
///
/// Returns a vector of suggested actions. The result should be used
/// as this performs state analysis and generates recommendations.
#[must_use]
pub fn suggest_next_actions(state: &SystemState) -> Vec<NextAction> {
    let mut actions = Vec::new();

    if !state.initialized {
        actions.push(NextAction {
            action: "Initialize scp".to_string(),
            commands: vec!["scp init".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    if state.sessions.is_empty() {
        actions.push(NextAction {
            action: "Create first session".to_string(),
            commands: vec!["scp session add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    let has_active = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Active);

    if has_active {
        actions.push(NextAction {
            action: "Review session status".to_string(),
            commands: vec!["scp session status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
    }

    if let Some(action) = completed_session_action(state) {
        actions.push(action);
    }

    actions.push(NextAction {
        action: "Create new session".to_string(),
        commands: vec!["scp session add <name>".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
}

/// Generate a cleanup action if any session is completed.
fn completed_session_action(state: &SystemState) -> Option<NextAction> {
    let has_completed = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Completed);

    if !has_completed {
        return None;
    }

    let completed_name = state
        .sessions
        .iter()
        .find(|s| s.status == SessionStatus::Completed)
        .map(|s| s.name.as_str())?;

    Some(NextAction {
        action: "Clean up completed sessions".to_string(),
        commands: vec![format!("scp session remove {completed_name} --merge",)],
        risk: ActionRisk::Medium,
        description: Some("Merge and remove completed session".to_string()),
    })
}

/// Generate complete hints response
///
/// # Errors
///
/// Returns error if unable to generate hints
pub fn generate_hints_response(state: &SystemState) -> Result<HintsResponse> {
    let hints = generate_hints(state)?;
    let next_actions = suggest_next_actions(state);

    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    let context = SystemContext {
        initialized: state.initialized,
        git_repo: state.git_repo,
        sessions_count: state.sessions.len(),
        active_sessions: active_count,
        has_changes: false,
    };

    Ok(HintsResponse {
        context,
        hints,
        next_actions,
    })
}
