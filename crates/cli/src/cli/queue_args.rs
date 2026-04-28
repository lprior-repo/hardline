//! Queue command definitions
//!
//! Subcommand enum for queue management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum QueueCommands {
    /// List queue items
    List,

    /// Add item to queue
    Enqueue {
        /// Branch name
        branch: String,

        /// Priority (low/normal/high/critical)
        #[arg(short, long)]
        priority: Option<String>,
    },

    /// Remove front item from queue
    Dequeue,

    /// Process next item in queue
    Process {
        /// Run pre-flight checks
        #[arg(short, long)]
        checks: bool,
    },

    /// Insert item at position
    Insert {
        /// Position
        position: usize,

        /// Branch name
        branch: String,
    },

    /// Remove item from queue
    Remove {
        /// Branch name or ID
        branch: String,
    },

    /// Show queue status
    Status,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct QueueParser {
        #[command(subcommand)]
        command: QueueCommands,
    }

    fn parse(args: &[&str]) -> QueueCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        QueueParser::parse_from(full).command
    }

    // -- List / Dequeue / Status --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), QueueCommands::List));
    }

    #[test]
    fn dequeue_no_args() {
        assert!(matches!(parse(&["dequeue"]), QueueCommands::Dequeue));
    }

    #[test]
    fn status_no_args() {
        assert!(matches!(parse(&["status"]), QueueCommands::Status));
    }

    // -- Enqueue --
    #[test]
    fn enqueue_default_priority() {
        match parse(&["enqueue", "feature-branch"]) {
            QueueCommands::Enqueue { branch, priority } => {
                assert_eq!(branch, "feature-branch");
                assert_eq!(priority, None);
            }
            other => panic!("Expected Enqueue, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn enqueue_with_priority() {
        match parse(&["enqueue", "feature-branch", "-p", "high"]) {
            QueueCommands::Enqueue { priority, .. } => {
                assert_eq!(priority, Some("high".to_string()));
            }
            other => panic!("Expected Enqueue, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn enqueue_requires_branch() {
        let result = QueueParser::try_parse_from(["scp", "enqueue"]);
        assert!(result.is_err());
    }

    // -- Process --
    #[test]
    fn process_default_checks() {
        match parse(&["process"]) {
            QueueCommands::Process { checks } => assert!(!checks),
            other => panic!("Expected Process, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn process_with_checks() {
        match parse(&["process", "-c"]) {
            QueueCommands::Process { checks } => assert!(checks),
            other => panic!("Expected Process, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Insert --
    #[test]
    fn insert_parses() {
        match parse(&["insert", "3", "my-branch"]) {
            QueueCommands::Insert { position, branch } => {
                assert_eq!(position, 3);
                assert_eq!(branch, "my-branch");
            }
            other => panic!("Expected Insert, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn insert_requires_both_args() {
        let result = QueueParser::try_parse_from(["scp", "insert", "3"]);
        assert!(result.is_err());
    }

    // -- Remove --
    #[test]
    fn remove_parses() {
        match parse(&["remove", "my-branch"]) {
            QueueCommands::Remove { branch } => assert_eq!(branch, "my-branch"),
            other => panic!("Expected Remove, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn remove_requires_branch() {
        let result = QueueParser::try_parse_from(["scp", "remove"]);
        assert!(result.is_err());
    }
}
