//! Agent command definitions
//!
//! Subcommand enum for agent management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Create an agent
    Create {
        /// Agent name
        name: String,
    },

    /// List agents
    List,

    /// Kill an agent
    Kill {
        /// Agent ID
        id: String,
    },

    /// Show agent status
    Status {
        /// Agent ID
        id: Option<String>,
    },

    /// Register current agent session
    Register {
        /// Session name to register for
        #[arg(long)]
        session: Option<String>,
    },

    /// Send agent heartbeat
    Heartbeat {
        /// Session name
        #[arg(long)]
        session: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct AgentParser {
        #[command(subcommand)]
        command: AgentCommands,
    }

    fn parse(args: &[&str]) -> AgentCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        AgentParser::parse_from(full).command
    }

    // -- List --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), AgentCommands::List));
    }

    // -- Create (required name) --
    #[test]
    fn create_parses() {
        match parse(&["create", "my-agent"]) {
            AgentCommands::Create { name } => assert_eq!(name, "my-agent"),
            other => panic!("Expected Create, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn create_requires_name() {
        let result = AgentParser::try_parse_from(["scp", "create"]);
        assert!(result.is_err());
    }

    // -- Kill (required id) --
    #[test]
    fn kill_parses() {
        match parse(&["kill", "agent-123"]) {
            AgentCommands::Kill { id } => assert_eq!(id, "agent-123"),
            other => panic!("Expected Kill, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn kill_requires_id() {
        let result = AgentParser::try_parse_from(["scp", "kill"]);
        assert!(result.is_err());
    }

    // -- Status (optional id) --
    #[test]
    fn status_default() {
        match parse(&["status"]) {
            AgentCommands::Status { id } => assert_eq!(id, None),
            other => panic!("Expected Status, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn status_with_id() {
        match parse(&["status", "agent-123"]) {
            AgentCommands::Status { id } => assert_eq!(id, Some("agent-123".to_string())),
            other => panic!("Expected Status, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Register (optional session) --
    #[test]
    fn register_default() {
        match parse(&["register"]) {
            AgentCommands::Register { session } => assert_eq!(session, None),
            other => panic!(
                "Expected Register, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn register_with_session() {
        match parse(&["register", "--session", "sess1"]) {
            AgentCommands::Register { session } => {
                assert_eq!(session, Some("sess1".to_string()));
            }
            other => panic!(
                "Expected Register, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    // -- Heartbeat (optional session) --
    #[test]
    fn heartbeat_default() {
        match parse(&["heartbeat"]) {
            AgentCommands::Heartbeat { session } => assert_eq!(session, None),
            other => panic!(
                "Expected Heartbeat, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn heartbeat_with_session() {
        match parse(&["heartbeat", "--session", "sess1"]) {
            AgentCommands::Heartbeat { session } => {
                assert_eq!(session, Some("sess1".to_string()));
            }
            other => panic!(
                "Expected Heartbeat, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }
}
