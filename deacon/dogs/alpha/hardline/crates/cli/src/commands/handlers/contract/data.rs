//! Data types for the contract command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the contract command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct ContractOptions {
    /// Specific command to show contract for (or all if None).
    pub command: Option<String>,
}

/// Contract information for a single command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContract {
    /// Command name.
    pub name: String,
    /// Short description.
    pub description: String,
    /// Required arguments.
    pub required_args: Vec<ArgContract>,
    /// Optional arguments.
    pub optional_args: Vec<ArgContract>,
    /// Flags (boolean options).
    pub flags: Vec<FlagContract>,
    /// Output schema type.
    pub output_schema: String,
    /// Side effects of this command.
    pub side_effects: Vec<String>,
    /// Related commands.
    pub related_commands: Vec<String>,
    /// Example usage.
    pub examples: Vec<String>,
    /// Whether this command is reversible.
    pub reversible: bool,
    /// Undo command if reversible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Required prerequisites.
    pub prerequisites: Vec<String>,
}

/// Contract for a command argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgContract {
    /// Argument name.
    pub name: String,
    /// Argument type (string, number, path, etc.).
    pub arg_type: String,
    /// Whether this argument is required.
    pub required: bool,
    /// Description.
    pub description: String,
    /// Example value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// Contract for a command flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagContract {
    /// Flag name (e.g., "--dry-run").
    pub name: String,
    /// Short form (e.g., "-n").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Description.
    pub description: String,
    /// Default value.
    pub default: bool,
}

/// Registry of all known command contracts.
pub fn known_contracts() -> Vec<CommandContract> {
    vec![
        CommandContract {
            name: "spawn".to_string(),
            description: "Create a new workspace".to_string(),
            required_args: vec![ArgContract {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Workspace name or task ID".to_string(),
                example: Some("feature-auth".to_string()),
            }],
            optional_args: vec![],
            flags: vec![FlagContract {
                name: "--sync".to_string(),
                short: Some("-s".to_string()),
                description: "Sync with main after creation".to_string(),
                default: false,
            }],
            output_schema: "WorkspaceInfo".to_string(),
            side_effects: vec!["Creates git worktree".to_string(), "Registers workspace".to_string()],
            related_commands: vec!["switch".to_string(), "done".to_string()],
            examples: vec!["scp workspace spawn feature-auth".to_string()],
            reversible: true,
            undo_command: Some("abort".to_string()),
            prerequisites: vec!["Git repository initialized".to_string()],
        },
        CommandContract {
            name: "done".to_string(),
            description: "Complete workspace and merge".to_string(),
            required_args: vec![],
            optional_args: vec![ArgContract {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: false,
                description: "Workspace name (default: current)".to_string(),
                example: Some("feature-auth".to_string()),
            }],
            flags: vec![
                FlagContract {
                    name: "--message".to_string(),
                    short: Some("-m".to_string()),
                    description: "Commit message (auto-generated if not provided)".to_string(),
                    default: false,
                },
                FlagContract {
                    name: "--squash".to_string(),
                    short: None,
                    description: "Squash all commits into one".to_string(),
                    default: false,
                },
                FlagContract {
                    name: "--dry-run".to_string(),
                    short: None,
                    description: "Preview without executing".to_string(),
                    default: false,
                },
            ],
            output_schema: "DoneOutput".to_string(),
            side_effects: vec!["Merges branch into main".to_string(), "Removes worktree".to_string()],
            related_commands: vec!["spawn".to_string(), "abort".to_string(), "revert".to_string()],
            examples: vec!["scp workspace done feature-auth -m 'Add auth'".to_string()],
            reversible: true,
            undo_command: Some("revert".to_string()),
            prerequisites: vec!["Workspace exists".to_string(), "On main branch".to_string()],
        },
        CommandContract {
            name: "revert".to_string(),
            description: "Revert a specific session merge".to_string(),
            required_args: vec![ArgContract {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Session name to revert".to_string(),
                example: Some("feature-auth".to_string()),
            }],
            optional_args: vec![],
            flags: vec![FlagContract {
                name: "--dry-run".to_string(),
                short: None,
                description: "Preview without executing".to_string(),
                default: false,
            }],
            output_schema: "RevertOutput".to_string(),
            side_effects: vec!["Resets HEAD to pre-merge commit".to_string()],
            related_commands: vec!["done".to_string(), "recover".to_string()],
            examples: vec!["scp workspace revert feature-auth".to_string()],
            reversible: false,
            undo_command: None,
            prerequisites: vec!["Session merge exists in undo log".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_contract_serialization_roundtrip() {
        let contract = CommandContract {
            name: "test".to_string(),
            description: "Test command".to_string(),
            required_args: vec![],
            optional_args: vec![],
            flags: vec![],
            output_schema: "TestOutput".to_string(),
            side_effects: vec![],
            related_commands: vec![],
            examples: vec![],
            reversible: false,
            undo_command: None,
            prerequisites: vec![],
        };
        let json = serde_json::to_string(&contract).expect("serialize");
        let deserialized: CommandContract =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "test");
    }

    #[test]
    fn arg_contract_with_example() {
        let arg = ArgContract {
            name: "session".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            example: Some("feature-auth".to_string()),
        };
        let json = serde_json::to_string(&arg).expect("serialize");
        assert!(json.contains("feature-auth"));
    }

    #[test]
    fn flag_contract_serialization() {
        let flag = FlagContract {
            name: "--dry-run".to_string(),
            short: None,
            description: "Preview".to_string(),
            default: false,
        };
        let json = serde_json::to_string(&flag).expect("serialize");
        let deserialized: FlagContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "--dry-run");
        assert!(!deserialized.default);
    }

    #[test]
    fn known_contracts_not_empty() {
        let contracts = known_contracts();
        assert!(!contracts.is_empty());
        assert!(contracts.iter().any(|c| c.name == "spawn"));
        assert!(contracts.iter().any(|c| c.name == "done"));
        assert!(contracts.iter().any(|c| c.name == "revert"));
    }

    #[test]
    fn known_contracts_have_required_fields() {
        for contract in known_contracts() {
            assert!(!contract.name.is_empty());
            assert!(!contract.description.is_empty());
            assert!(!contract.output_schema.is_empty());
        }
    }
}
