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
