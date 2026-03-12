#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod error;

// Re-export core types
pub use domain::entities::session::{Session, SessionId, SessionState, BranchState};
pub use domain::value_objects::{
    SessionName, WorkspaceId, BeadId, AgentId, WorkspaceName, TaskId, AbsolutePath,
    Title, Description, Labels, DependsOn, Priority, IssueType,
};
pub use domain::events::{SessionEvent, SessionCreatedEvent, SessionCompletedEvent, SessionFailedEvent};
pub use domain::workspace::{Workspace, WorkspaceId as WsId, WorkspaceName as WsName, WorkspacePath};
pub use domain::bead::{Bead, BeadId as BdId, BeadState, BeadTitle, BeadDescription, BeadType, Priority as BeadPriority};
pub use domain::workspace_state::{WorkspaceState, WorkspaceStateMachine};
pub use error::{SessionError, Result};
