//! # Aggregate Roots
//!
//! This module contains **DDD aggregate roots**, which are consistency boundaries
//! for business logic. Each aggregate encapsulates domain rules and enforces invariants.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod bead;
pub mod session;
pub mod workspace;
pub mod workspace_builder;
pub mod workspace_error;
pub mod workspace_tests;

// Re-export aggregate types
pub use bead::{Bead, BeadError, BeadState};
pub use session::{Session, SessionBuilder, SessionError};
pub use workspace::{Workspace, WorkspaceError};
pub use workspace_builder::WorkspaceBuilder;
