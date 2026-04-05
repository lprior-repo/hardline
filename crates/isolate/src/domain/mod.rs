//! Isolate domain — types and state machine for isolated workspace lifecycle.

pub mod state_machine;
pub mod types;

pub use state_machine::WorkspaceStateMachine;
pub use types::WorkspaceState;
