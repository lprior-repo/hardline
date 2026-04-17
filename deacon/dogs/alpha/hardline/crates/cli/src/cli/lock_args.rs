//! Lock management arguments

use clap::Subcommand;

/// Lock management commands
#[derive(Subcommand)]
pub enum LockCommands {
    /// Acquire a lock on a session
    Acquire {
        /// Session name to lock
        session: String,
        /// Agent ID acquiring the lock
        #[arg(short, long)]
        agent: String,
        /// Time-To-Live in seconds (default: 300)
        #[arg(short, long)]
        ttl: Option<u64>,
    },
    /// Release a lock on a session
    Release {
        /// Session name to unlock
        session: String,
        /// Agent ID releasing the lock
        #[arg(short, long)]
        agent: String,
    },
    /// Send a heartbeat to maintain a lock
    Heartbeat {
        /// Session name
        session: String,
        /// Agent ID sending the heartbeat
        #[arg(short, long)]
        agent: String,
    },
    /// Show the status of a lock
    Status {
        /// Session name to check
        session: String,
    },
    /// List all active locks
    List,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct LockParser {
        #[command(subcommand)]
        command: LockCommands,
    }

    fn parse(args: &[&str]) -> LockCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        LockParser::parse_from(full).command
    }

    // -- List --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), LockCommands::List));
    }

    // -- Acquire --
    #[test]
    fn acquire_required_session_and_agent() {
        let result = LockParser::try_parse_from(["scp", "acquire", "-a", "bot1"]);
        assert!(result.is_err(), "acquire requires session positional arg");
    }

    #[test]
    fn acquire_defaults() {
        match parse(&["acquire", "sess1", "-a", "bot1"]) {
            LockCommands::Acquire {
                session,
                agent,
                ttl,
            } => {
                assert_eq!(session, "sess1");
                assert_eq!(agent, "bot1");
                assert_eq!(ttl, None);
            }
            other => panic!("Expected Acquire, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn acquire_with_ttl() {
        match parse(&["acquire", "sess1", "-a", "bot1", "-t", "600"]) {
            LockCommands::Acquire { ttl, .. } => assert_eq!(ttl, Some(600)),
            other => panic!("Expected Acquire, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Release --
    #[test]
    fn release_parses() {
        match parse(&["release", "sess1", "-a", "bot1"]) {
            LockCommands::Release { session, agent } => {
                assert_eq!(session, "sess1");
                assert_eq!(agent, "bot1");
            }
            other => panic!("Expected Release, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Heartbeat --
    #[test]
    fn heartbeat_parses() {
        match parse(&["heartbeat", "sess1", "-a", "bot1"]) {
            LockCommands::Heartbeat { session, agent } => {
                assert_eq!(session, "sess1");
                assert_eq!(agent, "bot1");
            }
            other => panic!(
                "Expected Heartbeat, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    // -- Status --
    #[test]
    fn status_parses() {
        match parse(&["status", "sess1"]) {
            LockCommands::Status { session } => assert_eq!(session, "sess1"),
            other => panic!("Expected Status, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn status_requires_session() {
        let result = LockParser::try_parse_from(["scp", "status"]);
        assert!(result.is_err());
    }
}
