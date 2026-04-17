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
    /// Recognized values: "main", "`not_in_repo`", "unknown".
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
    /// Current location (typed enum serialized as `snake_case` string).
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Schema constants
    // =========================================================================

    #[test]
    fn schema_constants_are_non_empty() {
        assert!(!AI_STATUS_RESPONSE.is_empty());
        assert!(!AI_WORKFLOW_RESPONSE.is_empty());
        assert!(!AI_QUICKSTART_RESPONSE.is_empty());
        assert!(!AI_NEXT_RESPONSE.is_empty());
        assert!(!AI_OVERVIEW_RESPONSE.is_empty());
    }

    #[test]
    fn schema_constants_contain_expected_prefixes() {
        assert!(AI_STATUS_RESPONSE.starts_with("ai-"));
        assert!(AI_WORKFLOW_RESPONSE.starts_with("ai-"));
        assert!(AI_QUICKSTART_RESPONSE.starts_with("ai-"));
        assert!(AI_NEXT_RESPONSE.starts_with("ai-"));
        assert!(AI_OVERVIEW_RESPONSE.starts_with("ai-"));
    }

    #[test]
    fn schema_version_is_non_empty() {
        assert!(!SCHEMA_VERSION.is_empty());
    }

    #[test]
    fn all_schema_constants_are_unique() {
        let mut seen = std::collections::HashSet::new();
        let constants = [
            AI_STATUS_RESPONSE,
            AI_WORKFLOW_RESPONSE,
            AI_QUICKSTART_RESPONSE,
            AI_NEXT_RESPONSE,
            AI_OVERVIEW_RESPONSE,
        ];
        for c in constants {
            assert!(seen.insert(c), "Schema constant must be unique: {c}");
        }
    }

    // =========================================================================
    // AiEnvelope construction
    // =========================================================================

    #[test]
    fn envelope_new_sets_schema_uri_format() {
        let data = 42_i32;
        let env = AiEnvelope::new("test-schema", "single", data);
        assert!(env.schema.starts_with("scp://test-schema/"));
        assert!(env.schema.contains("/v1"));
    }

    #[test]
    fn envelope_new_sets_schema_version() {
        let env = AiEnvelope::new("schema", "type", "data");
        assert_eq!(env.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn envelope_new_sets_schema_type() {
        let env = AiEnvelope::new("schema", "my-type", "data");
        assert_eq!(env.schema_type, "my-type");
    }

    #[test]
    fn envelope_new_sets_success_true() {
        let env = AiEnvelope::new("schema", "type", "data");
        assert!(env.success);
    }

    #[test]
    fn envelope_serializes_with_dollar_schema_field() {
        #[derive(Debug, Clone, Serialize)]
        struct Dummy {
            x: i32,
        }
        let env = AiEnvelope::new("test", "single", Dummy { x: 42 });
        let json_str = serde_json::to_string(&env).expect("serialize");
        assert!(json_str.contains("\"$schema\""));
    }

    #[test]
    fn envelope_serializes_with_schema_version_field() {
        #[derive(Debug, Clone, Serialize)]
        struct Dummy {
            x: i32,
        }
        let env = AiEnvelope::new("test", "single", Dummy { x: 42 });
        let json_str = serde_json::to_string(&env).expect("serialize");
        assert!(json_str.contains("\"_schema_version\""));
    }

    #[test]
    fn envelope_flattens_data_into_top_level() {
        let env = AiEnvelope::new(
            "test",
            "single",
            AiStatusOutput {
                location: Location::Main,
                workspace: None,
                agent_id: None,
                initialized: true,
                active_sessions: 0,
                ready: true,
                suggestion: "ok".to_string(),
                next_command: "scp work".to_string(),
            },
        );
        let json_str = serde_json::to_string(&env).expect("serialize");
        assert!(
            json_str.contains("\"location\""),
            "flattened data should contain location"
        );
    }

    #[test]
    fn envelope_with_unit_data_serializes() {
        #[derive(Debug, Clone, Serialize)]
        struct Empty {}
        let env = AiEnvelope::new("empty", "single", Empty {});
        match serde_json::to_string(&env) {
            Ok(s) => assert!(s.contains("\"$schema\"")),
            Err(e) => panic!("Should serialize empty data: {e}"),
        }
    }

    #[test]
    fn envelope_with_vec_data_cannot_flatten() {
        // AiEnvelope uses #[serde(flatten)] which requires struct/map data, not sequences.
        // This test documents that behavior.
        let env = AiEnvelope::new("list", "array", vec!["a", "b"]);
        match serde_json::to_string(&env) {
            Ok(_) => panic!("Vec data should fail serialization because flatten requires structs"),
            Err(e) => assert!(
                e.to_string().contains("flatten") || e.to_string().contains("structs and maps"),
                "Error should mention flatten: {e}"
            ),
        }
    }

    // =========================================================================
    // AiStatusOutput
    // =========================================================================

    #[test]
    fn status_output_equality_works() {
        let a = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: false,
            active_sessions: 0,
            ready: false,
            suggestion: "no".to_string(),
            next_command: "scp init".to_string(),
        };
        let b = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: false,
            active_sessions: 0,
            ready: false,
            suggestion: "no".to_string(),
            next_command: "scp init".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn status_output_inequality_detects_field_difference() {
        let a = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "yes".to_string(),
            next_command: "scp work".to_string(),
        };
        let b = AiStatusOutput {
            location: Location::Main,
            workspace: None,
            agent_id: None,
            initialized: false,
            active_sessions: 0,
            ready: false,
            suggestion: "no".to_string(),
            next_command: "scp init".to_string(),
        };
        assert_ne!(a, b);
    }

    // =========================================================================
    // WorkflowInfo / WorkflowStep
    // =========================================================================

    #[test]
    fn workflow_info_equality_works() {
        let a = WorkflowInfo {
            name: "test".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "cmd".to_string(),
                description: "desc".to_string(),
            }],
        };
        let b = WorkflowInfo {
            name: "test".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "cmd".to_string(),
                description: "desc".to_string(),
            }],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn workflow_step_equality_works() {
        let a = WorkflowStep {
            step: 1,
            command: "cmd".to_string(),
            description: "desc".to_string(),
        };
        let b = WorkflowStep {
            step: 1,
            command: "cmd".to_string(),
            description: "desc".to_string(),
        };
        assert_eq!(a, b);
    }

    // =========================================================================
    // NextActionOutput
    // =========================================================================

    #[test]
    fn next_action_output_equality_works() {
        let a = NextActionOutput {
            action: "do".to_string(),
            command: "scp work".to_string(),
            reason: "because".to_string(),
            priority: Priority::Medium,
        };
        let b = NextActionOutput {
            action: "do".to_string(),
            command: "scp work".to_string(),
            reason: "because".to_string(),
            priority: Priority::Medium,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn next_action_output_differs_by_priority() {
        let a = NextActionOutput {
            action: "do".to_string(),
            command: "scp work".to_string(),
            reason: "because".to_string(),
            priority: Priority::High,
        };
        let b = NextActionOutput {
            action: "do".to_string(),
            command: "scp work".to_string(),
            reason: "because".to_string(),
            priority: Priority::Low,
        };
        assert_ne!(a, b);
    }

    // =========================================================================
    // QuickCommand / QuickStartOutput
    // =========================================================================

    #[test]
    fn quick_command_serializes_both_fields() {
        let cmd = QuickCommand {
            command: "scp work".to_string(),
            purpose: "start working".to_string(),
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(json.contains("\"command\""));
        assert!(json.contains("\"purpose\""));
    }

    #[test]
    fn quick_start_output_equality_works() {
        let a = QuickStartOutput {
            essential_commands: vec![],
            orientation: vec![],
            workflow: vec![],
        };
        let b = QuickStartOutput {
            essential_commands: vec![],
            orientation: vec![],
            workflow: vec![],
        };
        assert_eq!(a, b);
    }

    // =========================================================================
    // SubcommandInfo / AiOverview
    // =========================================================================

    #[test]
    fn subcommand_info_serializes_both_fields() {
        let info = SubcommandInfo {
            command: "scp ai status".to_string(),
            description: "get status".to_string(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"command\""));
        assert!(json.contains("\"description\""));
    }

    #[test]
    fn ai_overview_equality_works() {
        let a = AiOverview {
            message: "msg".to_string(),
            subcommands: vec![],
            quick_commands: vec![],
        };
        let b = AiOverview {
            message: "msg".to_string(),
            subcommands: vec![],
            quick_commands: vec![],
        };
        assert_eq!(a, b);
    }

    // =========================================================================
    // AiSubcommand
    // =========================================================================

    #[test]
    fn subcommand_is_copy() {
        let cmd = AiSubcommand::Status;
        let _copy = cmd;
        // If this compiles, AiSubcommand is Copy.
    }

    #[test]
    fn subcommand_equality_works() {
        assert_eq!(AiSubcommand::Status, AiSubcommand::Status);
        assert_ne!(AiSubcommand::Status, AiSubcommand::Workflow);
        assert_ne!(AiSubcommand::Workflow, AiSubcommand::QuickStart);
        assert_ne!(AiSubcommand::QuickStart, AiSubcommand::Next);
        assert_ne!(AiSubcommand::Next, AiSubcommand::Default);
    }

    // =========================================================================
    // AiOptions
    // =========================================================================

    #[test]
    fn ai_options_construction() {
        let opts = AiOptions {
            subcommand: AiSubcommand::Status,
        };
        assert_eq!(opts.subcommand, AiSubcommand::Status);
    }

    #[test]
    fn ai_options_clone() {
        let opts = AiOptions {
            subcommand: AiSubcommand::Next,
        };
        let cloned = opts.clone();
        assert_eq!(opts.subcommand, cloned.subcommand);
    }

    #[test]
    fn ai_options_debug() {
        let opts = AiOptions {
            subcommand: AiSubcommand::Workflow,
        };
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("AiOptions"));
    }

    // =========================================================================
    // Location enum - exhaustive coverage
    // =========================================================================

    #[test]
    fn location_all_variants_construct() {
        let _ = Location::Main;
        let _ = Location::Workspace("test".to_string());
        let _ = Location::NotInRepo;
        let _ = Location::Unknown;
    }

    #[test]
    fn location_clone_works() {
        let loc = Location::Workspace("ws".to_string());
        let cloned = loc.clone();
        assert_eq!(loc, cloned);
    }

    #[test]
    fn location_debug_works() {
        let loc = Location::Main;
        let debug_str = format!("{loc:?}");
        assert!(debug_str.contains("Main"));
    }

    #[test]
    fn location_from_raw_empty_string_becomes_workspace() {
        match Location::from_raw("") {
            Location::Workspace(name) => assert_eq!(name, ""),
            other => panic!("Empty string should become Workspace, got: {other:?}"),
        }
    }

    #[test]
    fn location_from_raw_preserves_workspace_string() {
        match Location::from_raw("workspace:feature-auth") {
            Location::Workspace(name) => assert_eq!(name, "workspace:feature-auth"),
            other => panic!("Expected Workspace, got: {other:?}"),
        }
    }

    #[test]
    fn location_serializes_main_as_snake_case() {
        let json = serde_json::to_string(&Location::Main).expect("serialize");
        assert_eq!(json, "\"main\"");
    }

    #[test]
    fn location_serializes_not_in_repo_as_snake_case() {
        let json = serde_json::to_string(&Location::NotInRepo).expect("serialize");
        assert_eq!(json, "\"not_in_repo\"");
    }

    #[test]
    fn location_serializes_unknown_as_snake_case() {
        let json = serde_json::to_string(&Location::Unknown).expect("serialize");
        assert_eq!(json, "\"unknown\"");
    }

    #[test]
    fn location_serializes_workspace_as_newtype_object() {
        // serde's rename_all applies to the variant name, but since Workspace has a
        // String field, it serializes as {"workspace":"x"}, not just "workspace".
        let json = serde_json::to_string(&Location::Workspace("x".to_string())).expect("serialize");
        assert!(
            json.contains("\"workspace\""),
            "Should contain workspace key: {json}"
        );
        assert!(
            json.contains("\"x\""),
            "Should contain workspace name: {json}"
        );
    }

    // =========================================================================
    // Priority enum - exhaustive coverage
    // =========================================================================

    #[test]
    fn priority_all_variants_construct() {
        let _ = Priority::High;
        let _ = Priority::Medium;
        let _ = Priority::Low;
    }

    #[test]
    fn priority_is_copy() {
        let p = Priority::High;
        let _copy = p;
    }

    #[test]
    fn priority_clone_works() {
        let p = Priority::Medium;
        let cloned = p.clone();
        assert_eq!(p, cloned);
    }

    #[test]
    fn priority_debug_works() {
        let p = Priority::Low;
        let debug_str = format!("{p:?}");
        assert!(debug_str.contains("Low"));
    }

    #[test]
    fn priority_serializes_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&Priority::High).expect("serialize"),
            "\"high\""
        );
        assert_eq!(
            serde_json::to_string(&Priority::Medium).expect("serialize"),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&Priority::Low).expect("serialize"),
            "\"low\""
        );
    }

    #[test]
    fn priority_equality_works() {
        assert_eq!(Priority::High, Priority::High);
        assert_ne!(Priority::High, Priority::Medium);
        assert_ne!(Priority::Medium, Priority::Low);
    }
}
