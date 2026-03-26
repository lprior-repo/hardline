#![allow(clippy::redundant_closure_for_method_calls)]
//! Object-based CLI type definitions
//!
//! This module defines the type system for the object-based command structure
//! following the pattern: `isolate <object> <action>`

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

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
