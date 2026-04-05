//! Isolate domain — types, events, and state machine for isolated workspace lifecycle.

pub mod events;
pub mod guard;
pub mod state_machine;
pub mod types;

pub use events::{EventContext, EventType, IsolateEvent};
pub use guard::{CommittedGuard, WorkspaceGuard};
pub use state_machine::WorkspaceStateMachine;
pub use types::{BeadId, BeadWorkspaceMapping, WorkspaceId, WorkspaceState};
