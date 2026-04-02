//! Session command definitions
//!
//! Subcommand enum for session management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionCommands {
    /// List sessions
    List,

    /// Show session status
    Status,

    /// Focus (switch to) a session
    Focus {
        /// Session name
        name: String,
    },

    /// Submit session changes for review
    Submit {
        /// Session name (default: current)
        name: Option<String>,

        /// Automatically commit dirty changes
        #[arg(short, long)]
        auto_commit: bool,

        /// Custom commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Remove a session
    Remove {
        /// Session name
        name: String,

        /// Force removal (skip confirmation)
        #[arg(short, long)]
        force: bool,

        /// Merge changes to main before removing
        #[arg(short, long)]
        merge: bool,
    },

    /// Pause an active session
    Pause {
        /// Session name
        name: String,
    },

    /// Resume a paused session
    Resume {
        /// Session name
        name: String,
    },

    /// Clone a session
    Clone {
        /// Source session name
        source: String,

        /// Target session name for the clone
        target: String,

        /// Dry-run mode (show what would happen)
        #[arg(short, long)]
        dry_run: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Parser, Subcommand};

    /// Wrapper to parse SessionCommands directly via clap
    #[derive(Parser)]
    struct SessionParser {
        #[command(subcommand)]
        command: SessionCommands,
    }

    fn parse(args: &[&str]) -> SessionCommands {
        let full: Vec<&str> = std::iter::once("scp")
            .chain(args.iter().copied())
            .collect();
        let parsed = SessionParser::parse_from(full);
        parsed.command
    }

    // -- List / Status --
    #[test]
    fn list_requires_no_args() {
        assert!(matches!(parse(&["list"]), SessionCommands::List));
    }

    #[test]
    fn status_requires_no_args() {
        assert!(matches!(parse(&["status"]), SessionCommands::Status));
    }

    // -- Focus (required) --
    #[test]
    fn focus_parses_name() {
        match parse(&["focus", "my-session"]) {
            SessionCommands::Focus { name } => assert_eq!(name, "my-session"),
            other => panic!("Expected Focus, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn focus_requires_name() {
        let result = SessionParser::try_parse_from(["scp", "focus"]);
        assert!(result.is_err());
    }

    // -- Submit (optional name, bool flags, optional message) --
    #[test]
    fn submit_defaults() {
        match parse(&["submit"]) {
            SessionCommands::Submit {
                name,
                auto_commit,
                message,
            } => {
                assert_eq!(name, None);
                assert!(!auto_commit);
                assert_eq!(message, None);
            }
            other => panic!("Expected Submit, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn submit_with_all_flags() {
        match parse(&["submit", "--auto-commit", "-m", "done", "sess-name"]) {
            SessionCommands::Submit {
                name,
                auto_commit,
                message,
            } => {
                assert_eq!(name, Some("sess-name".to_string()));
                assert!(auto_commit);
                assert_eq!(message, Some("done".to_string()));
            }
            other => panic!("Expected Submit, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Remove (required name, bool flags) --
    #[test]
    fn remove_required_name() {
        let result = SessionParser::try_parse_from(["scp", "remove"]);
        assert!(result.is_err());
    }

    #[test]
    fn remove_defaults() {
        match parse(&["remove", "s1"]) {
            SessionCommands::Remove {
                name,
                force,
                merge,
            } => {
                assert_eq!(name, "s1");
                assert!(!force);
                assert!(!merge);
            }
            other => panic!("Expected Remove, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn remove_with_flags() {
        match parse(&["remove", "s1", "-f", "-m"]) {
            SessionCommands::Remove { force, merge, .. } => {
                assert!(force);
                assert!(merge);
            }
            other => panic!("Expected Remove, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Pause (required) --
    #[test]
    fn pause_parses() {
        match parse(&["pause", "s1"]) {
            SessionCommands::Pause { name } => assert_eq!(name, "s1"),
            other => panic!("Expected Pause, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn pause_requires_name() {
        let result = SessionParser::try_parse_from(["scp", "pause"]);
        assert!(result.is_err());
    }

    // -- Resume (required) --
    #[test]
    fn resume_parses() {
        match parse(&["resume", "s1"]) {
            SessionCommands::Resume { name } => assert_eq!(name, "s1"),
            other => panic!("Expected Resume, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn resume_requires_name() {
        let result = SessionParser::try_parse_from(["scp", "resume"]);
        assert!(result.is_err());
    }

    // -- Clone (two required + bool flag) --
    #[test]
    fn clone_parses() {
        match parse(&["clone", "src", "dst"]) {
            SessionCommands::Clone {
                source,
                target,
                dry_run,
            } => {
                assert_eq!(source, "src");
                assert_eq!(target, "dst");
                assert!(!dry_run);
            }
            other => panic!("Expected Clone, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn clone_with_dry_run() {
        match parse(&["clone", "src", "dst", "--dry-run"]) {
            SessionCommands::Clone { dry_run, .. } => assert!(dry_run),
            other => panic!("Expected Clone, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn clone_requires_source_and_target() {
        let result = SessionParser::try_parse_from(["scp", "clone", "only-src"]);
        assert!(result.is_err());
    }
}
