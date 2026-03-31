//! Data types for the AI command (Tier 1).
//!
//! Inert, serializable structs and enums with no business logic.
//! All types here are pure data carriers following the Data->Calc->Actions pattern.

use serde::Serialize;
use std::fmt;

// =============================================================================
// Schema constants (local, matching scp-core/json/schemas.rs pattern)
// =============================================================================

pub(super) const AI_STATUS_RESPONSE: &str = "ai-status-response";
pub(super) const AI_WORKFLOW_RESPONSE: &str = "ai-workflow-response";
pub(super) const AI_QUICKSTART_RESPONSE: &str = "ai-quickstart-response";
pub(super) const AI_NEXT_RESPONSE: &str = "ai-next-response";
pub(super) const AI_OVERVIEW_RESPONSE: &str = "ai-overview-response";
pub(super) const SCHEMA_VERSION: &str = "1.0";

// =============================================================================
// AiEnvelope - local schema wrapper
// =============================================================================

/// Schema envelope wrapper for JSON responses.
///
/// # Why a local type instead of reusing `scp_core::json::SchemaEnvelope`?
///
/// The core json module (`scp_core::json`) is not yet publicly exported.
/// Once it becomes part of the public API, this struct should be replaced
/// with a re-export or type alias. Until then, this local definition
/// mirrors the same `$schema` / `_schema_version` / `schema_type` pattern
/// used throughout `scp_core::json::schemas`.
///
/// See: `crates/core/src/json/schemas.rs` for the canonical definition.
#[derive(Debug, Clone, Serialize)]
pub struct AiEnvelope<T> {
    #[serde(rename = "$schema")]
    pub schema: String,
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    pub schema_type: String,
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> AiEnvelope<T> {
    pub(super) fn new(schema_name: &str, schema_type: &str, data: T) -> Self {
        Self {
            schema: format!("scp://{schema_name}/v1"),
            schema_version: SCHEMA_VERSION.to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
        }
    }
}

// =============================================================================
// Location enum (DEFECT-9NB-3: replaces primitive String)
// =============================================================================

/// Represents where the user is relative to a repository.
///
/// Replaces the previous `String`-typed `location` field to make
/// illegal location states unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    /// At the repository root
    Main,
    /// Inside a named workspace
    Workspace(String),
    /// Not inside any recognized repository
    NotInRepo,
    /// Location could not be determined
    Unknown,
}

impl Location {
    /// Parse a location from a raw string (backward-compatible deserialization).
    ///
    /// Recognized values: "main", "not_in_repo", "unknown".
    /// Any string starting with "workspace" becomes `Location::Workspace`.
    /// Everything else falls back to `Location::Unknown`.
    #[must_use]
    pub fn from_raw(s: &str) -> Self {
        match s {
            "main" => Self::Main,
            "not_in_repo" => Self::NotInRepo,
            "unknown" => Self::Unknown,
            other => Self::Workspace(other.to_string()),
        }
    }

    /// Convert to the canonical string representation for serialization.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Main => "main",
            Self::Workspace(_) => "workspace",
            Self::NotInRepo => "not_in_repo",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Main => write!(f, "main"),
            Self::Workspace(name) => write!(f, "workspace:{name}"),
            Self::NotInRepo => write!(f, "not_in_repo"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// =============================================================================
// Priority enum (DEFECT-9NB-3: replaces primitive String)
// =============================================================================

/// Priority level for next-action recommendations.
///
/// Replaces the previous `String`-typed `priority` field to make
/// invalid priority values unrepresentable.
/// Priority level for next-action recommendations.
///
/// Replaces the previous `String`-typed `priority` field to make
/// illegal priority states unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    /// Convert to the canonical string for backward-compatible assertions.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// =============================================================================
// Data structs
// =============================================================================

/// AI Status output.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AiStatusOutput {
    /// Current location (typed enum serialized as snake_case string).
    pub location: Location,
    /// Current workspace name if in one.
    pub workspace: Option<String>,
    /// Agent ID if registered.
    pub agent_id: Option<String>,
    /// Whether SCP is initialized.
    pub initialized: bool,
    /// Number of active sessions.
    pub active_sessions: usize,
    /// Ready for work.
    pub ready: bool,
    /// Suggested next action.
    pub suggestion: String,
    /// Command to run.
    pub next_command: String,
}

/// Workflow information.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkflowInfo {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

/// Workflow step.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkflowStep {
    pub step: usize,
    pub command: String,
    pub description: String,
}

/// Next action output.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NextActionOutput {
    /// What to do.
    pub action: String,
    /// Command to run (copy-paste ready).
    pub command: String,
    /// Why this is the next step.
    pub reason: String,
    /// Priority level.
    pub priority: Priority,
}

/// Quick-start command reference.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuickCommand {
    pub command: String,
    pub purpose: String,
}

/// Quick-start output structure.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuickStartOutput {
    pub essential_commands: Vec<QuickCommand>,
    pub orientation: Vec<QuickCommand>,
    pub workflow: Vec<QuickCommand>,
}

/// Subcommand info for overview.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SubcommandInfo {
    pub command: String,
    pub description: String,
}

/// AI overview output.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AiOverview {
    pub message: String,
    pub subcommands: Vec<SubcommandInfo>,
    pub quick_commands: Vec<String>,
}

/// AI subcommands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSubcommand {
    /// Show AI-optimized status.
    Status,
    /// Show the parallel agent workflow.
    Workflow,
    /// Show quick-start guide.
    QuickStart,
    /// Get single next action.
    Next,
    /// Default: show overview.
    Default,
}

/// Options for the ai command.
#[derive(Debug, Clone)]
pub struct AiOptions {
    pub subcommand: AiSubcommand,
}
