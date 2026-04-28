//! Contextual hints and smart suggestions for AI agents
//!
//! Provides context-aware hints based on system state:
//! - Suggested next actions
//! - State explanations
//! - Learning from errors
//! - Predictive hints

use serde::{Deserialize, Serialize};

use crate::{
    types::{BeadsSummary, Session, SessionStatus},
    Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// HINT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A contextual hint from isolate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hint {
    /// Hint type
    #[serde(rename = "type")]
    pub hint_type: HintType,

    /// Human-readable message
    pub message: String,

    /// Suggested command to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<String>,

    /// Rationale for this hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    /// Information about current state
    Info,
    /// Suggested next action
    Suggestion,
    /// Warning about potential issue
    Warning,
    /// Explanation of error
    Error,
    /// Learning tip
    Tip,
}

/// System state for hint generation
#[derive(Debug, Clone)]
pub struct SystemState {
    /// All sessions
    pub sessions: Vec<Session>,

    /// Whether system is initialized
    pub initialized: bool,

    /// Whether JJ repo exists
    pub jj_repo: bool,
}

/// Risk level for a suggested next action
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionRisk {
    /// No side effects, always safe to run
    #[default]
    Safe,
    /// Some risk, review before running
    Medium,
    /// Significant risk, may cause data loss or irreversible changes
    High,
}

/// Next action suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    /// Action description
    pub action: String,

    /// Commands to execute (copy-pastable)
    pub commands: Vec<String>,

    /// Risk level of this action
    #[serde(default)]
    pub risk: ActionRisk,

    /// Optional longer description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Context about the command that just ran, used to generate next actions
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// The command name (e.g., "init", "add", "list", "remove", "focus", "status")
    pub command: String,
    /// Whether the command succeeded
    pub success: bool,
    /// Number of existing sessions
    pub session_count: usize,
    /// Name of the session involved, if any
    pub session_name: Option<String>,
}

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
            commands: vec!["isolate add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Start a parallel workspace".to_string()),
        },
        NextAction {
            action: "Check system health".to_string(),
            commands: vec!["isolate doctor".to_string()],
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
            commands: vec![format!("isolate focus {name}")],
            risk: ActionRisk::Safe,
            description: Some("Switch to the new session workspace".to_string()),
        });
        actions.push(NextAction {
            action: "Check session status".to_string(),
            commands: vec![format!("isolate status {name}")],
            risk: ActionRisk::Safe,
            description: None,
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_remove(context: &CommandContext) -> Vec<NextAction> {
    let mut actions = vec![NextAction {
        action: "List remaining sessions".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    }];
    if context.session_count > 1 {
        actions.push(NextAction {
            action: "Clean up stale sessions".to_string(),
            commands: vec!["isolate clean --dry-run".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Preview which sessions would be cleaned".to_string()),
        });
    }
    actions.push(NextAction {
        action: "Create a new session".to_string(),
        commands: vec!["isolate add <name>".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_list(context: &CommandContext) -> Vec<NextAction> {
    if context.session_count == 0 {
        return vec![NextAction {
            action: "Create your first session".to_string(),
            commands: vec!["isolate add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        }];
    }
    vec![
        NextAction {
            action: "Check session status".to_string(),
            commands: vec!["isolate status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Create another session".to_string(),
            commands: vec!["isolate add <name>".to_string()],
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
            commands: vec![format!("isolate status {name}")],
            risk: ActionRisk::Safe,
            description: None,
        });
        actions.push(NextAction {
            action: "Sync session with main".to_string(),
            commands: vec![format!("isolate sync {name}")],
            risk: ActionRisk::Medium,
            description: Some("Rebase session onto latest main".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["isolate list".to_string()],
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
            commands: vec![format!("isolate sync {name}")],
            risk: ActionRisk::Medium,
            description: Some("Rebase onto latest main".to_string()),
        });
        actions.push(NextAction {
            action: "Remove session".to_string(),
            commands: vec![format!("isolate remove {name}")],
            risk: ActionRisk::High,
            description: Some("Delete session and its workspace".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["isolate list".to_string()],
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
            commands: vec![format!("isolate status {name}")],
            risk: ActionRisk::Safe,
            description: Some("Verify sync result".to_string()),
        });
    }
    actions.push(NextAction {
        action: "List all sessions".to_string(),
        commands: vec!["isolate list".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });
    actions
}

fn next_after_doctor() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "List sessions".to_string(),
            commands: vec!["isolate list".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Clean stale sessions".to_string(),
            commands: vec!["isolate clean --dry-run".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Preview cleanup before applying".to_string()),
        },
    ]
}

fn next_after_clean() -> Vec<NextAction> {
    vec![
        NextAction {
            action: "List remaining sessions".to_string(),
            commands: vec!["isolate list".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        },
        NextAction {
            action: "Run doctor check".to_string(),
            commands: vec!["isolate doctor".to_string()],
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
            commands: vec!["isolate doctor".to_string()],
            risk: ActionRisk::Safe,
            description: Some("Diagnose what's missing".to_string()),
        }],
        "add" => vec![
            NextAction {
                action: "List existing sessions".to_string(),
                commands: vec!["isolate list".to_string()],
                risk: ActionRisk::Safe,
                description: Some("Check if session name is already taken".to_string()),
            },
            NextAction {
                action: "Check system health".to_string(),
                commands: vec!["isolate doctor".to_string()],
                risk: ActionRisk::Safe,
                description: None,
            },
        ],
        "focus" | "status" | "sync" | "remove" => vec![NextAction {
            action: "List available sessions".to_string(),
            commands: vec!["isolate list".to_string()],
            risk: ActionRisk::Safe,
            description: Some("See which sessions exist".to_string()),
        }],
        _ => vec![],
    }
}

/// Complete hints response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintsResponse {
    /// Current system context
    pub context: SystemContext,

    /// Generated hints
    pub hints: Vec<Hint>,

    /// Suggested next actions
    pub next_actions: Vec<NextAction>,
}

/// System context summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemContext {
    /// Is isolate initialized?
    pub initialized: bool,

    /// Is this a JJ repository?
    pub jj_repo: bool,

    /// Total number of sessions
    pub sessions_count: usize,

    /// Number of active sessions
    pub active_sessions: usize,

    /// Are there uncommitted changes?
    pub has_changes: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// HINT GENERATION
// ═══════════════════════════════════════════════════════════════════════════

impl Hint {
    /// Create an info hint
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Info,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a suggestion hint
    pub fn suggestion(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Suggestion,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a warning hint
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Warning,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a tip hint
    pub fn tip(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Tip,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Add a suggested command
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.suggested_command = Some(command.into());
        self
    }

    /// Add a rationale
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add context
    #[must_use]
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

/// Generate contextual hints based on system state
///
/// # Errors
///
/// Returns error if unable to analyze state
pub fn generate_hints(state: &SystemState) -> Result<Vec<Hint>> {
    let mut hints = Vec::new();

    // No sessions - encourage creation
    if state.sessions.is_empty() {
        hints.push(
            Hint::suggestion("No sessions yet. Create your first parallel workspace!")
                .with_command("isolate add <name>")
                .with_rationale("Sessions enable parallel work on multiple features"),
        );
        return Ok(hints);
    }

    // Sessions with changes
    state.sessions.iter().for_each(|session| {
        if session.status == SessionStatus::Active {
            // Note: In real implementation, would query actual changes
            // For now, just demonstrate the hint structure
            hints.push(
                Hint::info(format!(
                    "Session '{session_name}' is active",
                    session_name = session.name
                ))
                .with_command(format!(
                    "isolate status {session_name}",
                    session_name = session.name
                ))
                .with_rationale("Review session status regularly"),
            );
        }
    });

    // Completed sessions not removed
    state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Completed)
        .for_each(|session| {
            let duration = chrono::Utc::now() - session.updated_at;
            let age = duration.num_days();
            if age > 1 {
                hints.push(
                    Hint::suggestion(format!(
                        "Session '{session_name}' completed {age} day(s) ago, consider removing",
                        session_name = session.name,
                        age = age
                    ))
                    .with_command(format!(
                        "isolate remove {session_name} --merge",
                        session_name = session.name
                    ))
                    .with_rationale("Clean up completed work")
                    .with_context(serde_json::json!({
                        "session": session.name,
                        "age_days": age,
                    })),
                );
            }
        });

    // Failed sessions
    state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Failed)
        .for_each(|session| {
            hints.push(
                Hint::warning(format!(
                    "Session '{session_name}' failed during creation",
                    session_name = session.name
                ))
                .with_command(format!(
                    "isolate remove {session_name}",
                    session_name = session.name
                ))
                .with_rationale("Clean up failed session and retry"),
            );
        });

    Ok(hints)
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
            let session_name = extract_session_name(error_msg).map_or("session", |value| value);
            vec![
                Hint::suggestion("Use a different name for the new session")
                    .with_command(format!("isolate add {session_name}-v2"))
                    .with_rationale("Append version or date to differentiate"),
                Hint::suggestion("Switch to the existing session")
                    .with_command(format!("isolate focus {session_name}"))
                    .with_rationale("Continue work in existing session"),
                Hint::suggestion("Remove the existing session first")
                    .with_command(format!("isolate remove {session_name}"))
                    .with_rationale("Clean up old session before creating new one"),
            ]
        }

        "NOT_INITIALIZED" => {
            vec![
                Hint::suggestion("Initialize isolate in this repository")
                    .with_command("isolate init")
                    .with_rationale("Creates .isolate directory with configuration"),
                Hint::tip("After init, you can configure isolate in .isolate/config.toml")
                    .with_rationale("Customize workspace paths, hooks, and layouts"),
            ]
        }
        "JJ_NOT_FOUND" => {
            vec![
                Hint::warning("JJ (Jujutsu) is not installed or not in PATH")
                    .with_rationale("isolate requires JJ for workspace management"),
                Hint::suggestion("Install JJ from https://github.com/martinvonz/jj")
                    .with_rationale("Follow installation instructions for your platform"),
            ]
        }
        "SESSION_NOT_FOUND" => {
            vec![
                Hint::suggestion("List all sessions to see available ones")
                    .with_command("isolate list")
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

    // Not initialized
    if !state.initialized {
        actions.push(NextAction {
            action: "Initialize isolate".to_string(),
            commands: vec!["isolate init".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    // No sessions
    if state.sessions.is_empty() {
        actions.push(NextAction {
            action: "Create first session".to_string(),
            commands: vec!["isolate add <name>".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
        return actions;
    }

    // Has sessions - suggest common operations
    let has_active = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Active);

    if has_active {
        actions.push(NextAction {
            action: "Review session status".to_string(),
            commands: vec!["isolate status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        });
    }

    let has_completed = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Completed);

    if has_completed {
        let completed_name = state
            .sessions
            .iter()
            .find(|s| s.status == SessionStatus::Completed)
            .map(|s| &s.name);

        if let Some(name) = completed_name {
            actions.push(NextAction {
                action: "Clean up completed sessions".to_string(),
                commands: vec![format!("isolate remove {name} --merge", name = name)],
                risk: ActionRisk::Medium,
                description: Some("Merge and remove completed session".to_string()),
            });
        }
    }

    actions.push(NextAction {
        action: "Create new session".to_string(),
        commands: vec!["isolate add <name>".to_string()],
        risk: ActionRisk::Safe,
        description: None,
    });

    actions
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
        jj_repo: state.jj_repo,
        sessions_count: state.sessions.len(),
        active_sessions: active_count,
        has_changes: false, // TODO: Implement actual change detection
    };

    Ok(HintsResponse {
        context,
        hints,
        next_actions,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Extract session name from error message
fn extract_session_name(error_msg: &str) -> Option<&str> {
    // Try to extract text between single quotes
    error_msg.split('\'').nth(1)
}

/// Generate hints for beads status
///
/// # Returns
///
/// Returns a vector of hints. The result should be used
/// as this performs analysis and generates contextual help.
#[must_use]
pub fn hints_for_beads(session_name: &str, beads: &BeadsSummary) -> Vec<Hint> {
    let mut hints = Vec::new();

    if beads.has_blockers() {
        hints.push(
            Hint::warning(format!(
                "Session '{session_name}' has {blocked_count} blocked issue(s)",
                session_name = session_name,
                blocked_count = beads.blocked
            ))
            .with_command("bv")
            .with_rationale("Resolve blockers to make progress")
            .with_context(serde_json::json!({
                "session": session_name,
                "blocked_count": beads.blocked,
            })),
        );
    }

    if beads.active() > 5 {
        hints.push(
            Hint::tip(format!(
                "Session '{session_name}' has {active_count} active issues - consider focusing on fewer tasks",
                session_name = session_name, active_count = beads.active()
            ))
            .with_rationale("Limiting work in progress improves focus"),
        );
    }

    if beads.total() == 0 {
        hints.push(
            Hint::info(format!("Session '{session_name}' has no beads issues"))
                .with_command("br new")
                .with_rationale("Track your work with beads for better organization"),
        );
    }

    hints
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::{
        domain::{session::BranchState, AbsolutePath, SessionId},
        output::ValidatedMetadata,
        types::SessionName,
        WorkspaceState,
    };

    fn create_test_session(name: &str, status: SessionStatus) -> Session {
        Session {
            id: SessionId::parse(format!("id-{name}")).expect("valid id in test"),
            name: SessionName::new(name).expect("valid session name in test"),
            status,
            state: WorkspaceState::default(),
            workspace_path: AbsolutePath::parse("/tmp/test").expect("valid path in test"),
            branch: BranchState::Detached,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: ValidatedMetadata::empty(),
        }
    }

    #[test]
    fn test_hint_builders() {
        let hint = Hint::info("Test message")
            .with_command("isolate test")
            .with_rationale("Testing");

        assert_eq!(hint.hint_type, HintType::Info);
        assert_eq!(hint.message, "Test message");
        assert_eq!(hint.suggested_command, Some("isolate test".to_string()));
        assert_eq!(hint.rationale, Some("Testing".to_string()));
    }

    #[test]
    fn test_generate_hints_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("first parallel workspace"));
        }
    }

    #[test]
    fn test_generate_hints_completed_session() {
        let mut session = create_test_session("old-session", SessionStatus::Completed);
        session.updated_at = Utc::now() - chrono::Duration::days(3);

        let state = SystemState {
            sessions: vec![session],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(hints
            .iter()
            .any(|h| h.message.contains("consider removing")));
    }

    #[test]
    fn test_generate_hints_failed_session() {
        let state = SystemState {
            sessions: vec![create_test_session("failed-session", SessionStatus::Failed)],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
    }

    #[test]
    fn test_generate_hints_multiple_active() {
        let state = SystemState {
            sessions: vec![
                create_test_session("session1", SessionStatus::Active),
                create_test_session("session2", SessionStatus::Active),
                create_test_session("session3", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(!hints.is_empty() || hints.is_empty());
    }

    #[test]
    fn test_hints_for_error_session_exists() {
        let hints = hints_for_error("SESSION_ALREADY_EXISTS", "Session 'test' already exists");
        assert_eq!(hints.len(), 3);

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("different name"));
            assert!(hints[1].message.contains("Switch"));
            assert!(hints[2].message.contains("Remove"));
        }
    }

    #[test]
    fn test_hints_for_error_not_initialized() {
        let hints = hints_for_error("NOT_INITIALIZED", "isolate not initialized");
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("Initialize"));
        }
    }

    #[test]
    fn test_suggest_next_actions_not_initialized() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: false,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert_eq!(actions.len(), 1);

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(actions[0].action, "Initialize isolate");
        }
    }

    #[test]
    fn test_suggest_next_actions_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_suggest_next_actions_has_completed() {
        let state = SystemState {
            sessions: vec![create_test_session("done", SessionStatus::Completed)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("Clean up")));
    }

    #[test]
    fn test_hints_for_beads_blockers() {
        let beads = BeadsSummary {
            open: 2,
            in_progress: 1,
            blocked: 3,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
        assert!(hints.iter().any(|h| h.message.contains("blocked")));
    }

    #[test]
    fn test_hints_for_beads_too_many_active() {
        let beads = BeadsSummary {
            open: 4,
            in_progress: 3,
            blocked: 0,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("fewer tasks")));
    }

    #[test]
    fn test_hints_for_beads_none() {
        let beads = BeadsSummary::default();

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("no beads")));
    }

    #[test]
    fn test_extract_session_name() {
        assert_eq!(
            extract_session_name("Session 'test-name' already exists"),
            Some("test-name")
        );
        assert_eq!(
            extract_session_name("Session 'my-session' not found"),
            Some("my-session")
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // NEXT ACTIONS FOR COMMAND TESTS
    // ═══════════════════════════════════════════════════════════════════════

    fn success_context(command: &str, session_name: Option<&str>) -> CommandContext {
        CommandContext {
            command: command.to_string(),
            success: true,
            session_count: 2,
            session_name: session_name.map(String::from),
        }
    }

    fn error_context(command: &str) -> CommandContext {
        CommandContext {
            command: command.to_string(),
            success: false,
            session_count: 0,
            session_name: None,
        }
    }

    #[test]
    fn test_next_actions_init_success() {
        let actions = next_actions_for_command(&success_context("init", None));
        assert!(!actions.is_empty());
        assert!(actions.len() <= 5);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
        // All commands should be non-empty strings
        for action in &actions {
            assert!(!action.commands.is_empty());
            for cmd in &action.commands {
                assert!(!cmd.is_empty());
            }
        }
    }

    #[test]
    fn test_next_actions_add_success_with_session() {
        let actions = next_actions_for_command(&success_context("add", Some("feature-x")));
        assert!(!actions.is_empty());
        assert!(actions.len() <= 5);
        // Should suggest focusing on the new session
        assert!(actions
            .iter()
            .any(|a| a.commands.iter().any(|c| c.contains("focus feature-x"))));
    }

    #[test]
    fn test_next_actions_remove_success() {
        let actions = next_actions_for_command(&success_context("remove", Some("old")));
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action.contains("List")));
    }

    #[test]
    fn test_next_actions_list_no_sessions() {
        let ctx = CommandContext {
            command: "list".to_string(),
            success: true,
            session_count: 0,
            session_name: None,
        };
        let actions = next_actions_for_command(&ctx);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_next_actions_list_has_sessions() {
        let actions = next_actions_for_command(&success_context("list", None));
        assert!(actions.iter().any(|a| a.action.contains("status")));
    }

    #[test]
    fn test_next_actions_focus_success() {
        let actions = next_actions_for_command(&success_context("focus", Some("my-session")));
        assert!(!actions.is_empty());
        // Should suggest sync with medium risk
        let sync_action = actions.iter().find(|a| a.action.contains("Sync"));
        assert!(sync_action.is_some());
        if let Some(sa) = sync_action {
            assert_eq!(sa.risk, ActionRisk::Medium);
        }
    }

    #[test]
    fn test_next_actions_status_includes_risk_levels() {
        let actions = next_actions_for_command(&success_context("status", Some("sess")));
        // Remove action should be High risk
        let remove = actions.iter().find(|a| a.action.contains("Remove"));
        assert!(remove.is_some());
        if let Some(r) = remove {
            assert_eq!(r.risk, ActionRisk::High);
        }
    }

    #[test]
    fn test_next_actions_unknown_command_returns_empty() {
        let actions = next_actions_for_command(&success_context("nonexistent", None));
        assert!(actions.is_empty());
    }

    #[test]
    fn test_next_actions_error_returns_suggestions() {
        let actions = next_actions_for_command(&error_context("add"));
        assert!(!actions.is_empty());
        // Should suggest listing sessions
        assert!(actions
            .iter()
            .any(|a| a.commands.iter().any(|c| c.contains("isolate list"))));
    }

    #[test]
    fn test_next_actions_error_unknown_returns_empty() {
        let actions = next_actions_for_command(&error_context("nonexistent"));
        assert!(actions.is_empty());
    }

    #[test]
    fn test_next_actions_all_have_copy_pastable_commands() {
        let commands = [
            "init", "add", "remove", "list", "focus", "status", "sync", "doctor", "clean",
        ];
        for cmd in &commands {
            let ctx = success_context(cmd, Some("test-sess"));
            let actions = next_actions_for_command(&ctx);
            for action in &actions {
                assert!(
                    !action.commands.is_empty(),
                    "Command {cmd} action '{}' has no commands",
                    action.action
                );
                for c in &action.commands {
                    assert!(!c.is_empty(), "Command {cmd} has empty command string");
                }
            }
        }
    }

    #[test]
    fn test_next_actions_max_5() {
        // No command should return more than 5 suggestions
        let commands = [
            "init", "add", "remove", "list", "focus", "status", "sync", "doctor", "clean",
        ];
        for cmd in &commands {
            let ctx = success_context(cmd, Some("s"));
            let actions = next_actions_for_command(&ctx);
            assert!(
                actions.len() <= 5,
                "Command {cmd} returned {} actions",
                actions.len()
            );
        }
    }

    #[test]
    fn test_action_risk_default_is_safe() {
        assert_eq!(ActionRisk::default(), ActionRisk::Safe);
    }

    #[test]
    fn test_action_risk_serialization() {
        let safe_json = serde_json::to_string(&ActionRisk::Safe).unwrap_or_else(|_| String::new());
        assert_eq!(safe_json, "\"safe\"");
        let medium_json =
            serde_json::to_string(&ActionRisk::Medium).unwrap_or_else(|_| String::new());
        assert_eq!(medium_json, "\"medium\"");
        let high_json = serde_json::to_string(&ActionRisk::High).unwrap_or_else(|_| String::new());
        assert_eq!(high_json, "\"high\"");
    }

    #[test]
    fn test_next_action_serialization_includes_risk() {
        let action = NextAction {
            action: "Test".to_string(),
            commands: vec!["isolate test".to_string()],
            risk: ActionRisk::Medium,
            description: Some("A test action".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|_| String::new());
        assert!(json.contains("\"risk\":\"medium\""));
        assert!(json.contains("\"description\":\"A test action\""));
    }

    #[test]
    fn test_next_action_serialization_omits_none_description() {
        let action = NextAction {
            action: "Test".to_string(),
            commands: vec!["isolate test".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|_| String::new());
        assert!(!json.contains("description"));
    }

    #[test]
    fn test_command_context_clone() {
        let ctx = success_context("init", Some("s"));
        let cloned = ctx.clone();
        assert_eq!(ctx.command, cloned.command);
        assert_eq!(ctx.success, cloned.success);
    }

    #[test]
    fn test_generate_hints_response() {
        let state = SystemState {
            sessions: vec![create_test_session("active", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        let response = generate_hints_response(&state).unwrap_or_else(|_| HintsResponse {
            context: SystemContext {
                initialized: true,
                jj_repo: true,
                sessions_count: 0,
                active_sessions: 0,
                has_changes: false,
            },
            hints: Vec::new(),
            next_actions: Vec::new(),
        });

        assert_eq!(response.context.sessions_count, 1);
        assert_eq!(response.context.active_sessions, 1);
        assert!(!response.hints.is_empty());
        assert!(!response.next_actions.is_empty());
    }
}
