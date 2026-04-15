//! Hint types for isolate workspace context.

use serde::{Deserialize, Serialize};

use crate::domain::WorkspaceState;

// ============================================================================
// HINT TYPES
// ============================================================================

/// A contextual hint for isolate workspace operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hint {
    #[serde(rename = "type")]
    pub hint_type: HintType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Categories of hints for isolate operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    Info,
    Suggestion,
    Warning,
    Error,
    Tip,
}

// ============================================================================
// SYSTEM STATE
// ============================================================================

/// System state for isolate hint generation.
#[derive(Debug, Clone)]
pub struct SystemState {
    pub workspaces: Vec<WorkspaceInfo>,
    pub initialized: bool,
}

/// Information about a single workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub state: WorkspaceState,
}

// ============================================================================
// NEXT ACTION TYPES
// ============================================================================

/// Risk level for a suggested action.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionRisk {
    #[default]
    Safe,
    Medium,
    High,
}

/// A suggested next action with commands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    pub action: String,
    pub commands: Vec<String>,
    #[serde(default)]
    pub risk: ActionRisk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Context about a command that was executed.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub command: String,
    pub success: bool,
    pub workspace_count: usize,
    pub workspace_name: Option<String>,
}
