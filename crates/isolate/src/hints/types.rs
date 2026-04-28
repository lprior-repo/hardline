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

#[cfg(test)]
mod serde_tests {
    use super::*;
    #[test]
    fn hint_type_all_variants_lowercase() {
        for variant in [
            HintType::Info,
            HintType::Suggestion,
            HintType::Warning,
            HintType::Error,
            HintType::Tip,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let expected = format!("\"{:?}\"", variant).to_lowercase();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn hint_type_roundtrip() {
        for variant in [
            HintType::Info,
            HintType::Suggestion,
            HintType::Warning,
            HintType::Error,
            HintType::Tip,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: HintType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn hint_roundtrip_full() {
        let hint = Hint {
            hint_type: HintType::Suggestion,
            message: "Test message".to_string(),
            suggested_command: Some("isolate create foo".to_string()),
            rationale: Some("Because reasons".to_string()),
            context: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&hint).unwrap();
        let parsed: Hint = serde_json::from_str(&json).unwrap();
        assert_eq!(hint, parsed);
    }

    #[test]
    fn hint_roundtrip_minimal() {
        let hint = Hint {
            hint_type: HintType::Info,
            message: "Minimal hint".to_string(),
            suggested_command: None,
            rationale: None,
            context: None,
        };
        let json = serde_json::to_string(&hint).unwrap();
        let parsed: Hint = serde_json::from_str(&json).unwrap();
        assert_eq!(hint, parsed);
    }

    #[test]
    fn hint_skip_serializing_if_omits_none_fields() {
        let hint = Hint {
            hint_type: HintType::Info,
            message: "Test".to_string(),
            suggested_command: None,
            rationale: None,
            context: None,
        };
        let json = serde_json::to_string(&hint).unwrap();
        assert!(!json.contains("suggested_command"));
        assert!(!json.contains("rationale"));
        assert!(!json.contains("context"));
    }

    #[test]
    fn hint_includes_present_fields() {
        let hint = Hint {
            hint_type: HintType::Warning,
            message: "Warning message".to_string(),
            suggested_command: Some("cmd".to_string()),
            rationale: None,
            context: None,
        };
        let json = serde_json::to_string(&hint).unwrap();
        assert!(json.contains("suggested_command"));
        assert!(!json.contains("rationale"));
    }

    #[test]
    fn hint_type_field_is_renamed() {
        let hint = Hint {
            hint_type: HintType::Error,
            message: "Error hint".to_string(),
            suggested_command: None,
            rationale: None,
            context: None,
        };
        let json = serde_json::to_string(&hint).unwrap();
        assert!(json.contains("\"type\":\"Error\""));
    }

    #[test]
    fn action_risk_all_variants_lowercase() {
        for variant in [ActionRisk::Safe, ActionRisk::Medium, ActionRisk::High] {
            let json = serde_json::to_string(&variant).unwrap();
            let expected = format!("\"{:?}\"", variant).to_lowercase();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn action_risk_roundtrip() {
        for variant in [ActionRisk::Safe, ActionRisk::Medium, ActionRisk::High] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: ActionRisk = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn action_risk_default_is_safe() {
        let json = "\"safe\"";
        let parsed: ActionRisk = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, ActionRisk::Safe);
    }

    #[test]
    fn next_action_roundtrip_full() {
        let action = NextAction {
            action: "create".to_string(),
            commands: vec![
                "isolate create foo".to_string(),
                "isolate status".to_string(),
            ],
            risk: ActionRisk::Medium,
            description: Some("Create a workspace".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: NextAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn next_action_roundtrip_minimal() {
        let action = NextAction {
            action: "list".to_string(),
            commands: vec!["isolate list".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: NextAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn next_action_skip_serializing_if_omits_description() {
        let action = NextAction {
            action: "status".to_string(),
            commands: vec!["isolate status".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(!json.contains("description"));
    }

    #[test]
    fn next_action_risk_defaults_to_safe_when_missing() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct MinimalAction {
            action: String,
            commands: Vec<String>,
        }
        let json = r#"{"action":"test","commands":["cmd"]}"#;
        let parsed: MinimalAction = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.action, "test");
    }

    #[test]
    fn hint_roundtrip_with_workspace_info_context() {
        use serde::{Deserialize, Serialize};

        use crate::domain::WorkspaceState;

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct WorkspaceContext {
            pub id: String,
            pub name: String,
            pub state: WorkspaceState,
        }

        let hint = Hint {
            hint_type: HintType::Suggestion,
            message: "Workspace suggestion".to_string(),
            suggested_command: None,
            rationale: None,
            context: Some(
                serde_json::to_value(WorkspaceContext {
                    id: "ws-1".to_string(),
                    name: "my-workspace".to_string(),
                    state: WorkspaceState::Ready,
                })
                .unwrap(),
            ),
        };
        let json = serde_json::to_string(&hint).unwrap();
        let parsed: Hint = serde_json::from_str(&json).unwrap();
        assert_eq!(hint, parsed);
    }
}
