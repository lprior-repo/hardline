#![allow(clippy::redundant_closure_for_method_calls)]
//! Object-based CLI type definitions
//!
//! This module defines the type system for the object-based command structure
//! following the pattern: `hardline <object> <action>`

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

/// Top-level objects in the hardline CLI
///
/// Each object represents a domain of related operations following
/// the `hardline <object> <action>` pattern.
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
            Self::Config => "Manage hardline configuration",
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
    /// Initialize hardline in repository
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ZjjObject ----

    #[test]
    fn zjj_object_all_returns_five_variants() {
        let all = ZjjObject::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn zjj_object_all_contains_each_variant() {
        let all = ZjjObject::all();
        assert!(all.contains(&ZjjObject::Task));
        assert!(all.contains(&ZjjObject::Session));
        assert!(all.contains(&ZjjObject::Status));
        assert!(all.contains(&ZjjObject::Config));
        assert!(all.contains(&ZjjObject::Doctor));
    }

    #[test]
    fn zjj_object_name_returns_lowercase_identifier() {
        assert_eq!(ZjjObject::Task.name(), "task");
        assert_eq!(ZjjObject::Session.name(), "session");
        assert_eq!(ZjjObject::Status.name(), "status");
        assert_eq!(ZjjObject::Config.name(), "config");
        assert_eq!(ZjjObject::Doctor.name(), "doctor");
    }

    #[test]
    fn zjj_object_about_returns_non_empty_string() {
        for obj in ZjjObject::all() {
            assert!(
                !obj.about().is_empty(),
                "about() should be non-empty for {:?}",
                obj
            );
        }
    }

    #[test]
    fn zjj_object_names_are_unique() {
        let names: Vec<&str> = ZjjObject::all().iter().map(|o| o.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            names.len(),
            sorted.len(),
            "ZjjObject names should be unique"
        );
    }

    #[test]
    fn zjj_object_equality() {
        assert_eq!(ZjjObject::Task, ZjjObject::Task);
        assert_ne!(ZjjObject::Task, ZjjObject::Session);
    }

    #[test]
    fn zjj_object_clone() {
        let obj = ZjjObject::Config;
        let cloned = obj.clone();
        assert_eq!(obj, cloned);
    }

    #[test]
    fn zjj_object_debug_format() {
        let debug = format!("{:?}", ZjjObject::Task);
        assert!(debug.contains("Task"));
    }

    // ---- TaskAction ----

    #[test]
    fn task_action_equality() {
        assert_eq!(TaskAction::List, TaskAction::List);
        assert_eq!(TaskAction::Claim, TaskAction::Claim);
        assert_ne!(TaskAction::List, TaskAction::Show);
    }

    #[test]
    fn task_action_clone() {
        let action = TaskAction::Start;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn task_action_debug() {
        let debug = format!("{:?}", TaskAction::Done);
        assert!(debug.contains("Done"));
    }

    // ---- SessionAction ----

    #[test]
    fn session_action_variants_are_distinct() {
        let actions = [
            SessionAction::List,
            SessionAction::Add,
            SessionAction::Remove,
            SessionAction::Focus,
            SessionAction::Pause,
            SessionAction::Resume,
            SessionAction::Clone,
            SessionAction::Rename,
            SessionAction::Attach,
            SessionAction::Spawn,
            SessionAction::Sync,
            SessionAction::Init,
        ];
        for i in 0..actions.len() {
            for j in (i + 1)..actions.len() {
                assert_ne!(
                    actions[i], actions[j],
                    "SessionAction variants must be distinct"
                );
            }
        }
    }

    #[test]
    fn session_action_debug() {
        let debug = format!("{:?}", SessionAction::Spawn);
        assert!(debug.contains("Spawn"));
    }

    // ---- StatusAction ----

    #[test]
    fn status_action_equality() {
        assert_eq!(StatusAction::Show, StatusAction::Show);
        assert_eq!(StatusAction::Whereami, StatusAction::Whereami);
        assert_ne!(StatusAction::Show, StatusAction::Whoami);
    }

    // ---- ConfigAction ----

    #[test]
    fn config_action_equality() {
        assert_eq!(ConfigAction::List, ConfigAction::List);
        assert_eq!(ConfigAction::Get, ConfigAction::Get);
        assert_ne!(ConfigAction::Set, ConfigAction::Schema);
    }

    // ---- DoctorAction ----

    #[test]
    fn doctor_action_equality() {
        assert_eq!(DoctorAction::Check, DoctorAction::Check);
        assert_eq!(DoctorAction::Fix, DoctorAction::Fix);
        assert_ne!(DoctorAction::Check, DoctorAction::Integrity);
    }

    // ---- GlobalFlags ----

    #[test]
    fn global_flags_default_all_false() {
        let flags = GlobalFlags::default();
        assert!(!flags.json);
        assert!(!flags.verbose);
        assert!(!flags.dry_run);
    }

    #[test]
    fn global_flags_with_json() {
        let flags = GlobalFlags {
            json: true,
            ..Default::default()
        };
        assert!(flags.json);
        assert!(!flags.verbose);
    }

    #[test]
    fn global_flags_with_all_enabled() {
        let flags = GlobalFlags {
            json: true,
            verbose: true,
            dry_run: true,
        };
        assert!(flags.json);
        assert!(flags.verbose);
        assert!(flags.dry_run);
    }

    #[test]
    fn global_flags_clone() {
        let flags = GlobalFlags {
            json: true,
            verbose: false,
            dry_run: true,
        };
        let cloned = flags.clone();
        assert_eq!(flags.json, cloned.json);
        assert_eq!(flags.verbose, cloned.verbose);
        assert_eq!(flags.dry_run, cloned.dry_run);
    }

    #[test]
    fn global_flags_debug() {
        let flags = GlobalFlags {
            json: true,
            verbose: false,
            dry_run: false,
        };
        let debug = format!("{:?}", flags);
        assert!(debug.contains("json"));
    }
}
