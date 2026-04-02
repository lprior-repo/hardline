//! Task command definitions
//!
//! Subcommand enum for task (bead) management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum TaskCommands {
    /// List all tasks
    List,

    /// Show task details
    Show {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Claim a task (assign to self)
    Claim {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Yield a task (release assignment)
    Yield {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Start working on a task
    Start {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Complete a task
    Done {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TaskParser {
        #[command(subcommand)]
        command: TaskCommands,
    }

    fn parse(args: &[&str]) -> TaskCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        TaskParser::parse_from(full).command
    }

    // -- List --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), TaskCommands::List));
    }

    // -- Show (required task_id, default user) --
    #[test]
    fn show_default_user() {
        match parse(&["show", "hl-0g4"]) {
            TaskCommands::Show { task_id, user } => {
                assert_eq!(task_id, "hl-0g4");
                assert_eq!(user, "current-user");
            }
            other => panic!("Expected Show, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn show_custom_user() {
        match parse(&["show", "hl-0g4", "--user", "agent-bot"]) {
            TaskCommands::Show { user, .. } => assert_eq!(user, "agent-bot"),
            other => panic!("Expected Show, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn show_requires_task_id() {
        let result = TaskParser::try_parse_from(["scp", "show"]);
        assert!(result.is_err());
    }

    // -- Claim (required task_id, default user) --
    #[test]
    fn claim_default_user() {
        match parse(&["claim", "hl-0g4"]) {
            TaskCommands::Claim { task_id, user } => {
                assert_eq!(task_id, "hl-0g4");
                assert_eq!(user, "current-user");
            }
            other => panic!("Expected Claim, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn claim_custom_user() {
        match parse(&["claim", "hl-0g4", "--user", "agent-bot"]) {
            TaskCommands::Claim { user, .. } => assert_eq!(user, "agent-bot"),
            other => panic!("Expected Claim, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn claim_requires_task_id() {
        let result = TaskParser::try_parse_from(["scp", "claim"]);
        assert!(result.is_err());
    }

    // -- Yield (required task_id, default user) --
    #[test]
    fn yield_default_user() {
        match parse(&["yield", "hl-0g4"]) {
            TaskCommands::Yield { task_id, user } => {
                assert_eq!(task_id, "hl-0g4");
                assert_eq!(user, "current-user");
            }
            other => panic!("Expected Yield, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn yield_requires_task_id() {
        let result = TaskParser::try_parse_from(["scp", "yield"]);
        assert!(result.is_err());
    }

    // -- Start (required task_id, default user) --
    #[test]
    fn start_default_user() {
        match parse(&["start", "hl-0g4"]) {
            TaskCommands::Start { task_id, user } => {
                assert_eq!(task_id, "hl-0g4");
                assert_eq!(user, "current-user");
            }
            other => panic!("Expected Start, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn start_requires_task_id() {
        let result = TaskParser::try_parse_from(["scp", "start"]);
        assert!(result.is_err());
    }

    // -- Done (required task_id, default user) --
    #[test]
    fn done_default_user() {
        match parse(&["done", "hl-0g4"]) {
            TaskCommands::Done { task_id, user } => {
                assert_eq!(task_id, "hl-0g4");
                assert_eq!(user, "current-user");
            }
            other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn done_custom_user() {
        match parse(&["done", "hl-0g4", "--user", "agent-bot"]) {
            TaskCommands::Done { user, .. } => assert_eq!(user, "agent-bot"),
            other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn done_requires_task_id() {
        let result = TaskParser::try_parse_from(["scp", "done"]);
        assert!(result.is_err());
    }
}
