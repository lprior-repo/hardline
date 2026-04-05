//! Isolate domain — types, events, state machine, and checkpoint logic.
//!
//! Follows Data->Calc->Actions layering:
//! - Data: Value types (WorkspaceState, OperationRisk, CheckpointState)
//! - Calc: Pure functions (classify_command, state transitions)
//! - Actions: Side effects (in parent module)

pub mod checkpoint_calc;
pub mod checkpoint_types;
pub mod events;
pub mod guard;
pub mod state_machine;
pub mod types;

pub use checkpoint_calc::classify_command;
pub use checkpoint_types::{CheckpointRecord, CheckpointState, OperationRisk};
pub use events::{EventContext, EventType, IsolateEvent};
pub use guard::{CommittedGuard, WorkspaceGuard};
pub use state_machine::WorkspaceStateMachine;
pub use types::{BeadId, BeadWorkspaceMapping, WorkspaceId, WorkspaceState};
