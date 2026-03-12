//! # Aggregate Roots
//!
//! This module contains **DDD aggregate roots**, which are consistency boundaries
//! for business logic. Each aggregate encapsulates domain rules and enforces invariants.
//!
//! ## What are Aggregates?
//!
//! In Domain-Driven Design, an aggregate is a cluster of domain objects that can be
//! treated as a unit. The aggregate root is the entry point for the aggregate.
//!
//! **Key characteristics:**
//! - **Consistency boundary** - All invariants are enforced within the aggregate
//! - **Transaction boundary** - All changes to an aggregate happen atomically
//! - **Encapsulation** - External code cannot directly modify internal state
//! - **Business logic** - Domain rules are implemented in aggregate methods
//!
//! ## Aggregates
//!
//! ### Session Aggregate
//!
//! [`Session`] - Development session with branch and parent hierarchy
//!
//! **Responsibilities:**
//! - Track active development session
//! - Manage branch state (detached or on branch)
//! - Track parent relationships (root or child)
//! - Store workspace path
//!
//! **Business rules:**
//! - A session cannot be both detached and on a branch
//! - A session cannot be both root and child
//! - Session names must be unique
//!
//! **Usage:**
//! ```rust,ignore
//! # use std::error::Error;
//! # use std::path::PathBuf;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use isolate_core::domain::aggregates::{Session, SessionBuilder};
//! use isolate_core::domain::identifiers::{SessionName, SessionId};
//! use isolate_core::domain::session::BranchState;
//!
//! let session = SessionBuilder::new()
//!     .id(SessionId::parse("session-123")?)
//!     .name(SessionName::parse("my-session")?)
//!     .branch(BranchState::Detached)
//!     .workspace_path(PathBuf::from("/path/to/workspace"))
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Workspace Aggregate
//!
//! [`Workspace`] - Workspace lifecycle management
//!
//! **Responsibilities:**
//! - Track workspace state (creating, ready, active, cleaning, removed)
//! - Manage workspace path
//! - Enforce state transition rules
//!
//! **Business rules:**
//! - Cannot activate a workspace that doesn't exist
//! - Cannot remove a workspace that is active
//! - State transitions follow: Creating → Ready → Active → Cleaning → Removed
//!
//! **Usage:**
//! ```rust,ignore
//! # use std::error::Error;
//! # use std::path::PathBuf;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use isolate_core::domain::aggregates::{Workspace, WorkspaceBuilder, WorkspaceState};
//! use isolate_core::domain::identifiers::WorkspaceName;
//!
//! let workspace = WorkspaceBuilder::new()
//!     .name(WorkspaceName::parse("my-workspace")?)
//!     .path(PathBuf::from("/path/to/workspace"))
//!     .state(WorkspaceState::Ready)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Bead Aggregate
//!
//! [`Bead`] - Issue/task with state transitions
//!
//! **Responsibilities:**
//! - Track bead state (open, in-progress, blocked, deferred, closed)
//! - Store bead title and description
//! - Enforce state transition rules
//! - Track timestamps for creation, updates, and closure
//!
//! **Business rules:**
//! - Cannot reopen a closed bead
//! - Must provide `closed_at` timestamp when closing
//! - State transitions are validated
//!
//! **Usage:**
//! ```rust,ignore
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use isolate_core::domain::aggregates::{Bead, BeadState};
//! use isolate_core::domain::identifiers::BeadId;
//! use chrono::Utc;
//!
//! let mut bead = Bead::new(
//!     BeadId::parse("bd-abc123")?,
//!     "Fix bug",
//! )?;
//!
//! bead.transition_to(BeadState::InProgress)?;
//! bead.close();
//! # Ok(())
//! # }
//! ```
//!
//! ## Design Principles
//!
//! ### 1. Encapsulation
//!
//! All state changes go through methods that enforce business rules:
//!
//! ```rust,ignore
//! // BAD: Direct field access allows invalid states
//! let mut session = Session { /* ... */ };
//! session.branch = BranchState::Detached;  // No validation!
//!
//! // GOOD: Methods enforce invariants
//! let session = session.transition_to_detached()?;
//! ```
//!
//! ### 2. Validation
//!
//! Business rules are enforced at the aggregate boundary:
//!
//! ```rust,ignore
//! impl Bead {
//!     pub fn transition_to(&mut self, new_state: BeadState) -> Result<(), BeadError> {
//!         // Validate state transition
//!         if self.state.is_closed() {
//!             return Err(BeadError::InvalidStateTransition {
//!                 from: self.state,
//!                 to: new_state,
//!             });
//!         }
//!         // ... transition logic
//!     }
//! }
//! ```
//!
//! ### 3. Immutability (where possible)
//!
//! State transitions return new instances for value types:
//!
//! ```rust,ignore
//! // Pure function - no mutation
//! let new_state = state.transition_to_next()?;
//!
//! // For complex aggregates, use builder pattern
//! let updated = session_builder.update().branch(new_branch).build()?;
//! ```
//!
//! ### 4. Error Handling
//!
//! All operations return `Result<T, E>` for proper error handling:
//!
//! ```rust,ignore
//! pub fn build(self) -> Result<Session, SessionError> {
//!     // Validate all fields
//!     if self.id.is_none() {
//!         return Err(SessionError::MissingId);
//!     }
//!     // ... more validation
//! }
//! ```
//!
//! ### 5. Type Safety
//!
//! Domain types prevent invalid states:
//!
//! ```rust,ignore
//! // BAD: Can represent invalid states
//! struct BadSession {
//!     is_detached: bool,
//!     branch_name: Option<String>,  // Can be detached AND have a branch
//! }
//!
//! // GOOD: Enum makes valid states explicit
//! enum BranchState {
//!     Detached,
//!     OnBranch { name: String },
//! }
//! ```
//!
//! ## Builder Pattern
//!
//! Complex aggregates use the builder pattern for construction:
//!
//! ```rust,ignore
//! use isolate_core::domain::aggregates::{Session, SessionBuilder};
//!
//! let session = SessionBuilder::new()
//!     .id(session_id)
//!     .name(session_name)
//!     .branch(BranchState::Detached)
//!     .workspace_path(PathBuf::from("/path/to/workspace"))
//!     .build()?;
//! ```
//!
//! Builders provide:
//! - **Fluent interface** - Chainable methods
//! - **Validation** - Errors at build time, not during use
//! - **Optional fields** - Sensible defaults for non-critical fields
//! - **Clear intent** - Readable construction code
//!
//! ## Error Types
//!
//! Each aggregate has its own error type:
//! - [`SessionError`] - Session operation errors
//! - [`WorkspaceError`] - Workspace operation errors
//! - [`BeadError`] - Bead operation errors
//!
//! Error types are:
//! - **Specific** - Clear error conditions
//! - **Actionable** - Include context for handling
//! - **Typed** - Can be matched and handled specifically
//!
//! ## Related Modules
//!
//! - **`crate::domain::identifiers`** - Semantic identifier types
//! - **`crate::domain::events`** - Domain events for aggregates
//! - **`crate::domain::repository`** - Repository traits for persistence
//! - **`crate::domain::builders`** - Builder implementations

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod bead;
pub mod session;
pub mod workspace;

// Re-export aggregate types
pub use bead::{Bead, BeadError, BeadState};
pub use session::{Session, SessionBuilder, SessionError};
pub use workspace::{Workspace, WorkspaceBuilder, WorkspaceError};
