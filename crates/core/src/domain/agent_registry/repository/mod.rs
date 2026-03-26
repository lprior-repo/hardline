//! Agent repository modules.

pub mod in_memory;

/// Re-export the main trait.
pub use crate::domain::agent_registry::repository::AgentRegistryRepository;

/// Re-export the in-memory implementation.
pub use in_memory::InMemoryAgentRepository;
