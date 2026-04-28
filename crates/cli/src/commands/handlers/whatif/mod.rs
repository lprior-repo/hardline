//! WhatIf command - Preview what a command would do
//!
//! Provides detailed preview of command effects without execution.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: WhatIfOptions, WhatIfResult, WhatIfStep, ResourceChange (inert, serializable)
//! - **Calculations**: preview functions for each command (pure) - see [`simulation`]
//! - **Actions**: run_whatif (I/O - output) - see [`report`]

pub mod analysis;
pub mod report;
pub mod simulation;

use scp_core::{
    output::Output, validation::domain::validate_session_name, Error, OutputFormat, Result,
};
use serde::{Deserialize, Serialize};

/// Options for the whatif command
#[derive(Debug, Clone)]
pub struct WhatIfOptions {
    /// Command to preview
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Output format
    pub format: OutputFormat,
}

impl Default for WhatIfOptions {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            format: OutputFormat::Json,
        }
    }
}

/// What-if preview result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfResult {
    /// The command being previewed
    pub command: String,
    /// Arguments provided
    pub args: Vec<String>,
    /// Steps that would be executed
    pub steps: Vec<WhatIfStep>,
    /// Resources that would be created
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub creates: Vec<ResourceChange>,
    /// Resources that would be modified
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifies: Vec<ResourceChange>,
    /// Resources that would be deleted
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub deletes: Vec<ResourceChange>,
    /// Side effects
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub side_effects: Vec<String>,
    /// Whether this operation is reversible
    pub reversible: bool,
    /// Undo command if reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Potential risks or warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Prerequisites that must be met
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<PrerequisiteCheck>,
}

/// A step in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfStep {
    /// Step number
    pub order: usize,
    /// Description of what this step does
    pub description: String,
    /// Command or action being performed
    pub action: String,
    /// Whether this step can fail
    pub can_fail: bool,
    /// What happens if this step fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
}

/// A resource that would be changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    /// Type of resource (session, workspace, file, database)
    pub resource_type: String,
    /// Resource identifier or path
    pub resource: String,
    /// Description of change
    pub description: String,
}

/// A prerequisite check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteCheck {
    /// What is being checked
    pub check: String,
    /// Current status
    pub status: PrerequisiteStatus,
    /// Description
    pub description: String,
}

/// Status of a prerequisite
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrerequisiteStatus {
    /// Prerequisite is met
    Met,
    /// Prerequisite is not met
    NotMet,
    /// Status is unknown (needs checking)
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatif_default_options() {
        let opts = WhatIfOptions::default();
        assert!(opts.command.is_empty());
        assert!(opts.args.is_empty());
    }

    #[test]
    fn test_whatif_result_structure() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec!["test-session".to_string()],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![],
        };
        assert_eq!(result.command, "add");
        assert_eq!(result.args.len(), 1);
        assert!(!result.reversible);
    }

    #[test]
    fn test_whatif_step_structure() {
        let step = WhatIfStep {
            order: 1,
            description: "Test step".to_string(),
            action: "Do something".to_string(),
            can_fail: true,
            on_failure: Some("Handle failure".to_string()),
        };
        assert_eq!(step.order, 1);
        assert!(step.can_fail);
    }

    #[test]
    fn test_whatif_resource_change_structure() {
        let change = ResourceChange {
            resource_type: "test".to_string(),
            resource: "resource".to_string(),
            description: "Test resource".to_string(),
        };
        assert_eq!(change.resource_type, "test");
    }

    #[test]
    fn test_whatif_prerequisite_status_serialization() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec![],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![
                PrerequisiteCheck {
                    check: "valid_name".to_string(),
                    status: PrerequisiteStatus::Met,
                    description: "Name is valid".to_string(),
                },
                PrerequisiteCheck {
                    check: "workspace_exists".to_string(),
                    status: PrerequisiteStatus::Unknown,
                    description: "Workspace exists".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let prereqs = parsed.get("prerequisites").unwrap().as_array().unwrap();
        assert_eq!(prereqs.len(), 2);
        assert_eq!(prereqs[0].get("status").unwrap().as_str(), Some("met"));
        assert_eq!(prereqs[1].get("status").unwrap().as_str(), Some("unknown"));
    }

    #[test]
    fn what_if_result_roundtrip_empty() {
        let result = WhatIfResult {
            command: "test".to_string(),
            args: vec![],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: WhatIfResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.command, deserialized.command);
        assert_eq!(result.reversible, deserialized.reversible);
        assert!(deserialized.creates.is_empty());
        assert!(deserialized.modifies.is_empty());
        assert!(deserialized.deletes.is_empty());
        assert!(deserialized.warnings.is_empty());
    }

    #[test]
    fn what_if_result_with_all_fields() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec!["test-session".to_string()],
            steps: vec![WhatIfStep {
                order: 1,
                description: "Test step".to_string(),
                action: "Do thing".to_string(),
                can_fail: true,
                on_failure: Some("Error".to_string()),
            }],
            creates: vec![ResourceChange {
                resource_type: "workspace".to_string(),
                resource: ".scp/workspaces/test".to_string(),
                description: "Creates workspace".to_string(),
            }],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec!["Changes cwd".to_string()],
            reversible: true,
            undo_command: Some("scp workspace remove test".to_string()),
            warnings: vec!["Warning 1".to_string()],
            prerequisites: vec![PrerequisiteCheck {
                check: "valid_name".to_string(),
                status: PrerequisiteStatus::Met,
                description: "Name is valid".to_string(),
            }],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: WhatIfResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.command, deserialized.command);
        assert_eq!(result.steps.len(), deserialized.steps.len());
        assert_eq!(result.creates.len(), deserialized.creates.len());
        assert_eq!(result.reversible, deserialized.reversible);
        assert!(deserialized.undo_command.is_some());
        assert_eq!(result.prerequisites.len(), deserialized.prerequisites.len());
    }

    #[test]
    fn what_if_result_skip_serializing_if_empty() {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec![],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(!json.contains("\"creates\""));
        assert!(!json.contains("\"warnings\""));
        assert!(!json.contains("\"undo_command\""));
        assert!(!json.contains("\"side_effects\""));
    }

    #[test]
    fn what_if_step_roundtrip() {
        let step = WhatIfStep {
            order: 5,
            description: "Complex step".to_string(),
            action: "do complex thing".to_string(),
            can_fail: false,
            on_failure: None,
        };
        let json = serde_json::to_string(&step).expect("serialize");
        let deserialized: WhatIfStep = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(step.order, deserialized.order);
        assert_eq!(step.description, deserialized.description);
        assert_eq!(step.action, deserialized.action);
        assert_eq!(step.can_fail, deserialized.can_fail);
        assert!(deserialized.on_failure.is_none());
    }

    #[test]
    fn what_if_step_with_on_failure_roundtrip() {
        let step = WhatIfStep {
            order: 1,
            description: "Risky step".to_string(),
            action: "do risky thing".to_string(),
            can_fail: true,
            on_failure: Some("Rollback changes".to_string()),
        };
        let json = serde_json::to_string(&step).expect("serialize");
        let deserialized: WhatIfStep = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.on_failure.is_some());
        assert_eq!(deserialized.on_failure.unwrap(), "Rollback changes");
    }

    #[test]
    fn what_if_step_on_failure_omitted_when_none() {
        let step = WhatIfStep {
            order: 1,
            description: "Safe step".to_string(),
            action: "do safe thing".to_string(),
            can_fail: false,
            on_failure: None,
        };
        let json = serde_json::to_string(&step).expect("serialize");
        assert!(!json.contains("on_failure"));
    }

    #[test]
    fn resource_change_roundtrip() {
        let change = ResourceChange {
            resource_type: "session".to_string(),
            resource: "session:test".to_string(),
            description: "Agent session".to_string(),
        };
        let json = serde_json::to_string(&change).expect("serialize");
        let deserialized: ResourceChange = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(change.resource_type, deserialized.resource_type);
        assert_eq!(change.resource, deserialized.resource);
        assert_eq!(change.description, deserialized.description);
    }

    #[test]
    fn prerequisite_check_roundtrip() {
        let check = PrerequisiteCheck {
            check: "git_installed".to_string(),
            status: PrerequisiteStatus::Met,
            description: "Git is installed".to_string(),
        };
        let json = serde_json::to_string(&check).expect("serialize");
        let deserialized: PrerequisiteCheck = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(check.check, deserialized.check);
        assert_eq!(check.status, deserialized.status);
        assert_eq!(check.description, deserialized.description);
    }

    #[test]
    fn prerequisite_status_lowercase_rename() {
        let status = PrerequisiteStatus::Met;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"met\"");
        let status = PrerequisiteStatus::NotMet;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"notmet\"");
        let status = PrerequisiteStatus::Unknown;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"unknown\"");
    }

    #[test]
    fn prerequisite_status_roundtrip_all_variants() {
        for status in [
            PrerequisiteStatus::Met,
            PrerequisiteStatus::NotMet,
            PrerequisiteStatus::Unknown,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: PrerequisiteStatus =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn what_if_result_unknown_fields_ignored() {
        let json = r#"{
            "command": "test",
            "args": [],
            "steps": [],
            "creates": [],
            "modifies": [],
            "deletes": [],
            "side_effects": [],
            "reversible": false,
            "undo_command": null,
            "warnings": [],
            "prerequisites": [],
            "extra_field": "should be ignored",
            "another_extra": 123
        }"#;
        let result: std::result::Result<WhatIfResult, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "Should deserialize despite unknown fields");
    }

    #[test]
    fn what_if_result_missing_optional_fields_deserializes() {
        let json = r#"{
            "command": "add",
            "args": [],
            "steps": [],
            "creates": [],
            "modifies": [],
            "deletes": [],
            "side_effects": [],
            "reversible": true,
            "undo_command": null
        }"#;
        let result: std::result::Result<WhatIfResult, _> = serde_json::from_str(json);
        assert!(
            result.is_ok(),
            "Should deserialize despite missing warnings and prerequisites"
        );
    }
}
