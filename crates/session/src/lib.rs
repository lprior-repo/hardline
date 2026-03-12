#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

// Re-export core types
pub use domain::bead::{
    Bead, BeadDescription, BeadId as BdId, BeadState, BeadTitle, BeadType, Priority as BeadPriority,
};
pub use domain::entities::session::{BranchState, Session, SessionId, SessionState};
pub use domain::events::{
    SessionCompletedEvent, SessionCreatedEvent, SessionEvent, SessionFailedEvent,
};
pub use domain::value_objects::{
    AbsolutePath, AgentId, BeadId, DependsOn, Description, IssueType, Labels, Priority,
    SessionName, TaskId, Title, WorkspaceId, WorkspaceName,
};
pub use domain::workspace::{
    Workspace, WorkspaceId as WsId, WorkspaceName as WsName, WorkspacePath,
};
pub use domain::workspace_state::{WorkspaceState, WorkspaceStateMachine};
pub use error::{Result, SessionError};
