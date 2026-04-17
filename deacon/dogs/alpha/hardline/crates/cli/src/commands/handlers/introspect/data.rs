//! Data types for the introspect command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Target for introspection: all commands or a specific one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntrospectTarget {
    /// Show all known commands.
    All,
    /// Show details for a specific command.
    Specific(String),
}

/// Options for the introspect command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct IntrospectOptions {
    /// Which command(s) to introspect.
    pub target: IntrospectTarget,
}

impl IntrospectOptions {
    /// Construct options from an optional CLI target string.
    ///
    /// `None` means show all commands; `Some(name)` targets a specific command.
    pub fn from_cli(target: Option<String>) -> Self {
        Self {
            target: target.map_or(IntrospectTarget::All, IntrospectTarget::Specific),
        }
    }
}

/// Complete introspection result for a single command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Command name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Command aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Required arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Optional flags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagInfo>,
    /// Usage examples.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ExampleInfo>,
    /// Side effects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    /// Known error conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_conditions: Vec<ErrorConditionInfo>,
    /// Whether the command requires initialization.
    /// Metadata booleans kept for registry lookup; not a state machine violation.
    pub requires_init: bool,
    /// Whether the command requires git.
    /// Metadata booleans kept for registry lookup; not a state machine violation.
    pub requires_git: bool,
}

/// Argument metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentInfo {
    /// Argument name.
    pub name: String,
    /// Argument type (string, number, path, etc.).
    pub arg_type: String,
    /// Whether this argument is required.
    pub required: bool,
    /// Description.
    pub description: String,
    /// Example values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Flag metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagInfo {
    /// Long flag name (e.g., "dry-run").
    pub long: String,
    /// Short flag name (e.g., "n").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Description.
    pub description: String,
    /// Flag type (bool, string, enum).
    pub flag_type: String,
    /// Default value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// Usage example.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleInfo {
    /// Example command line.
    pub command: String,
    /// What the example does.
    pub description: String,
}

/// Known error condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorConditionInfo {
    /// Error code.
    pub code: String,
    /// Description.
    pub description: String,
    /// How to resolve.
    pub resolution: String,
}

// ============================================================================
// Per-command factory functions
// ============================================================================

fn init_command_info() -> CommandInfo {
    CommandInfo {
        name: "init".to_string(),
        description: "Initialize hardline in a Git repository".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![ExampleInfo {
            command: "scp init".to_string(),
            description: "Initialize hardline in current directory".to_string(),
        }],
        side_effects: vec![
            "Creates .scp directory".to_string(),
            "Creates config.toml".to_string(),
            "Creates state.db".to_string(),
        ],
        error_conditions: vec![ErrorConditionInfo {
            code: "ALREADY_INITIALIZED".to_string(),
            description: "Hardline already initialized".to_string(),
            resolution: "Remove .scp directory to reinitialize".to_string(),
        }],
        requires_init: false,
        requires_git: true,
    }
}

fn add_command_info() -> CommandInfo {
    CommandInfo {
        name: "add".to_string(),
        description: "Create new parallel development workspace".to_string(),
        aliases: vec!["a".to_string(), "new".to_string()],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Workspace name".to_string(),
            examples: vec!["feature-auth".to_string(), "bugfix-123".to_string()],
        }],
        flags: vec![
            FlagInfo {
                long: "no-hooks".to_string(),
                short: None,
                description: "Skip post_create hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
            FlagInfo {
                long: "no-open".to_string(),
                short: None,
                description: "Create workspace but don't open terminal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
        ],
        examples: vec![
            ExampleInfo {
                command: "scp add feature-auth".to_string(),
                description: "Create workspace".to_string(),
            },
            ExampleInfo {
                command: "scp add bugfix-123 --no-hooks".to_string(),
                description: "Create without running hooks".to_string(),
            },
        ],
        side_effects: vec![
            "Creates git worktree".to_string(),
            "Executes post_create hooks".to_string(),
            "Records workspace in state.db".to_string(),
        ],
        error_conditions: vec![
            ErrorConditionInfo {
                code: "WORKSPACE_ALREADY_EXISTS".to_string(),
                description: "Workspace with this name already exists".to_string(),
                resolution: "Choose a different name or remove the existing workspace".to_string(),
            },
            ErrorConditionInfo {
                code: "INVALID_WORKSPACE_NAME".to_string(),
                description: "Workspace name contains invalid characters".to_string(),
                resolution: "Use only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            },
        ],
        requires_init: true,
        requires_git: true,
    }
}

fn remove_command_info() -> CommandInfo {
    CommandInfo {
        name: "remove".to_string(),
        description: "Remove a workspace".to_string(),
        aliases: vec!["rm".to_string(), "delete".to_string()],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the workspace to remove".to_string(),
            examples: vec!["my-workspace".to_string()],
        }],
        flags: vec![
            FlagInfo {
                long: "force".to_string(),
                short: Some("f".to_string()),
                description: "Skip confirmation prompt and hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
            FlagInfo {
                long: "merge".to_string(),
                short: Some("m".to_string()),
                description: "Squash-merge to main before removal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
        ],
        examples: vec![
            ExampleInfo {
                command: "scp remove my-workspace".to_string(),
                description: "Remove workspace".to_string(),
            },
            ExampleInfo {
                command: "scp remove my-workspace -f".to_string(),
                description: "Remove and skip hooks".to_string(),
            },
        ],
        side_effects: vec![
            "Removes git worktree".to_string(),
            "Removes workspace from state.db".to_string(),
        ],
        error_conditions: vec![ErrorConditionInfo {
            code: "WORKSPACE_NOT_FOUND".to_string(),
            description: "The specified workspace does not exist".to_string(),
            resolution: "List workspaces with 'scp list' to verify the name".to_string(),
        }],
        requires_init: true,
        requires_git: true,
    }
}

fn list_command_info() -> CommandInfo {
    CommandInfo {
        name: "list".to_string(),
        description: "List all workspaces".to_string(),
        aliases: vec!["ls".to_string()],
        arguments: vec![],
        flags: vec![FlagInfo {
            long: "all".to_string(),
            short: None,
            description: "Include completed and failed workspaces".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        }],
        examples: vec![
            ExampleInfo {
                command: "scp list".to_string(),
                description: "List active workspaces".to_string(),
            },
            ExampleInfo {
                command: "scp list --all".to_string(),
                description: "List all workspaces including completed".to_string(),
            },
        ],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: true,
        requires_git: false,
    }
}

fn status_command_info() -> CommandInfo {
    CommandInfo {
        name: "status".to_string(),
        description: "Show detailed workspace status".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Workspace name (shows all if omitted)".to_string(),
            examples: vec!["my-workspace".to_string()],
        }],
        flags: vec![],
        examples: vec![
            ExampleInfo {
                command: "scp status".to_string(),
                description: "Show status of all workspaces".to_string(),
            },
            ExampleInfo {
                command: "scp status my-workspace".to_string(),
                description: "Show status of specific workspace".to_string(),
            },
        ],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: true,
        requires_git: true,
    }
}

fn done_command_info() -> CommandInfo {
    CommandInfo {
        name: "done".to_string(),
        description: "Complete workspace and merge".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Workspace name (default: current)".to_string(),
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![
            FlagInfo {
                long: "message".to_string(),
                short: Some("m".to_string()),
                description: "Commit message (auto-generated if not provided)".to_string(),
                flag_type: "string".to_string(),
                default: None,
            },
            FlagInfo {
                long: "squash".to_string(),
                short: None,
                description: "Squash all commits into one".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
            FlagInfo {
                long: "dry-run".to_string(),
                short: None,
                description: "Preview without executing".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
            },
        ],
        examples: vec![ExampleInfo {
            command: "scp done feature-auth -m 'Add auth'".to_string(),
            description: "Complete and merge workspace".to_string(),
        }],
        side_effects: vec![
            "Merges branch into main".to_string(),
            "Removes worktree".to_string(),
        ],
        error_conditions: vec![],
        requires_init: true,
        requires_git: true,
    }
}

fn sync_command_info() -> CommandInfo {
    CommandInfo {
        name: "sync".to_string(),
        description: "Sync workspace with main (rebase)".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Workspace name (syncs current if omitted)".to_string(),
            examples: vec!["my-workspace".to_string()],
        }],
        flags: vec![],
        examples: vec![ExampleInfo {
            command: "scp sync my-workspace".to_string(),
            description: "Sync workspace with main branch".to_string(),
        }],
        side_effects: vec![
            "Rebases workspace onto main".to_string(),
            "Updates last_synced timestamp".to_string(),
        ],
        error_conditions: vec![ErrorConditionInfo {
            code: "CONFLICTS".to_string(),
            description: "Rebase resulted in conflicts".to_string(),
            resolution: "Resolve conflicts manually".to_string(),
        }],
        requires_init: true,
        requires_git: true,
    }
}

fn diff_command_info() -> CommandInfo {
    CommandInfo {
        name: "diff".to_string(),
        description: "Show diff between workspace and main".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Workspace name".to_string(),
            examples: vec!["my-workspace".to_string()],
        }],
        flags: vec![FlagInfo {
            long: "stat".to_string(),
            short: None,
            description: "Show diffstat only".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        }],
        examples: vec![
            ExampleInfo {
                command: "scp diff my-workspace".to_string(),
                description: "Show full diff".to_string(),
            },
            ExampleInfo {
                command: "scp diff my-workspace --stat".to_string(),
                description: "Show diffstat summary".to_string(),
            },
        ],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: true,
        requires_git: true,
    }
}

fn introspect_command_info() -> CommandInfo {
    CommandInfo {
        name: "introspect".to_string(),
        description: "Discover hardline capabilities".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Command to introspect (shows all if omitted)".to_string(),
            examples: vec!["add".to_string(), "remove".to_string()],
        }],
        flags: vec![],
        examples: vec![
            ExampleInfo {
                command: "scp introspect".to_string(),
                description: "Show all capabilities".to_string(),
            },
            ExampleInfo {
                command: "scp introspect add".to_string(),
                description: "Get add command schema".to_string(),
            },
        ],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: false,
        requires_git: false,
    }
}

fn doctor_command_info() -> CommandInfo {
    CommandInfo {
        name: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        aliases: vec!["check".to_string()],
        arguments: vec![],
        flags: vec![FlagInfo {
            long: "fix".to_string(),
            short: None,
            description: "Auto-fix issues where possible".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        }],
        examples: vec![
            ExampleInfo {
                command: "scp doctor".to_string(),
                description: "Check system health".to_string(),
            },
            ExampleInfo {
                command: "scp doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
            },
        ],
        side_effects: vec!["May fix issues with --fix flag".to_string()],
        error_conditions: vec![],
        requires_init: false,
        requires_git: false,
    }
}

fn query_command_info() -> CommandInfo {
    CommandInfo {
        name: "query".to_string(),
        description: "Query system state".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "query_type".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Type of query".to_string(),
            examples: vec![
                "session-exists".to_string(),
                "session-count".to_string(),
                "can-run".to_string(),
            ],
        }],
        flags: vec![],
        examples: vec![
            ExampleInfo {
                command: "scp query session-exists my-workspace".to_string(),
                description: "Check if workspace exists".to_string(),
            },
            ExampleInfo {
                command: "scp query can-run add".to_string(),
                description: "Check if add command can run".to_string(),
            },
        ],
        side_effects: vec![],
        error_conditions: vec![],
        requires_init: false,
        requires_git: false,
    }
}

fn revert_command_info() -> CommandInfo {
    CommandInfo {
        name: "revert".to_string(),
        description: "Revert a specific session merge".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentInfo {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name to revert".to_string(),
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![FlagInfo {
            long: "dry-run".to_string(),
            short: None,
            description: "Preview without executing".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        }],
        examples: vec![ExampleInfo {
            command: "scp revert feature-auth".to_string(),
            description: "Revert a merged session".to_string(),
        }],
        side_effects: vec!["Resets HEAD to pre-merge commit".to_string()],
        error_conditions: vec![ErrorConditionInfo {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session merge not found in undo log".to_string(),
            resolution: "Verify the session name with 'scp list'".to_string(),
        }],
        requires_init: true,
        requires_git: true,
    }
}

/// Return all known command introspection data.
///
/// Pure function: returns a static list of command metadata,
/// assembled from per-command factory functions.
pub fn known_commands() -> Vec<CommandInfo> {
    vec![
        init_command_info(),
        add_command_info(),
        remove_command_info(),
        list_command_info(),
        status_command_info(),
        done_command_info(),
        sync_command_info(),
        diff_command_info(),
        introspect_command_info(),
        doctor_command_info(),
        query_command_info(),
        revert_command_info(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_commands_not_empty() {
        let commands = known_commands();
        assert!(!commands.is_empty());
    }

    #[test]
    fn known_commands_contains_core_commands() {
        let commands = known_commands();
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"init"));
        assert!(names.contains(&"add"));
        assert!(names.contains(&"remove"));
        assert!(names.contains(&"list"));
        assert!(names.contains(&"done"));
        assert!(names.contains(&"revert"));
        assert!(names.contains(&"introspect"));
        assert!(names.contains(&"doctor"));
        assert!(names.contains(&"query"));
    }

    #[test]
    fn all_commands_have_required_fields() {
        for cmd in known_commands() {
            assert!(!cmd.name.is_empty(), "command name must not be empty");
            assert!(
                !cmd.description.is_empty(),
                "command '{}' must have a description",
                cmd.name
            );
        }
    }

    #[test]
    fn command_info_serialization_roundtrip() {
        let info = CommandInfo {
            name: "test".to_string(),
            description: "Test command".to_string(),
            aliases: vec!["t".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![],
            side_effects: vec![],
            error_conditions: vec![],
            requires_init: true,
            requires_git: false,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: CommandInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.aliases.len(), 1);
    }

    #[test]
    fn argument_info_serialization() {
        let arg = ArgumentInfo {
            name: "session".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            examples: vec!["feature-auth".to_string()],
        };
        let json = serde_json::to_string(&arg).expect("serialize");
        assert!(json.contains("feature-auth"));
    }

    #[test]
    fn flag_info_serialization() {
        let flag = FlagInfo {
            long: "dry-run".to_string(),
            short: Some("n".to_string()),
            description: "Preview".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
        };
        let json = serde_json::to_string(&flag).expect("serialize");
        let deserialized: FlagInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.long, "dry-run");
        assert_eq!(deserialized.short, Some("n".to_string()));
    }

    #[test]
    fn example_info_serialization() {
        let example = ExampleInfo {
            command: "scp add foo".to_string(),
            description: "Add workspace".to_string(),
        };
        let json = serde_json::to_string(&example).expect("serialize");
        let deserialized: ExampleInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.command, "scp add foo");
    }

    #[test]
    fn error_condition_info_serialization() {
        let ec = ErrorConditionInfo {
            code: "NOT_FOUND".to_string(),
            description: "Not found".to_string(),
            resolution: "Check name".to_string(),
        };
        let json = serde_json::to_string(&ec).expect("serialize");
        let deserialized: ErrorConditionInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.code, "NOT_FOUND");
    }

    #[test]
    fn command_names_are_unique() {
        let commands = known_commands();
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        let unique_count = names.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(names.len(), unique_count, "command names must be unique");
    }

    #[test]
    fn add_command_has_required_argument() {
        let cmds = known_commands();
        let add_cmd = cmds
            .iter()
            .find(|c| c.name == "add")
            .expect("add command must exist");
        assert!(add_cmd
            .arguments
            .iter()
            .any(|a| a.name == "name" && a.required));
    }

    #[test]
    fn done_command_is_reversible() {
        let cmds = known_commands();
        let done_cmd = cmds
            .iter()
            .find(|c| c.name == "done")
            .expect("done command must exist");
        // done should have side effects (merge + remove)
        assert!(!done_cmd.side_effects.is_empty());
    }

    #[test]
    fn introspect_command_requires_nothing() {
        let cmds = known_commands();
        let introspect_cmd = cmds
            .iter()
            .find(|c| c.name == "introspect")
            .expect("introspect command must exist");
        assert!(!introspect_cmd.requires_init);
        assert!(!introspect_cmd.requires_git);
    }

    #[test]
    fn introspect_target_all_is_default() {
        let target = IntrospectTarget::All;
        assert!(matches!(target, IntrospectTarget::All));
    }

    #[test]
    fn introspect_target_specific_holds_name() {
        let target = IntrospectTarget::Specific("add".to_string());
        assert_eq!(
            match target {
                IntrospectTarget::Specific(name) => name,
                _ => String::new(),
            },
            "add"
        );
    }

    #[test]
    fn introspect_options_with_all_target() {
        let options = IntrospectOptions {
            target: IntrospectTarget::All,
        };
        assert!(matches!(options.target, IntrospectTarget::All));
    }

    #[test]
    fn introspect_options_with_specific_target() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("add".to_string()),
        };
        assert!(matches!(options.target, IntrospectTarget::Specific(_)));
    }
}
