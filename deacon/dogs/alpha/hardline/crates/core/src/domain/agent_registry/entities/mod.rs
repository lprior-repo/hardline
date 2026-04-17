//! Entity types for agent registry

pub mod agent;
pub mod heartbeat;

pub use agent::{Agent, AgentMetadata};
pub use heartbeat::{Heartbeat, HeartbeatConfig};
