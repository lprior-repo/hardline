//! Type definitions for VCS operations
//!
//! This module provides:
//! - `BackendType` - Enumeration distinguishing Git vs JJ repositories
//! - `RepositoryPath` - Absolute path to a version-controlled directory
//! - `BranchName` - Named reference to a line of development
//! - `CommitId` - Unique identifier for a commit
//! - `ChangeId` - Unique identifier for a VCS change/commit (Git SHA or JJ ID)

pub mod backend_type;
pub mod branch;
pub mod change_id;
pub mod commit;
pub mod repository;

pub use backend_type::BackendType;
pub use branch::BranchName;
pub use change_id::ChangeId;
pub use commit::CommitId;
pub use repository::RepositoryPath;
