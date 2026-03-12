#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod error;

pub use domain::entities::{Workspace, WorkspaceId, WorkspaceState};
pub use domain::value_objects::{WorkspaceName, WorkspacePath};
pub use domain::events::WorkspaceEvent;
pub use domain::state::WorkspaceStateMachine;
pub use application::workspace_service::WorkspaceService;
pub use infrastructure::workspace_repository::{WorkspaceRepository, InMemoryWorkspaceRepository};
pub use error::{WorkspaceError, Result};
