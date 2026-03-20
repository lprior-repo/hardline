#![allow(clippy::redundant_closure_for_method_calls)]
//! Object-based CLI command type system
//!
//! This module defines the new object-based command structure following
//! the pattern: `isolate <object> <action>`
//!
//! Objects are nouns (Task, Session, Agent, etc.) and actions are verbs.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use clap::{Arg, Command as ClapCommand};

/// Top-level objects in the isolate CLI
///
/// Each object represents a domain of related operations following
/// the `isolate <object> <action>` pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZjjObject {
    /// Task management (beads, work items)
    Task,
    /// Session management (workspaces)
    Session,
    /// Status and introspection queries
    Status,
    /// Configuration management
    Config,
    /// Diagnostics and health checks
    Doctor,
}

impl ZjjObject {
    /// Returns all object variants
    pub const fn all() -> &'static [Self] {
        &[
            Self::Task,
            Self::Session,
            Self::Status,
            Self::Config,
            Self::Doctor,
        ]
    }

    /// Returns the CLI name for this object
    pub const fn name(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Session => "session",
            Self::Status => "status",
            Self::Config => "config",
            Self::Doctor => "doctor",
        }
    }

    /// Returns a short description for this object
    pub const fn about(self) -> &'static str {
        match self {
            Self::Task => "Manage tasks and work items (beads)",
            Self::Session => "Manage workspaces and sessions",
            Self::Status => "Query system and session status",
            Self::Config => "Manage isolate configuration",
            Self::Doctor => "Run diagnostics and health checks",
        }
    }
}

/// Subcommands for the Task object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAction {
    /// List all tasks
    List,
    /// Show task details
    Show,
    /// Claim a task for work
    Claim,
    /// Yield a claimed task
    Yield,
    /// Start work on a task (creates session)
    Start,
    /// Complete a task
    Done,
}

/// Subcommands for the Session object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAction {
    /// List all sessions
    List,
    /// Create a new session
    Add,
    /// Remove a session
    Remove,
    /// Switch to a session
    Focus,
    /// Pause a session
    Pause,
    /// Resume a session
    Resume,
    /// Clone a session
    Clone,
    /// Rename a session
    Rename,
    /// Attach to session from shell
    Attach,
    /// Spawn session for agent work
    Spawn,
    /// Sync session with remote
    Sync,
    /// Initialize isolate in repository
    Init,
}

/// Subcommands for the Status object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusAction {
    /// Show current status
    Show,
    /// Show where you are
    Whereami,
    /// Show who you are
    Whoami,
    /// Show context information
    Context,
}

/// Subcommands for the Config object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    /// List configuration
    List,
    /// Get a config value
    Get,
    /// Set a config value
    Set,
    /// Show configuration schema
    Schema,
}

/// Subcommands for the Doctor object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorAction {
    /// Run diagnostics
    Check,
    /// Fix issues
    Fix,
    /// Show system integrity
    Integrity,
    /// Clean up invalid sessions
    Clean,
}

/// Global flags available on all commands
#[derive(Debug, Clone, Default)]
pub struct GlobalFlags {
    /// Output as JSON
    pub json: bool,
    /// Verbose output
    pub verbose: bool,
    /// Dry run (preview without executing)
    pub dry_run: bool,
}

// ============================================================================
// Command Builders
// ============================================================================

/// Create the JSON argument (common to all commands)
fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Output as JSON (machine-parseable format)")
}

/// Create the verbose argument
fn verbose_arg() -> Arg {
    Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Enable verbose output")
}

/// Create the dry-run argument
fn dry_run_arg() -> Arg {
    Arg::new("dry-run")
        .long("dry-run")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Preview without executing")
}

/// Create the contract argument (AI: Show machine-readable contract)
fn contract_arg() -> Arg {
    Arg::new("contract")
        .long("contract")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)")
}

/// Create the ai-hints argument (AI: Show execution hints)
fn ai_hints_arg() -> Arg {
    Arg::new("ai-hints")
        .long("ai-hints")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show execution hints and common patterns")
}

/// Build the Task object command with all subcommands
pub fn cmd_task() -> ClapCommand {
    ClapCommand::new("task")
        .about("Manage tasks and work items (beads)")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all tasks")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include completed tasks"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by task state"),
                ),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show task details")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to show")),
        )
        .subcommand(
            ClapCommand::new("start")
                .about("Start work on a task (creates session)")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to start")),
        )
        .subcommand(
            ClapCommand::new("done")
                .visible_alias("complete")
                .about("Complete a task")
                .arg(json_arg())
                .arg(Arg::new("id").help("Task/bead ID (uses current session if omitted)")),
        )
}

/// Build the Session object command with all subcommands
#[allow(clippy::too_many_lines)]
pub fn cmd_session() -> ClapCommand {
    ClapCommand::new("session")
        .about("Manage workspaces and sessions")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include closed sessions"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed information"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .value_name("BEAD_ID")
                        .help("Filter by bead ID"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Filter by agent owner"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by session state"),
                ),
        )
        .subcommand(
            ClapCommand::new("add")
                .visible_alias("create")
                .about("Create a new session for manual work")
                .arg(json_arg())
                .arg(dry_run_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .help("Succeed if session already exists (no-op)"),
                )
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new session"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .short('b')
                        .value_name("BEAD_ID")
                        .help("Associate with a bead ID"),
                )
                .arg(
                    Arg::new("no-open")
                        .long("no-open")
                        .action(clap::ArgAction::SetTrue)
                        .help("Create without opening terminal"),
                )
                .arg(
                    Arg::new("no-hooks")
                        .long("no-hooks")
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip post-create hooks"),
                ),
        )
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove a session")
                .arg(json_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .help("Succeed if session doesn't exist (no-op)"),
                )
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to remove"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force removal without confirmation"),
                ),
        )
        .subcommand(
            ClapCommand::new("pause")
                .about("Pause a session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("resume")
                .about("Resume a paused session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("clone")
                .about("Clone a session")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to clone"),
                )
                .arg(
                    Arg::new("new-name")
                        .long("new-name")
                        .value_name("NAME")
                        .help("Name for cloned session"),
                )
                .arg(dry_run_arg()),
        )
        .subcommand(
            ClapCommand::new("rename")
                .about("Rename a session")
                .arg(json_arg())
                .arg(
                    Arg::new("old-name")
                        .required(true)
                        .help("Current session name"),
                )
                .arg(Arg::new("new-name").required(true).help("New session name")),
        )
        .subcommand(
            ClapCommand::new("spawn")
                .about("Spawn session for automated agent work")
                .arg(json_arg())
                .arg(dry_run_arg())
                .arg(
                    Arg::new("idempotent")
                        .long("idempotent")
                        .action(clap::ArgAction::SetTrue)
                        .default_value("false")
                        .help("Succeed if session already exists (no-op)"),
                )
                .arg(
                    Arg::new("bead")
                        .required(true)
                        .help("Bead ID for the spawned session"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Agent to assign"),
                ),
        )
        .subcommand(
            ClapCommand::new("sync")
                .visible_alias("rebase")
                .about("Sync session with remote")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)"))
                .arg(
                    Arg::new("push")
                        .long("push")
                        .action(clap::ArgAction::SetTrue)
                        .help("Push changes to remote"),
                )
                .arg(
                    Arg::new("pull")
                        .long("pull")
                        .action(clap::ArgAction::SetTrue)
                        .help("Pull changes from remote"),
                ),
        )
        .subcommand(
            ClapCommand::new("init")
                .about("Initialize isolate in a JJ repository")
                .arg(json_arg())
                .arg(dry_run_arg()),
        )
}

/// Build the Status object command with all subcommands
pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Query system and session status")
        .subcommand_required(false)
        .arg(json_arg())
        .arg(contract_arg())
        .arg(ai_hints_arg())
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show current status")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("whereami")
                .about("Show current location")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("whoami")
                .about("Show current identity")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("context")
                .about("Show context information")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("field").help("Specific field to display"))
                .arg(
                    Arg::new("no-beads")
                        .long("no-beads")
                        .action(clap::ArgAction::SetTrue)
                        .help("Don't show beads in context"),
                )
                .arg(
                    Arg::new("no-health")
                        .long("no-health")
                        .action(clap::ArgAction::SetTrue)
                        .help("Don't show health checks in context"),
                )
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
}

/// Build the Config object command with all subcommands
pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("Manage isolate configuration")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List configuration values")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show global config instead of project"),
                ),
        )
        .subcommand(
            ClapCommand::new("get")
                .about("Get a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Get from global config"),
                )
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to get"),
                ),
        )
        .subcommand(
            ClapCommand::new("set")
                .about("Set a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("global")
                        .long("global")
                        .short('g')
                        .action(clap::ArgAction::SetTrue)
                        .help("Set in global config"),
                )
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to set"),
                )
                .arg(Arg::new("value").required(true).help("Value to set")),
        )
        .subcommand(
            ClapCommand::new("schema")
                .about("Show configuration schema")
                .arg(json_arg()),
        )
}

/// Build the Doctor object command with all subcommands
pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run diagnostics and health checks")
        .subcommand_required(false)
        .arg(json_arg())
        // Legacy flags for backward compatibility
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible (legacy mode)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview what would be fixed without making changes"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(clap::ArgAction::SetTrue)
                .help("Show detailed progress during fixes"),
        )
        .subcommand(
            ClapCommand::new("check")
                .about("Run diagnostics")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("fix")
                .about("Fix detected issues")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed progress during fixes"),
                ),
        )
        .subcommand(
            ClapCommand::new("integrity")
                .about("Check system integrity")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("clean")
                .about("Clean up invalid sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force cleanup without confirmation"),
                ),
        )
}

/// Build the complete object-based CLI
///
/// This creates the new `isolate <object> <action>` command structure
/// while maintaining compatibility with existing handlers.
#[allow(clippy::too_many_lines)]
pub fn build_object_cli() -> ClapCommand {
    ClapCommand::new("isolate")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Isolate Contributors")
        .about("Isolate - Isolated workspace manager (object-based CLI)")
        .long_about(
            "Isolate creates isolated JJ workspaces.\n\n\
             Object-based command structure:\n\
             \n\
  isolate task <action>     Manage tasks and work items\n\
             \n\
  isolate session <action>  Manage workspaces and sessions\n\
             \n\
  isolate status <action>   Query system status\n\
             \n\
  isolate config <action>   Manage configuration\n\
             \n\
  isolate doctor <action>   Run diagnostics\n",
        )
        .subcommand_required(true)
        .arg(json_arg().global(true))
        .arg(verbose_arg().global(true))
        .arg(
            Arg::new("on-success")
                .long("on-success")
                .global(true)
                .value_name("CMD")
                .help("Command to run after successful execution"),
        )
        .arg(
            Arg::new("on-failure")
                .long("on-failure")
                .global(true)
                .value_name("CMD")
                .help("Command to run after failed execution"),
        )
        .arg(
            Arg::new("command-id")
                .long("command-id")
                .global(true)
                .hide(true)
                .value_name("ID")
                .help("Override idempotency command id base for retries"),
        )
        .subcommand(cmd_task())
        .subcommand(cmd_session())
        .subcommand(cmd_status())
        .subcommand(cmd_config())
        .subcommand(cmd_doctor())
        // Legacy commands - route to same handlers
        .subcommand(
            ClapCommand::new("init")
                .about("Initialize isolate")
                .arg(dry_run_arg())
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("add")
                .about("Add session")
                .arg(Arg::new("name").required_unless_present("example-json"))
                .arg(dry_run_arg())
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("bead").long("bead").short('b').value_name("BEAD_ID"))
                .arg(Arg::new("no-open").long("no-open").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("no-hooks").long("no-hooks").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("idempotent").long("idempotent").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("example-json").long("example-json").action(clap::ArgAction::SetTrue).default_value("false")),
        )
        .subcommand(
            ClapCommand::new("list")
                .about("List sessions")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("all").long("all").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("verbose").short('v').long("verbose").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("bead").long("bead").value_name("BEAD_ID"))
                .arg(Arg::new("agent").long("agent").value_name("AGENT"))
                .arg(Arg::new("state").long("state").value_name("STATE")),
        )
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove session")
                .arg(Arg::new("name").required(true))
                .arg(Arg::new("force").short('f').long("force").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("merge").short('m').long("merge").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("keep-branch").short('k').long("keep-branch").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("idempotent").long("idempotent").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("spawn")
                .about("Spawn session")
                .arg(Arg::new("bead").required(true))
                .arg(dry_run_arg())
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("idempotent").long("idempotent").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("agent").long("agent").value_name("AGENT"))
                .arg(Arg::new("no-auto-merge").long("no-auto-merge").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("no-auto-cleanup").long("no-auto-cleanup").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("background").short('b').long("background").action(clap::ArgAction::SetTrue).default_value("false"))
                .arg(Arg::new("timeout").long("timeout").value_name("SECONDS").default_value("14400"))
                .arg(Arg::new("agent-command").long("agent-command").value_name("COMMAND").default_value("claude"))
                .arg(Arg::new("agent-args").long("agent-args").num_args(0..)),
        )
        .subcommand(
            ClapCommand::new("sync")
                .about("Sync session")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("clone")
                .about("Clone session")
                .arg(Arg::new("source").required(true))
                .arg(Arg::new("dest").required(true))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("rename")
                .about("Rename session")
                .arg(Arg::new("old_name").required(true))
                .arg(Arg::new("new_name").required(true))
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("pause")
                .about("Pause session")
                .arg(Arg::new("name").required(false))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("resume")
                .about("Resume session")
                .arg(Arg::new("name").required(false))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("whoami")
                .about("Who am I")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("whereami")
                .about("Where am I")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("context")
                .about("Show context")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("field").long("field").value_name("PATH").help("Extract single field (e.g., --field=repository.branch)"))
                .arg(Arg::new("no-beads").long("no-beads").action(clap::ArgAction::SetTrue).help("Skip beads database query (faster)"))
                .arg(Arg::new("no-health").long("no-health").action(clap::ArgAction::SetTrue).help("Skip health checks (faster)")),
        )
        .subcommand(
            ClapCommand::new("done")
                .about("Done (complete work)")
                .visible_alias("submit")
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg())
                .arg(Arg::new("name").required(false))
                .arg(Arg::new("workspace").short('w').long("workspace").value_name("NAME"))
                .arg(Arg::new("message").short('m').long("message").value_name("MSG"))
                .arg(Arg::new("keep-workspace").long("keep-workspace").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("no-keep").long("no-keep").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("squash").long("squash").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("detect-conflicts").long("detect-conflicts").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("no-bead-update").long("no-bead-update").action(clap::ArgAction::SetTrue)),
        )
        .subcommand(
            ClapCommand::new("work")
                .about("Start work on a task")
                .arg(Arg::new("bead").required(false))
                .arg(Arg::new("name").required(false))
                .arg(Arg::new("agent-id").long("agent-id").value_name("ID"))
                .arg(Arg::new("no-agent").long("no-agent").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("idempotent").long("idempotent").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("abort")
                .about("Abort work")
                .arg(Arg::new("name").required(false))
                .arg(Arg::new("workspace").short('w').long("workspace").value_name("NAME"))
                .arg(Arg::new("force").short('f').long("force").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("no-bead-update").long("no-bead-update").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("keep-workspace").long("keep-workspace").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(json_arg())
                .arg(contract_arg())
                .arg(ai_hints_arg()),
        )
        .subcommand(
            ClapCommand::new("checkpoint")
                .about("Create checkpoint")
                .visible_alias("ckpt")
                .arg(Arg::new("name").required(false))
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("undo")
                .about("Undo last operation")
                .arg(Arg::new("list").short('l').long("list").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("revert")
                .about("Revert changes")
                .arg(Arg::new("name").required(true))
                .arg(Arg::new("dry-run").long("dry-run").action(clap::ArgAction::SetTrue))
                .arg(json_arg()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolate_object_names() {
        assert_eq!(ZjjObject::Task.name(), "task");
        assert_eq!(ZjjObject::Session.name(), "session");
        assert_eq!(ZjjObject::Status.name(), "status");
        assert_eq!(ZjjObject::Config.name(), "config");
        assert_eq!(ZjjObject::Doctor.name(), "doctor");
    }

    #[test]
    fn test_isolate_object_all_count() {
        assert_eq!(ZjjObject::all().len(), 5);
    }

    #[test]
    fn test_build_object_cli_has_all_subcommands() {
        let cli = build_object_cli();
        let subcommands: Vec<&str> = cli.get_subcommands().map(clap::Command::get_name).collect();

        assert!(subcommands.contains(&"task"));
        assert!(subcommands.contains(&"session"));
        assert!(subcommands.contains(&"status"));
        assert!(subcommands.contains(&"config"));
        assert!(subcommands.contains(&"doctor"));
    }

    #[test]
    fn test_all_commands_have_json_flag() {
        let cli = build_object_cli();

        for object_cmd in cli.get_subcommands() {
            // Check object-level has json flag
            let has_json = object_cmd
                .get_arguments()
                .any(|arg| arg.get_id().as_str() == "json");
            assert!(
                has_json,
                "Object {} should have --json flag",
                object_cmd.get_name()
            );

            // Check all subcommands have json flag
            for action_cmd in object_cmd.get_subcommands() {
                let action_has_json = action_cmd
                    .get_arguments()
                    .any(|arg| arg.get_id().as_str() == "json");
                assert!(
                    action_has_json,
                    "Action {} {} should have --json flag",
                    object_cmd.get_name(),
                    action_cmd.get_name()
                );
            }
        }
    }

    #[test]
    fn test_task_subcommands() {
        let cmd = cmd_task();
        let subcommands: Vec<&str> = cmd.get_subcommands().map(clap::Command::get_name).collect();

        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"show"));
        assert!(subcommands.contains(&"start"));
        assert!(subcommands.contains(&"done"));
    }

    #[test]
    fn test_session_subcommands() {
        let cmd = cmd_session();
        let subcommands: Vec<&str> = cmd.get_subcommands().map(clap::Command::get_name).collect();

        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"add"));
        assert!(subcommands.contains(&"remove"));
        assert!(subcommands.contains(&"pause"));
        assert!(subcommands.contains(&"resume"));
        assert!(subcommands.contains(&"clone"));
        assert!(subcommands.contains(&"rename"));
        assert!(subcommands.contains(&"spawn"));
        assert!(subcommands.contains(&"sync"));
        assert!(subcommands.contains(&"init"));
    }
}
