//! Domain models for the session crate.
//!
//! This module contains the core domain logic including:
//! - Bead aggregate (atomic units of work)
//! - Workspace management
//! - Session entities and events

pub mod bead;
pub mod bead_state;
#[cfg(test)]
pub mod bead_tests;
pub mod bead_types;
pub mod bead_value;
pub mod entities;
pub mod events;
pub mod value_objects;
pub mod workspace;
pub mod workspace_state;

// Re-export bead components
// (BeadState, BeadType, Priority re-exported via bead module below)

// Re-export value objects from value_objects module
// Re-export new aggregates
pub use bead::{
    Bead, BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Priority as BeadPriority,
};
pub use bead_types::Priority;
// Re-export entities and events
pub use entities::session::{BranchState, Session, SessionId, SessionState};
pub use events::{SessionCompletedEvent, SessionCreatedEvent, SessionEvent, SessionFailedEvent};
pub use value_objects::{
    AbsolutePath, AbsolutePathError, AgentId, DependsOn, Description, IssueType, Labels,
    PathValidationError, Priority as SessionPriority, SessionName, ShellMetacharacterError, TaskId,
    Title, WorkspaceId as SessionWorkspaceId, WorkspaceName,
};
pub use workspace::{Workspace, WorkspaceId, WorkspaceName as WsName, WorkspacePath};
// Re-export state
pub use workspace_state::{WorkspaceState, WorkspaceStateMachine};
