#![allow(dead_code, clippy::missing_errors_doc)]
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
pub use domain::{
    bead::{
        Bead, BeadDescription, BeadId as BdId, BeadState, BeadTitle, BeadType,
        Priority as BeadPriority,
    },
    entities::session::{BranchState, Session, SessionId, SessionState},
    events::{SessionCompletedEvent, SessionCreatedEvent, SessionEvent, SessionFailedEvent},
    value_objects::{
        AbsolutePath, AbsolutePathError, AgentId, BeadId, DependsOn, Description, IssueType,
        Labels, PathValidationError, Priority, SessionName, ShellMetacharacterError, TaskId, Title,
        WorkspaceId, WorkspaceName,
    },
    workspace::{Workspace, WorkspaceId as WsId, WorkspaceName as WsName, WorkspacePath},
    workspace_state::{WorkspaceState, WorkspaceStateMachine},
};
pub use error::{Result, SessionError, TaskIdError};
// Re-export migration types
pub use infrastructure::migration::{
    get_migration_version, migrate_sessions_table, migrate_v2_add_branch_and_last_synced,
    rollback_v2_branch_and_last_synced, run_all_migrations, sessions_table_exists, MigrationError,
};
