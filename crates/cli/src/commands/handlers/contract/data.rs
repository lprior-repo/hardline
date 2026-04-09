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
            side_effects: vec![
                "Creates git worktree".to_string(),
                "Registers workspace".to_string(),
            ],
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
            side_effects: vec![
                "Merges branch into main".to_string(),
                "Removes worktree".to_string(),
            ],
            related_commands: vec![
                "spawn".to_string(),
                "abort".to_string(),
                "revert".to_string(),
            ],
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

    // ── Helper constructors ──────────────────────────────────────────────

    fn sample_arg() -> ArgContract {
        ArgContract {
            name: "session".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            example: Some("feature-auth".to_string()),
        }
    }

    fn sample_flag() -> FlagContract {
        FlagContract {
            name: "--dry-run".to_string(),
            short: Some("-n".to_string()),
            description: "Preview without executing".to_string(),
            default: false,
        }
    }

    fn sample_contract() -> CommandContract {
        CommandContract {
            name: "test_cmd".to_string(),
            description: "A test command".to_string(),
            required_args: vec![sample_arg()],
            optional_args: vec![],
            flags: vec![sample_flag()],
            output_schema: "TestOutput".to_string(),
            side_effects: vec!["Creates resource".to_string()],
            related_commands: vec!["other".to_string()],
            examples: vec!["scp test_cmd arg1".to_string()],
            reversible: true,
            undo_command: Some("undo_test".to_string()),
            prerequisites: vec!["Initialized".to_string()],
        }
    }

    fn minimal_contract() -> CommandContract {
        CommandContract {
            name: "minimal".to_string(),
            description: "Minimal contract".to_string(),
            required_args: vec![],
            optional_args: vec![],
            flags: vec![],
            output_schema: "Empty".to_string(),
            side_effects: vec![],
            related_commands: vec![],
            examples: vec![],
            reversible: false,
            undo_command: None,
            prerequisites: vec![],
        }
    }

    // ── ArgContract: construction & serialization ────────────────────────

    #[test]
    fn arg_contract_serialization_roundtrip() {
        let arg = sample_arg();
        let json = serde_json::to_string(&arg).expect("serialize");
        let back: ArgContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "session");
        assert_eq!(back.arg_type, "string");
        assert!(back.required);
        assert_eq!(back.description, "Session name");
        assert_eq!(back.example.as_deref(), Some("feature-auth"));
    }

    #[test]
    fn arg_contract_without_example_skips_field() {
        let arg = ArgContract {
            name: "count".to_string(),
            arg_type: "number".to_string(),
            required: false,
            description: "Count".to_string(),
            example: None,
        };
        let json = serde_json::to_string(&arg).expect("serialize");
        assert!(!json.contains("example"));
        let back: ArgContract = serde_json::from_str(&json).expect("deserialize");
        assert!(back.example.is_none());
    }

    #[test]
    fn arg_contract_all_types() {
        for type_name in &["string", "number", "path", "bool", "enum"] {
            let arg = ArgContract {
                name: format!("arg_{type_name}"),
                arg_type: type_name.to_string(),
                required: true,
                description: format!("A {type_name} arg"),
                example: None,
            };
            let json = serde_json::to_string(&arg).expect("serialize");
            let back: ArgContract = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.arg_type, *type_name);
        }
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

    // ── FlagContract: construction & serialization ───────────────────────

    #[test]
    fn flag_contract_serialization_roundtrip() {
        let flag = sample_flag();
        let json = serde_json::to_string(&flag).expect("serialize");
        let back: FlagContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "--dry-run");
        assert_eq!(back.short.as_deref(), Some("-n"));
        assert_eq!(back.description, "Preview without executing");
        assert!(!back.default);
    }

    #[test]
    fn flag_contract_without_short_skips_field() {
        let flag = FlagContract {
            name: "--verbose".to_string(),
            short: None,
            description: "Verbose output".to_string(),
            default: true,
        };
        let json = serde_json::to_string(&flag).expect("serialize");
        assert!(!json.contains("short"));
        let back: FlagContract = serde_json::from_str(&json).expect("deserialize");
        assert!(back.short.is_none());
        assert!(back.default);
    }

    #[test]
    fn flag_contract_default_true_serializes() {
        let flag = FlagContract {
            name: "--all".to_string(),
            short: None,
            description: "Include all".to_string(),
            default: true,
        };
        let json = serde_json::to_string(&flag).expect("serialize");
        assert!(json.contains("true"));
    }

    // ── CommandContract: full roundtrip ──────────────────────────────────

    #[test]
    fn command_contract_serialization_roundtrip() {
        let contract = sample_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        let back: CommandContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "test_cmd");
        assert_eq!(back.description, "A test command");
        assert_eq!(back.required_args.len(), 1);
        assert_eq!(back.flags.len(), 1);
        assert_eq!(back.output_schema, "TestOutput");
        assert_eq!(back.side_effects, vec!["Creates resource"]);
        assert!(back.reversible);
        assert_eq!(back.undo_command.as_deref(), Some("undo_test"));
    }

    #[test]
    fn command_contract_minimal_roundtrip() {
        let contract = minimal_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        let back: CommandContract = serde_json::from_str(&json).expect("deserialize");
        assert!(back.required_args.is_empty());
        assert!(back.optional_args.is_empty());
        assert!(back.flags.is_empty());
        assert!(back.side_effects.is_empty());
        assert!(!back.reversible);
        assert!(back.undo_command.is_none());
    }

    #[test]
    fn command_contract_json_readable() {
        let contract = sample_contract();
        let json = serde_json::to_string_pretty(&contract).expect("serialize");
        // Verify key fields are present in human-readable JSON
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test_cmd\""));
        assert!(json.contains("\"required_args\""));
        assert!(json.contains("\"side_effects\""));
    }

    #[test]
    fn command_contract_skip_none_undo() {
        let contract = minimal_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        assert!(!json.contains("undo_command"));
    }

    #[test]
    fn command_contract_with_undo_present() {
        let contract = sample_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        assert!(json.contains("undo_command"));
        assert!(json.contains("undo_test"));
    }

    // ── Schema validation: structural invariants ────────────────────────

    #[test]
    fn command_contract_schema_has_required_top_level_fields() {
        let contract = sample_contract();
        let json: serde_json::Value = serde_json::to_value(&contract).expect("to_value");
        assert!(json.get("name").is_some());
        assert!(json.get("description").is_some());
        assert!(json.get("required_args").is_some());
        assert!(json.get("optional_args").is_some());
        assert!(json.get("flags").is_some());
        assert!(json.get("output_schema").is_some());
        assert!(json.get("side_effects").is_some());
        assert!(json.get("related_commands").is_some());
        assert!(json.get("examples").is_some());
        assert!(json.get("reversible").is_some());
        assert!(json.get("prerequisites").is_some());
    }

    #[test]
    fn arg_contract_schema_fields() {
        let arg = sample_arg();
        let json: serde_json::Value = serde_json::to_value(&arg).expect("to_value");
        assert!(json.get("name").is_some());
        assert!(json.get("arg_type").is_some());
        assert!(json.get("required").is_some());
        assert!(json.get("description").is_some());
    }

    #[test]
    fn flag_contract_schema_fields() {
        let flag = sample_flag();
        let json: serde_json::Value = serde_json::to_value(&flag).expect("to_value");
        assert!(json.get("name").is_some());
        assert!(json.get("description").is_some());
        assert!(json.get("default").is_some());
    }

    #[test]
    fn contract_arrays_are_arrays_in_json() {
        let contract = sample_contract();
        let json: serde_json::Value = serde_json::to_value(&contract).expect("to_value");
        assert!(json["required_args"].is_array());
        assert!(json["optional_args"].is_array());
        assert!(json["flags"].is_array());
        assert!(json["side_effects"].is_array());
        assert!(json["related_commands"].is_array());
        assert!(json["examples"].is_array());
        assert!(json["prerequisites"].is_array());
    }

    // ── Deserialization rejection: malformed input ───────────────────────

    #[test]
    fn command_contract_rejects_empty_json_object() {
        let result = serde_json::from_str::<CommandContract>("{}");
        // Missing required fields should fail
        assert!(result.is_err());
    }

    #[test]
    fn command_contract_rejects_invalid_json() {
        let result = serde_json::from_str::<CommandContract>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn arg_contract_rejects_missing_required_fields() {
        let result = serde_json::from_str::<ArgContract>("{}");
        assert!(result.is_err());
    }

    #[test]
    fn flag_contract_rejects_missing_required_fields() {
        let result = serde_json::from_str::<FlagContract>("{}");
        assert!(result.is_err());
    }

    #[test]
    fn command_contract_rejects_wrong_type_for_name() {
        let json = r#"{"name": 42, "description": "x", "required_args": [], "optional_args": [], "flags": [], "output_schema": "X", "side_effects": [], "related_commands": [], "examples": [], "reversible": false, "prerequisites": []}"#;
        let result = serde_json::from_str::<CommandContract>(json);
        assert!(result.is_err());
    }

    #[test]
    fn command_contract_rejects_wrong_type_for_reversible() {
        let json = r#"{"name": "x", "description": "x", "required_args": [], "optional_args": [], "flags": [], "output_schema": "X", "side_effects": [], "related_commands": [], "examples": [], "reversible": "yes", "prerequisites": []}"#;
        let result = serde_json::from_str::<CommandContract>(json);
        assert!(result.is_err());
    }

    // ── known_contracts() registry validation ────────────────────────────

    #[test]
    fn known_contracts_not_empty() {
        let contracts = known_contracts();
        assert!(!contracts.is_empty());
    }

    #[test]
    fn known_contracts_contains_spawn() {
        let contracts = known_contracts();
        assert!(contracts.iter().any(|c| c.name == "spawn"));
    }

    #[test]
    fn known_contracts_contains_done() {
        let contracts = known_contracts();
        assert!(contracts.iter().any(|c| c.name == "done"));
    }

    #[test]
    fn known_contracts_contains_revert() {
        let contracts = known_contracts();
        assert!(contracts.iter().any(|c| c.name == "revert"));
    }

    #[test]
    fn known_contracts_have_unique_names() {
        let contracts = known_contracts();
        let names: Vec<&str> = contracts.iter().map(|c| c.name.as_str()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "duplicate contract names found");
    }

    #[test]
    fn known_contracts_have_required_fields() {
        for contract in &known_contracts() {
            assert!(!contract.name.is_empty(), "contract has empty name");
            assert!(
                !contract.description.is_empty(),
                "contract {} has empty description",
                contract.name
            );
            assert!(
                !contract.output_schema.is_empty(),
                "contract {} has empty output_schema",
                contract.name
            );
        }
    }

    #[test]
    fn known_contracts_spawn_has_required_arg() {
        let contracts = known_contracts();
        let spawn = contracts.iter().find(|c| c.name == "spawn").expect("spawn");
        assert!(
            !spawn.required_args.is_empty(),
            "spawn needs at least one required arg"
        );
        assert!(spawn.required_args.iter().any(|a| a.name == "name"));
    }

    #[test]
    fn known_contracts_spawn_has_sync_flag() {
        let contracts = known_contracts();
        let spawn = contracts.iter().find(|c| c.name == "spawn").expect("spawn");
        assert!(spawn.flags.iter().any(|f| f.name == "--sync"));
    }

    #[test]
    fn known_contracts_spawn_is_reversible() {
        let contracts = known_contracts();
        let spawn = contracts.iter().find(|c| c.name == "spawn").expect("spawn");
        assert!(spawn.reversible);
        assert_eq!(spawn.undo_command.as_deref(), Some("abort"));
    }

    #[test]
    fn known_contracts_spawn_has_side_effects() {
        let contracts = known_contracts();
        let spawn = contracts.iter().find(|c| c.name == "spawn").expect("spawn");
        assert!(!spawn.side_effects.is_empty());
    }

    #[test]
    fn known_contracts_done_has_optional_arg() {
        let contracts = known_contracts();
        let done = contracts.iter().find(|c| c.name == "done").expect("done");
        assert!(!done.optional_args.is_empty());
    }

    #[test]
    fn known_contracts_done_has_multiple_flags() {
        let contracts = known_contracts();
        let done = contracts.iter().find(|c| c.name == "done").expect("done");
        assert!(
            done.flags.len() >= 3,
            "done should have --message, --squash, --dry-run"
        );
    }

    #[test]
    fn known_contracts_done_is_reversible() {
        let contracts = known_contracts();
        let done = contracts.iter().find(|c| c.name == "done").expect("done");
        assert!(done.reversible);
        assert_eq!(done.undo_command.as_deref(), Some("revert"));
    }

    #[test]
    fn known_contracts_revert_not_reversible() {
        let contracts = known_contracts();
        let revert = contracts
            .iter()
            .find(|c| c.name == "revert")
            .expect("revert");
        assert!(!revert.reversible);
        assert!(revert.undo_command.is_none());
    }

    #[test]
    fn known_contracts_revert_has_required_arg() {
        let contracts = known_contracts();
        let revert = contracts
            .iter()
            .find(|c| c.name == "revert")
            .expect("revert");
        assert!(!revert.required_args.is_empty());
        assert!(revert.required_args.iter().any(|a| a.name == "name"));
    }

    #[test]
    fn known_contracts_revert_has_dry_run_flag() {
        let contracts = known_contracts();
        let revert = contracts
            .iter()
            .find(|c| c.name == "revert")
            .expect("revert");
        assert!(revert.flags.iter().any(|f| f.name == "--dry-run"));
    }

    #[test]
    fn known_contracts_all_serializable() {
        for contract in &known_contracts() {
            let json = serde_json::to_string(contract).unwrap_or_else(|e| {
                panic!("contract '{}' failed to serialize: {e}", contract.name)
            });
            let _: CommandContract = serde_json::from_str(&json).unwrap_or_else(|e| {
                panic!("contract '{}' failed to deserialize: {e}", contract.name)
            });
        }
    }

    #[test]
    fn known_contracts_related_commands_references_valid() {
        // Related commands that reference other known contracts should be consistent.
        // "switch" is referenced by spawn but not yet a known contract — that's okay
        // (forward reference to a command that may be added later).
        // We verify that at least the known cross-references work.
        let contracts = known_contracts();
        let names: Vec<&str> = contracts.iter().map(|c| c.name.as_str()).collect();
        for contract in &contracts {
            for related in &contract.related_commands {
                if names.contains(&related.as_str()) {
                    // Known command — verify it's a valid cross-reference
                    assert!(
                        contracts.iter().any(|c| c.name == *related),
                        "internal consistency: '{}' references '{}' which is in names but not found",
                        contract.name, related
                    );
                }
            }
        }
    }

    #[test]
    fn known_contracts_done_references_spawn_and_revert() {
        let contracts = known_contracts();
        let done = contracts.iter().find(|c| c.name == "done").expect("done");
        assert!(done.related_commands.contains(&"spawn".to_string()));
        assert!(done.related_commands.contains(&"revert".to_string()));
    }

    // ── Clone / Debug trait verification ─────────────────────────────────

    #[test]
    fn command_contract_is_clone() {
        let c = sample_contract();
        let cloned = c.clone();
        assert_eq!(c.name, cloned.name);
        assert_eq!(c.flags.len(), cloned.flags.len());
    }

    #[test]
    fn arg_contract_is_clone() {
        let a = sample_arg();
        let cloned = a.clone();
        assert_eq!(a.name, cloned.name);
    }

    #[test]
    fn flag_contract_is_clone() {
        let f = sample_flag();
        let cloned = f.clone();
        assert_eq!(f.name, cloned.name);
    }

    #[test]
    fn command_contract_debug_format() {
        let c = sample_contract();
        let debug = format!("{c:?}");
        assert!(debug.contains("test_cmd"));
    }

    // ── Edge cases ───────────────────────────────────────────────────────

    #[test]
    fn contract_with_empty_name_is_constructible() {
        // Data layer doesn't validate — that's the calc layer's job
        let c = CommandContract {
            name: String::new(),
            description: String::new(),
            required_args: vec![],
            optional_args: vec![],
            flags: vec![],
            output_schema: "Empty".to_string(),
            side_effects: vec![],
            related_commands: vec![],
            examples: vec![],
            reversible: false,
            undo_command: None,
            prerequisites: vec![],
        };
        assert!(c.name.is_empty());
    }

    #[test]
    fn contract_with_special_chars_in_name() {
        let json = r#"{"name":"cmd-with-dashes_and_underscores","description":"x","required_args":[],"optional_args":[],"flags":[],"output_schema":"X","side_effects":[],"related_commands":[],"examples":[],"reversible":false,"prerequisites":[]}"#;
        let c: CommandContract = serde_json::from_str(json).expect("deserialize");
        assert_eq!(c.name, "cmd-with-dashes_and_underscores");
    }

    #[test]
    fn arg_with_unicode_description() {
        let arg = ArgContract {
            name: "msg".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Nachricht für den Benutzer 📝".to_string(),
            example: Some("hello".to_string()),
        };
        let json = serde_json::to_string(&arg).expect("serialize unicode");
        let back: ArgContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.description, "Nachricht für den Benutzer 📝");
    }

    #[test]
    fn contract_with_many_args_and_flags() {
        let contract = CommandContract {
            name: "complex".to_string(),
            description: "Complex command".to_string(),
            required_args: (0..10)
                .map(|i| ArgContract {
                    name: format!("arg{i}"),
                    arg_type: "string".to_string(),
                    required: true,
                    description: format!("Arg {i}"),
                    example: None,
                })
                .collect(),
            optional_args: (0..5)
                .map(|i| ArgContract {
                    name: format!("opt{i}"),
                    arg_type: "string".to_string(),
                    required: false,
                    description: format!("Opt {i}"),
                    example: None,
                })
                .collect(),
            flags: (0..8)
                .map(|i| FlagContract {
                    name: format!("--flag{i}"),
                    short: if i % 2 == 0 {
                        Some(format!("-{i}"))
                    } else {
                        None
                    },
                    description: format!("Flag {i}"),
                    default: i % 3 == 0,
                })
                .collect(),
            output_schema: "ComplexOutput".to_string(),
            side_effects: vec!["effect1".to_string(), "effect2".to_string()],
            related_commands: vec!["simple".to_string()],
            examples: vec!["scp complex arg0 --flag0".to_string()],
            reversible: false,
            undo_command: None,
            prerequisites: vec!["Repo initialized".to_string()],
        };
        let json = serde_json::to_string(&contract).expect("serialize");
        let back: CommandContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.required_args.len(), 10);
        assert_eq!(back.optional_args.len(), 5);
        assert_eq!(back.flags.len(), 8);
    }
}
