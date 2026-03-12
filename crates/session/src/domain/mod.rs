pub mod bead;
pub mod entities;
pub mod events;
pub mod value_objects;
pub mod workspace;
pub mod workspace_state;

// Re-export value objects from value_objects module
pub use value_objects::{
    AbsolutePath, AgentId, DependsOn, Description, IssueType, Labels, Priority, SessionName,
    TaskId, Title, WorkspaceId as SessionWorkspaceId, WorkspaceName,
};

// Re-export new aggregates
pub use bead::{
    Bead, BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Priority as BeadPriority,
};
pub use workspace::{Workspace, WorkspaceId, WorkspaceName as WsName, WorkspacePath};

// Re-export state
pub use workspace_state::{WorkspaceState, WorkspaceStateMachine};

// Re-export entities and events
pub use entities::session::{BranchState, Session, SessionId, SessionState};
pub use events::{SessionCompletedEvent, SessionCreatedEvent, SessionEvent, SessionFailedEvent};
