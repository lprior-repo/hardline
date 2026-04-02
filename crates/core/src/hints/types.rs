//! Hint types and data structures
//!
//! Core types for contextual hints and suggestions

use serde::{Deserialize, Serialize};

use crate::types::Session;

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

    /// Whether Git repo exists
    pub git_repo: bool,
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
