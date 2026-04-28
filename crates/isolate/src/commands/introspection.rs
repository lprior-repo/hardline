//! Introspection command - discover isolate capabilities
//!
//! This module provides structured metadata about isolate capabilities,
//! enabling AI agents to discover features and understand system state.
//!
//! # Architecture
//!
//! - **Types**: CommandIntrospection, FlagSpec, ArgumentSpec defined in isolate_core
//! - **Helpers**: Pure functions for creating introspection data structures
//! - **Output**: Serialization through isolate_core::json::SchemaEnvelope

use isolate_core::introspection::{
    ArgumentSpec, CommandExample, CommandIntrospection, ErrorCondition, FlagSpec, Prerequisites,
};
use isolate_core::json::SchemaEnvelope;
use isolate_core::OutputFormat;

use crate::error::IsolateError;

/// Result type for commands
pub type Result<T> = std::result::Result<T, IsolateError>;

// ============================================================================
// Command Introspection Definitions
// ============================================================================

/// List command filter flags with comprehensive documentation
///
/// Returns a vector of filter flags used by the list command.
/// Factored out to reduce duplication and improve maintainability.
#[must_use]
pub fn create_list_filter_flags() -> Vec<FlagSpec> {
    vec![
        create_bool_flag("all", "Include completed and failed sessions"),
        create_bool_flag("json", "Output as JSON"),
        create_string_filter_flag(
            "bead",
            "b",
            "Filter by bead ID or pattern - supports dynamic values like 'feature-*'",
        ),
        create_string_filter_flag(
            "agent",
            "a",
            "Filter by agent name or pattern - supports dynamic values",
        ),
    ]
}

/// List command examples demonstrating filtering capabilities
///
/// Returns comprehensive examples showing basic usage and advanced filter combinations.
#[must_use]
pub fn create_list_examples() -> Vec<CommandExample> {
    vec![
        create_example("scp list", "List active sessions"),
        create_example("scp list --all", "List all sessions including completed"),
        create_example("scp list --bead feature-123", "List sessions for bead feature-123"),
        create_example("scp list --agent alice", "List sessions assigned to alice"),
        create_example(
            "scp list --bead feature-123 --agent alice",
            "List feature-123 sessions assigned to alice",
        ),
    ]
}

/// List command error conditions with recovery guidance
///
/// Documents expected error scenarios and how to resolve them.
#[must_use]
pub fn create_list_error_conditions() -> Vec<ErrorCondition> {
    vec![create_error_condition(
        "NO_MATCHING_SESSIONS",
        "No sessions match the specified filter criteria (bead, agent, status, etc.)",
        "Review filter parameters: check bead IDs or agent names, try with fewer restrictions",
    )]
}

/// Add command flags with comprehensive documentation
///
/// Returns the flags for the add command, organized for clarity.
#[must_use]
pub fn create_add_flags() -> Vec<FlagSpec> {
    vec![
        create_bool_flag("no-hooks", "Skip post_create hooks"),
        create_bool_flag("no-open", "Create workspace but don't open terminal"),
    ]
}

/// Add command examples showing various usage patterns
#[must_use]
pub fn create_add_examples() -> Vec<CommandExample> {
    vec![
        create_example("scp add feature-auth", "Create session"),
        create_example("scp add bugfix-123 --no-hooks", "Create without running hooks"),
        create_example("scp add experiment -t minimal", "Create with minimal layout"),
    ]
}

/// Add command error conditions with resolution guidance
#[must_use]
pub fn create_add_error_conditions() -> Vec<ErrorCondition> {
    vec![
        create_error_condition(
            "SESSION_ALREADY_EXISTS",
            "Session with this name already exists in the database",
            "Choose a different session name or remove the existing session with 'scp remove'",
        ),
        create_error_condition(
            "INVALID_SESSION_NAME",
            "Session name contains invalid characters or does not match naming rules",
            "Use only alphanumeric characters, hyphens, and underscores; must start with a letter",
        ),
    ]
}

/// Remove command flags
///
/// Provides control over removal behavior: force-skip, merge, and branch preservation.
#[must_use]
pub fn create_remove_flags() -> Vec<FlagSpec> {
    vec![
        FlagSpec {
            long: "force".to_string(),
            short: Some("f".to_string()),
            description: "Skip confirmation prompt and hooks".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
        FlagSpec {
            long: "merge".to_string(),
            short: Some("m".to_string()),
            description: "Squash-merge to main before removal".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
        FlagSpec {
            long: "keep-branch".to_string(),
            short: Some("k".to_string()),
            description: "Preserve branch after removal".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
    ]
}

/// Remove command examples showing cleanup patterns
#[must_use]
pub fn create_remove_examples() -> Vec<CommandExample> {
    vec![
        create_example("scp remove my-session", "Remove session (no confirmation)"),
        create_example("scp remove my-session -f", "Remove and skip pre_remove hooks"),
        create_example("scp remove my-session -m", "Merge changes before removing"),
    ]
}

/// Remove command error conditions
#[must_use]
pub fn create_remove_error_conditions() -> Vec<ErrorCondition> {
    vec![create_error_condition(
        "SESSION_NOT_FOUND",
        "The specified session does not exist in the database",
        "List active sessions with 'scp list' to verify the session name",
    )]
}

/// Status command flags
#[must_use]
pub fn create_status_flags() -> Vec<FlagSpec> {
    vec![
        FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
        FlagSpec {
            long: "watch".to_string(),
            short: None,
            description: "Continuously update status".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
    ]
}

// ============================================================================
// Command Introspection Builders
// ============================================================================

/// Get introspection for the add command
#[must_use]
pub fn get_add_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "add".to_string(),
        description: "Create new parallel development session".to_string(),
        aliases: vec!["a".to_string(), "new".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z0-9_-]+$".to_string()),
            examples: vec![
                "feature-auth".to_string(),
                "bugfix-123".to_string(),
                "experiment".to_string(),
            ],
        }],
        flags: create_add_flags(),
        examples: create_add_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec!["Session name must be unique".to_string()],
        },
        side_effects: vec![
            "Creates JJ workspace".to_string(),
            "Executes post_create hooks".to_string(),
            "Records session in state.db".to_string(),
        ],
        error_conditions: create_add_error_conditions(),
    }
}

/// Get introspection for the remove command
#[must_use]
pub fn get_remove_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "remove".to_string(),
        description: "Remove a session and its workspace".to_string(),
        aliases: vec!["rm".to_string(), "delete".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to remove".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: create_remove_flags(),
        examples: create_remove_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![
            "Removes JJ workspace".to_string(),
            "Removes session from state.db".to_string(),
        ],
        error_conditions: create_remove_error_conditions(),
    }
}

/// Get introspection for the list command
#[must_use]
pub fn get_list_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "list".to_string(),
        description: "List all sessions".to_string(),
        aliases: vec!["ls".to_string()],
        arguments: vec![],
        flags: create_list_filter_flags(),
        examples: create_list_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: create_list_error_conditions(),
    }
}

/// Get introspection for the init command
#[must_use]
pub fn get_init_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "init".to_string(),
        description: "Initialize isolate in a JJ repository".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![CommandExample {
            command: "scp init".to_string(),
            description: "Initialize isolate in current directory".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: true,
            custom: vec![],
        },
        side_effects: vec![
            "Creates .isolate directory".to_string(),
            "Creates config.toml".to_string(),
            "Creates state.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "ALREADY_INITIALIZED".to_string(),
            description: "Isolate already initialized".to_string(),
            resolution: "Remove .isolate directory to reinitialize".to_string(),
        }],
    }
}

/// Get introspection for the focus command
#[must_use]
pub fn get_focus_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "focus".to_string(),
        description: "Switch to a session's workspace".to_string(),
        aliases: vec!["switch".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to focus".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "scp focus my-session".to_string(),
            description: "Switch to my-session tab".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'scp list'".to_string(),
        }],
    }
}

/// Get introspection for the status command
#[must_use]
pub fn get_status_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "status".to_string(),
        description: "Show detailed session status".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: create_status_flags(),
        examples: vec![
            CommandExample {
                command: "scp status".to_string(),
                description: "Show status of all sessions".to_string(),
            },
            CommandExample {
                command: "scp status my-session".to_string(),
                description: "Show status of specific session".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Get introspection for the sync command
#[must_use]
pub fn get_sync_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "sync".to_string(),
        description: "Sync session workspace with main (rebase)".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (syncs current if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "scp sync my-session".to_string(),
            description: "Sync session with main branch".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        },
        side_effects: vec![
            "Rebases workspace onto main".to_string(),
            "Updates last_synced timestamp".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "CONFLICTS".to_string(),
            description: "Rebase resulted in conflicts".to_string(),
            resolution: "Resolve conflicts manually".to_string(),
        }],
    }
}

/// Get introspection for the diff command
#[must_use]
pub fn get_diff_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "diff".to_string(),
        description: "Show diff between session and main".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "stat".to_string(),
            short: None,
            description: "Show diffstat only".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "scp diff my-session".to_string(),
                description: "Show full diff".to_string(),
            },
            CommandExample {
                command: "scp diff my-session --stat".to_string(),
                description: "Show diffstat summary".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Get introspection for the introspect command itself
#[must_use]
pub fn get_introspect_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "introspect".to_string(),
        description: "Discover isolate capabilities".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Command to introspect (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["add".to_string(), "remove".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "scp introspect".to_string(),
                description: "Show all capabilities".to_string(),
            },
            CommandExample {
                command: "scp introspect add --json".to_string(),
                description: "Get add command schema as JSON".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

/// Get introspection for the doctor command
#[must_use]
pub fn get_doctor_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        aliases: vec!["check".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
            FlagSpec {
                long: "fix".to_string(),
                short: None,
                description: "Auto-fix issues where possible".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
        ],
        examples: vec![
            CommandExample {
                command: "scp doctor".to_string(),
                description: "Check system health".to_string(),
            },
            CommandExample {
                command: "scp doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            custom: vec![],
        },
        side_effects: vec!["May fix issues with --fix flag".to_string()],
        error_conditions: vec![],
    }
}

/// Get introspection for the query command
#[must_use]
pub fn get_query_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "query".to_string(),
        description: "Query system state".to_string(),
        aliases: vec![],
        arguments: vec![
            ArgumentSpec {
                name: "query_type".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Type of query".to_string(),
                validation: None,
                examples: vec![
                    "session-exists".to_string(),
                    "session-count".to_string(),
                    "can-run".to_string(),
                    "suggest-name".to_string(),
                ],
            },
            ArgumentSpec {
                name: "args".to_string(),
                arg_type: "string".to_string(),
                required: false,
                description: "Query-specific arguments".to_string(),
                validation: None,
                examples: vec!["my-session".to_string(), "feature-{n}".to_string()],
            },
        ],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(true)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "scp query session-exists my-session".to_string(),
                description: "Check if session exists".to_string(),
            },
            CommandExample {
                command: "scp query can-run add".to_string(),
                description: "Check if add command can run".to_string(),
            },
            CommandExample {
                command: "scp query suggest-name feature-{n}".to_string(),
                description: "Suggest next available name".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper function to create a boolean flag with common defaults
///
/// Creates a flag of type "bool" with default value of false.
/// Used for flags like --all, --json, --force, etc.
#[must_use]
pub fn create_bool_flag(long: &str, description: &str) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: None,
        description: description.to_string(),
        flag_type: "bool".to_string(),
        default: Some(serde_json::json!(false)),
        possible_values: vec![],
        category: None,
    }
}

/// Helper function to create a string filter flag
///
/// Creates a flag of type "string" for filtering operations.
/// These flags support dynamic values and pattern matching.
#[must_use]
pub fn create_string_filter_flag(long: &str, short: &str, description: &str) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: Some(short.to_string()),
        description: description.to_string(),
        flag_type: "string".to_string(),
        default: None,
        possible_values: vec![],
        category: None,
    }
}

/// Helper function to create an enum flag with predefined values
///
/// Creates a flag with specific allowed values and a default.
#[must_use]
pub fn create_enum_flag(
    long: &str,
    short: Option<&str>,
    description: &str,
    possible_values: Vec<String>,
    default_value: &str,
) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: short.map(ToString::to_string),
        description: description.to_string(),
        flag_type: "enum".to_string(),
        default: Some(serde_json::json!(default_value)),
        possible_values,
        category: None,
    }
}

/// Helper function to create an error condition with comprehensive context
///
/// Uses Railway-Oriented Programming to ensure consistent error documentation
/// across all commands.
#[must_use]
pub fn create_error_condition(code: &str, description: &str, resolution: &str) -> ErrorCondition {
    ErrorCondition {
        code: code.to_string(),
        description: description.to_string(),
        resolution: resolution.to_string(),
    }
}

/// Helper function to create a command example with description
#[must_use]
pub fn create_example(command: &str, description: &str) -> CommandExample {
    CommandExample {
        command: command.to_string(),
        description: description.to_string(),
    }
}

// ============================================================================
// AI-Focused Introspection Types
// ============================================================================

/// Environment variable information
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvVarInfo {
    /// Variable name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Read, write, or both
    pub direction: String,
    /// Default value if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Example usage
    pub example: String,
}

/// Output for --env-vars mode
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvVarsOutput {
    /// List of environment variables
    pub env_vars: Vec<EnvVarInfo>,
}

/// Workflow step
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowStep {
    /// Step number
    pub step: usize,
    /// Command to execute
    pub command: String,
    /// Description of the step
    pub description: String,
}

/// Workflow pattern
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowPattern {
    /// Workflow name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Steps in the workflow
    pub steps: Vec<WorkflowStep>,
}

/// Output for --workflows mode
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowsOutput {
    /// Available workflows
    pub workflows: Vec<WorkflowPattern>,
}

/// Session state transition
#[derive(Debug, Clone, serde::Serialize)]
pub struct StateTransition {
    /// Source state
    pub from: String,
    /// Target state
    pub to: String,
    /// What triggers this transition
    pub trigger: String,
}

/// Output for --session-states mode
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionStatesOutput {
    /// All valid states
    pub states: Vec<String>,
    /// Valid transitions
    pub transitions: Vec<StateTransition>,
}

/// Format flags grouped by category with deterministic ordering
///
/// Categories are displayed in the following order:
/// 1. Behavior
/// 2. Configuration
/// 3. Filter
/// 4. Output
/// 5. Advanced
/// 6. General (for uncategorized flags)
///
/// # Returns
///
/// Returns a formatted string with flags grouped by category.
#[must_use]
pub fn format_flags_by_category(flags: &[FlagSpec]) -> String {
    use std::{collections::BTreeMap, fmt::Write};

    let mut output = String::from("Flags:");

    // Group flags by category using functional iterator patterns
    // Map None to "general" for uncategorized flags
    let grouped = flags.iter().fold(
        BTreeMap::new(),
        |mut acc: BTreeMap<String, Vec<&FlagSpec>>, flag| {
            let category = flag.category.as_deref().unwrap_or("general").to_string();
            acc.entry(category).or_default().push(flag);
            acc
        },
    );

    // Define category display order (deterministic, consistent across runs)
    let category_order = [
        "behavior",
        "configuration",
        "filter",
        "output",
        "advanced",
        "general",
    ];

    // Display categories in defined order using functional patterns
    for category in &category_order {
        if let Some(flags_in_category) = grouped.get(*category) {
            let _ = write!(output, "\n\n  {}:", capitalize_category(category));

            for flag in flags_in_category {
                let short = flag
                    .short
                    .as_ref()
                    .map(|s| format!("-{s}, "))
                    .map_or(String::new(), |value| value);
                let _ = write!(output, "\n    {short}--{}", flag.long);
                let _ = write!(output, "\n      Type: {}", flag.flag_type);
                let _ = write!(output, "\n      Description: {}", flag.description);

                if let Some(ref default) = flag.default {
                    let _ = write!(output, "\n      Default: {default}");
                }

                if !flag.possible_values.is_empty() {
                    let _ = write!(
                        output,
                        "\n      Values: {}",
                        flag.possible_values.join(", ")
                    );
                }
            }
        }
    }

    output.push('\n');
    output
}

/// Capitalize category name for display
///
/// Converts category strings like "behavior" or "multi-word" to
/// "Behavior" or "Multi Word" using functional transformations.
#[must_use]
pub fn capitalize_category(category: &str) -> String {
    category
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .map_or(String::new(), |value| value)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print command introspection in human-readable format
pub fn print_command_human_readable(cmd: &CommandIntrospection) {
    println!("Command: {}", cmd.command);
    println!("Description: {}", cmd.description);
    println!();

    if !cmd.arguments.is_empty() {
        println!("Arguments:");
        cmd.arguments.iter().for_each(|arg| {
            let required = if arg.required {
                " (required)"
            } else {
                " (optional)"
            };
            println!("  {}{required}", arg.name);
            println!("    Type: {}", arg.arg_type);
            println!("    Description: {}", arg.description);
            if !arg.examples.is_empty() {
                println!("    Examples: {}", arg.examples.join(", "));
            }
        });
        println!();
    }

    if !cmd.flags.is_empty() {
        print!("{}", format_flags_by_category(&cmd.flags));
        println!();
    }

    if !cmd.examples.is_empty() {
        println!("Examples:");
        cmd.examples.iter().for_each(|example| {
            println!("  {}", example.command);
            println!("    {}", example.description);
        });
        println!();
    }

    println!("Prerequisites:");
    println!("  Initialized: {}", cmd.prerequisites.initialized);
    println!("  JJ Installed: {}", cmd.prerequisites.jj_installed);
}

/// Introspect a specific command
///
/// # Errors
///
/// Returns an error if the command name is not recognized.
pub fn run_command_introspect(command: &str, format: OutputFormat) -> Result<()> {
    let introspection = match command {
        "add" => get_add_introspection(),
        "remove" => get_remove_introspection(),
        "list" => get_list_introspection(),
        "init" => get_init_introspection(),
        "focus" => get_focus_introspection(),
        "status" => get_status_introspection(),
        "sync" => get_sync_introspection(),
        "diff" => get_diff_introspection(),
        "introspect" => get_introspect_introspection(),
        "doctor" => get_doctor_introspection(),
        "query" => get_query_introspection(),
        _ => {
            return Err(IsolateError::OperationFailed(format!(
                "Unknown command: {command}"
            )));
        }
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-command-response", "single", introspection);
        println!("{}", serde_json::to_string_pretty(&envelope).map_err(|e| {
            IsolateError::OperationFailed(format!("Failed to serialize introspection: {e}"))
        })?);
    } else {
        print_command_human_readable(&introspection);
    }

    Ok(())
}

/// Run introspect with --env-vars flag
pub fn run_env_vars(format: OutputFormat) -> Result<()> {
    let env_vars = vec![
        EnvVarInfo {
            name: "ISOLATE_AGENT_ID".to_string(),
            description: "Current agent ID for tracking".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "agent-12345678-abcd".to_string(),
        },
        EnvVarInfo {
            name: "ISOLATE_SESSION".to_string(),
            description: "Current session name".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "feature-auth".to_string(),
        },
        EnvVarInfo {
            name: "ISOLATE_WORKSPACE".to_string(),
            description: "Path to current workspace directory".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "/path/to/.isolate/workspaces/feature-auth".to_string(),
        },
        EnvVarInfo {
            name: "ISOLATE_BEAD_ID".to_string(),
            description: "Bead ID associated with current work".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "isolate-abc12".to_string(),
        },
        EnvVarInfo {
            name: "ISOLATE_ACTIVE".to_string(),
            description: "Set to 1 when in an active isolate workspace".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "1".to_string(),
        },
        EnvVarInfo {
            name: "ISOLATE_RECOVERY_POLICY".to_string(),
            description: "Database recovery behavior: silent, warn, fail-fast".to_string(),
            direction: "read".to_string(),
            default: Some("warn".to_string()),
            example: "fail-fast".to_string(),
        },
    ];

    let output = EnvVarsOutput { env_vars };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-env-vars-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| {
                IsolateError::OperationFailed(format!("Failed to serialize env vars: {e}"))
            })?
        );
    } else {
        println!("Environment Variables:\n");
        output.env_vars.iter().for_each(|var| {
            println!("  {} ({}):", var.name, var.direction);
            println!("    {}", var.description);
            if let Some(ref default) = var.default {
                println!("    Default: {default}");
            }
            println!("    Example: {}", var.example);
            println!();
        });
    }

    Ok(())
}

/// Run introspect with --workflows flag
#[allow(clippy::too_many_lines)]
pub fn run_workflows(format: OutputFormat) -> Result<()> {
    let workflows = vec![
        WorkflowPattern {
            name: "Quick Work Session".to_string(),
            description: "Start working on a task, do work, complete".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "scp work my-task --idempotent".to_string(),
                    description: "Create workspace (idempotent for retries)".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "cd $(scp context --field location.path)".to_string(),
                    description: "Enter workspace directory".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "# ... do work ...".to_string(),
                    description: "Implement changes".to_string(),
                },
                WorkflowStep {
                    step: 4,
                    command: "scp done".to_string(),
                    description: "Merge and cleanup".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Agent-Managed Workflow".to_string(),
            description: "Full agent lifecycle with registration".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "scp agent register".to_string(),
                    description: "Register as an agent".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "scp work my-task --bead isolate-abc12".to_string(),
                    description: "Create workspace for bead".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "scp agent heartbeat --command \"implementing\"".to_string(),
                    description: "Send heartbeat while working".to_string(),
                },
                WorkflowStep {
                    step: 4,
                    command: "scp done".to_string(),
                    description: "Complete work and merge".to_string(),
                },
                WorkflowStep {
                    step: 5,
                    command: "scp agent unregister".to_string(),
                    description: "Deregister agent".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Quick Orientation".to_string(),
            description: "Quickly understand current state".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "scp whereami".to_string(),
                    description: "Check location: main or workspace".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "scp whoami".to_string(),
                    description: "Check agent identity".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "scp query can-spawn".to_string(),
                    description: "Check if spawning is possible".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Abandon Work".to_string(),
            description: "Discard work without merging".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "scp abort --dry-run".to_string(),
                    description: "Preview what will be aborted".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "scp abort".to_string(),
                    description: "Abort and cleanup".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Sync All Workspaces".to_string(),
            description: "Keep all workspaces up to date".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "scp sync --all".to_string(),
                description: "Sync all active sessions with main".to_string(),
            }],
        },
    ];

    let output = WorkflowsOutput { workflows };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-workflows-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| {
                IsolateError::OperationFailed(format!("Failed to serialize workflows: {e}"))
            })?
        );
    } else {
        println!("Workflow Patterns:\n");
        output.workflows.iter().for_each(|workflow| {
            println!("  {}:", workflow.name);
            println!("    {}\n", workflow.description);
            workflow.steps.iter().for_each(|step| {
                println!("    {}. {}", step.step, step.command);
                println!("       {}", step.description);
            });
            println!();
        });
    }

    Ok(())
}

/// Run introspect with --session-states flag
pub fn run_session_states(format: OutputFormat) -> Result<()> {
    let states = vec![
        "creating".to_string(),
        "active".to_string(),
        "syncing".to_string(),
        "merging".to_string(),
        "completed".to_string(),
        "failed".to_string(),
    ];

    let transitions = vec![
        StateTransition {
            from: "none".to_string(),
            to: "creating".to_string(),
            trigger: "scp add / scp work".to_string(),
        },
        StateTransition {
            from: "creating".to_string(),
            to: "active".to_string(),
            trigger: "workspace created successfully".to_string(),
        },
        StateTransition {
            from: "creating".to_string(),
            to: "failed".to_string(),
            trigger: "workspace creation failed".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "syncing".to_string(),
            trigger: "scp sync".to_string(),
        },
        StateTransition {
            from: "syncing".to_string(),
            to: "active".to_string(),
            trigger: "sync completed".to_string(),
        },
        StateTransition {
            from: "syncing".to_string(),
            to: "failed".to_string(),
            trigger: "sync failed (conflicts)".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "merging".to_string(),
            trigger: "scp done".to_string(),
        },
        StateTransition {
            from: "merging".to_string(),
            to: "completed".to_string(),
            trigger: "merge successful".to_string(),
        },
        StateTransition {
            from: "merging".to_string(),
            to: "failed".to_string(),
            trigger: "merge failed".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "failed".to_string(),
            trigger: "scp abort".to_string(),
        },
    ];

    let output = SessionStatesOutput { states, transitions };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-session-states-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| {
                IsolateError::OperationFailed(format!("Failed to serialize session states: {e}"))
            })?
        );
    } else {
        println!("Session States: {}\n", output.states.join(" -> "));
        println!("Transitions:");
        output.transitions.iter().for_each(|t| {
            println!("  {} -> {} : {}", t.from, t.to, t.trigger);
        });
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bool_flag() {
        let flag = create_bool_flag("all", "Include everything");
        assert_eq!(flag.long, "all");
        assert_eq!(flag.flag_type, "bool");
        assert!(flag.default.is_some());
        assert_eq!(flag.default.as_ref().unwrap(), &serde_json::json!(false));
    }

    #[test]
    fn test_create_string_filter_flag() {
        let flag = create_string_filter_flag("bead", "b", "Filter by bead");
        assert_eq!(flag.long, "bead");
        assert_eq!(flag.short, Some("b".to_string()));
        assert_eq!(flag.flag_type, "string");
        assert!(flag.default.is_none());
    }

    #[test]
    fn test_create_error_condition() {
        let err = create_error_condition("TEST", "Test error", "Try again");
        assert_eq!(err.code, "TEST");
        assert_eq!(err.description, "Test error");
        assert_eq!(err.resolution, "Try again");
    }

    #[test]
    fn test_create_example() {
        let ex = create_example("scp add foo", "Add session foo");
        assert_eq!(ex.command, "scp add foo");
        assert_eq!(ex.description, "Add session foo");
    }

    #[test]
    fn test_capitalize_category() {
        assert_eq!(capitalize_category("behavior"), "Behavior");
        assert_eq!(capitalize_category("multi-word"), "Multi Word");
        assert_eq!(capitalize_category("advanced"), "Advanced");
    }

    #[test]
    fn test_get_add_introspection() {
        let intro = get_add_introspection();
        assert_eq!(intro.command, "add");
        assert!(!intro.arguments.is_empty());
        assert!(!intro.flags.is_empty());
        assert!(!intro.examples.is_empty());
    }

    #[test]
    fn test_get_list_introspection() {
        let intro = get_list_introspection();
        assert_eq!(intro.command, "list");
        assert!(intro.arguments.is_empty());
        assert!(!intro.flags.is_empty());
        assert!(intro.prerequisites.jj_installed == false);
    }

    #[test]
    fn test_get_introspect_introspection() {
        let intro = get_introspect_introspection();
        assert_eq!(intro.command, "introspect");
        assert!(intro.prerequisites.initialized == false);
        assert!(intro.prerequisites.jj_installed == false);
    }

    #[test]
    fn test_run_command_introspect_unknown() {
        let result = run_command_introspect("unknown-cmd", OutputFormat::json());
        assert!(result.is_err());
    }

    #[test]
    fn test_env_vars_output_serialization() {
        let env_vars = vec![EnvVarInfo {
            name: "TEST_VAR".to_string(),
            description: "A test variable".to_string(),
            direction: "read".to_string(),
            default: Some("default".to_string()),
            example: "test-value".to_string(),
        }];
        let output = EnvVarsOutput { env_vars };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("TEST_VAR"));
        assert!(json.contains("A test variable"));
    }

    #[test]
    fn test_workflows_output_serialization() {
        let workflows = vec![WorkflowPattern {
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "scp test".to_string(),
                description: "Run test".to_string(),
            }],
        }];
        let output = WorkflowsOutput { workflows };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("Test Workflow"));
        assert!(json.contains("scp test"));
    }

    #[test]
    fn test_session_states_output_serialization() {
        let output = SessionStatesOutput {
            states: vec!["active".to_string(), "completed".to_string()],
            transitions: vec![StateTransition {
                from: "active".to_string(),
                to: "completed".to_string(),
                trigger: "done".to_string(),
            }],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("active"));
        assert!(json.contains("completed"));
        assert!(json.contains("done"));
    }

    #[test]
    fn test_format_flags_by_category_empty() {
        let flags: Vec<FlagSpec> = vec![];
        let output = format_flags_by_category(&flags);
        assert!(output.contains("Flags:"));
    }

    #[test]
    fn test_format_flags_by_category_with_flags() {
        let flags = vec![
            create_bool_flag("all", "Include all"),
            create_string_filter_flag("bead", "b", "Filter by bead"),
        ];
        let output = format_flags_by_category(&flags);
        assert!(output.contains("--all"));
        assert!(output.contains("--bead"));
        assert!(output.contains("Type: bool"));
        assert!(output.contains("Type: string"));
    }
}
