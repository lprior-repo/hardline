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

pub mod commands;
pub mod config;
pub mod doctor;
pub mod helpers;
pub mod legacy_commands;
pub mod legacy_commands_done;
pub mod legacy_commands_misc;
pub mod legacy_commands_status;
pub mod session;
pub mod status;
pub mod task;
pub mod types;

pub use commands::build_object_cli;
pub use legacy_commands::build_legacy_commands;
pub use session::cmd_session;
pub use status::cmd_status;
pub use task::cmd_task;
pub use types::{ConfigAction, DoctorAction, SessionAction, StatusAction, TaskAction, ZjjObject};

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
