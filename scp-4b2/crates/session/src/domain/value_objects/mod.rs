//! Value objects for the session domain.
//!
//! This module is split into multiple files to keep each under 300 lines:
//! - session.rs: SessionName, WorkspaceId, BeadId
//! - task.rs: AgentId, TaskId, Title, Description
//! - path.rs: AbsolutePath
//! - metadata.rs: Labels, DependsOn, Priority, IssueType, WorkspaceName

pub mod metadata;
pub mod path;
pub mod session;
pub mod task;

// Re-export all types for convenient access
pub use metadata::{DependsOn, IssueType, Labels, Priority, WorkspaceName};
pub use path::AbsolutePath;
pub use session::{BeadId, IdentifierError, SessionName, WorkspaceId};
pub use task::{AgentId, Description, TaskId, Title};
