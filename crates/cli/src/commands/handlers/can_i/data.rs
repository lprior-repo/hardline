//! Data types for the can-i command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the can-i command,
//! which checks whether a given action is permitted for AI agents.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the can-i command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct CanIOptions {
    /// Action to check (e.g. "spawn", "done", "remove").
    pub action: String,
    /// Optional resource identifier (e.g. workspace name, bead ID).
    pub resource: Option<String>,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a can-i permission check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanIOutput {
    /// Whether the action is allowed.
    pub permitted: bool,
    /// The action that was checked.
    pub action: String,
    /// The resource if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Human-readable reason for the result.
    pub reason: String,
    /// Individual prerequisite checks that contributed to the result.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<Prerequisite>,
    /// Suggested commands to remediate a denied action.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fix_commands: Vec<String>,
}

/// A single prerequisite check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    /// Machine-readable check name (e.g. "workspace_initialized").
    pub check: String,
    /// Whether this prerequisite passed.
    pub passed: bool,
    /// Human-readable description.
    pub description: String,
}

// ============================================================================
// Pure Helper Functions (Tier 1 - no I/O)
// ============================================================================

/// Known actions that have specific permission logic.
///
/// Actions not in this list are generally allowed by default.
pub const KNOWN_ACTIONS: &[&str] = &[
    "add", "work", "remove", "done", "undo", "sync", "spawn", "merge",
];

/// Determine whether an action string is a known action.
///
/// This is a pure function with no I/O.
#[must_use]
pub fn is_known_action(action: &str) -> bool {
    KNOWN_ACTIONS.contains(&action)
}

/// Build a default "generally allowed" output for unknown actions.
#[must_use]
pub fn default_allowed_output(action: &str, resource: Option<&str>) -> CanIOutput {
    CanIOutput {
        permitted: true,
        action: action.to_string(),
        resource: resource.map(String::from),
        reason: "Action is generally allowed".to_string(),
        prerequisites: vec![],
        fix_commands: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- CanIOutput construction & serialization --

    #[test]
    fn can_i_output_serialization_roundtrip() {
        let output = CanIOutput {
            permitted: true,
            action: "add".to_string(),
            resource: Some("test-session".to_string()),
            reason: "Can create session".to_string(),
            prerequisites: vec![],
            fix_commands: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: CanIOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.permitted);
        assert_eq!(deserialized.action, "add");
        assert_eq!(deserialized.resource.as_deref(), Some("test-session"));
        assert_eq!(deserialized.reason, "Can create session");
    }

    #[test]
    fn can_i_output_with_prerequisites() {
        let output = CanIOutput {
            permitted: false,
            action: "spawn".to_string(),
            resource: Some("bead-abc12".to_string()),
            reason: "Bead ID required".to_string(),
            prerequisites: vec![
                Prerequisite {
                    check: "workspace_initialized".to_string(),
                    passed: true,
                    description: "Workspace is initialized".to_string(),
                },
                Prerequisite {
                    check: "bead_provided".to_string(),
                    passed: false,
                    description: "No bead ID specified".to_string(),
                },
            ],
            fix_commands: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"permitted\":false"));
        assert!(json.contains("\"prerequisites\""));
    }

    #[test]
    fn can_i_output_skips_empty_fields() {
        let output = CanIOutput {
            permitted: true,
            action: "unknown".to_string(),
            resource: None,
            reason: "ok".to_string(),
            prerequisites: vec![],
            fix_commands: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(!json.contains("\"prerequisites\""));
        assert!(!json.contains("\"fix_commands\""));
        assert!(!json.contains("\"resource\""));
    }

    // -- Prerequisite serialization --

    #[test]
    fn prerequisite_serialization() {
        let prereq = Prerequisite {
            check: "test_check".to_string(),
            passed: true,
            description: "Test passed".to_string(),
        };
        let json = serde_json::to_string(&prereq).expect("serialize");
        assert!(json.contains("\"passed\":true"));
    }

    // -- is_known_action --

    #[test]
    fn known_actions_recognized() {
        assert!(is_known_action("add"));
        assert!(is_known_action("work"));
        assert!(is_known_action("remove"));
        assert!(is_known_action("done"));
        assert!(is_known_action("undo"));
        assert!(is_known_action("sync"));
        assert!(is_known_action("spawn"));
        assert!(is_known_action("merge"));
    }

    #[test]
    fn unknown_action_not_recognized() {
        assert!(!is_known_action("fly"));
        assert!(!is_known_action(""));
    }

    // -- default_allowed_output --

    #[test]
    fn default_allowed_output_no_resource() {
        let output = default_allowed_output("custom-action", None);
        assert!(output.permitted);
        assert_eq!(output.action, "custom-action");
        assert!(output.resource.is_none());
        assert!(output.prerequisites.is_empty());
        assert!(output.fix_commands.is_empty());
    }

    #[test]
    fn default_allowed_output_with_resource() {
        let output = default_allowed_output("custom-action", Some("my-resource"));
        assert!(output.permitted);
        assert_eq!(output.resource.as_deref(), Some("my-resource"));
    }

    // -- CanIOptions construction --

    #[test]
    fn can_i_options_construction() {
        let opts = CanIOptions {
            action: "spawn".to_string(),
            resource: Some("bead-123".to_string()),
        };
        assert_eq!(opts.action, "spawn");
        assert_eq!(opts.resource.as_deref(), Some("bead-123"));
    }

    #[test]
    fn can_i_options_no_resource() {
        let opts = CanIOptions {
            action: "undo".to_string(),
            resource: None,
        };
        assert_eq!(opts.action, "undo");
        assert!(opts.resource.is_none());
    }
}
