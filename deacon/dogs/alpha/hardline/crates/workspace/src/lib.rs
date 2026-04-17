#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use application::workspace_service::WorkspaceService;
pub use domain::entities::{Workspace, WorkspaceId, WorkspaceState};
pub use domain::events::WorkspaceEvent;
pub use domain::state::WorkspaceStateMachine;
pub use domain::value_objects::{WorkspaceName, WorkspacePath};
pub use error::{Result, WorkspaceError};
pub use infrastructure::workspace_repository::{InMemoryWorkspaceRepository, WorkspaceRepository};
