//! AI-first introspection capabilities
//!
//! This module provides structured metadata about SCP capabilities,
//! enabling AI agents to discover features and understand system state.

pub mod command;
pub mod doctor;
pub mod query;
pub mod suggest;
pub mod tests;
pub mod types;

// Re-exports for backward compatibility
pub use command::{
    ArgumentSpec, CommandExample, CommandIntrospection, ErrorCondition, FlagSpec, Prerequisites,
};
pub use doctor::{
    CheckStatus, DoctorCheck, DoctorFixOutput, DoctorOutput, FixResult, UnfixableIssue,
};
pub use query::{
    Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
    SuggestNameQuery,
};
pub use suggest::suggest_name;
pub use types::{
    Capabilities, CapabilityCategory, DependencyInfo, IntrospectOutput, SystemState,
};
