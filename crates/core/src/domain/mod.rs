//! # Domain Layer
//!
//! This module implements the **domain layer** following Domain-Driven Design (DDD) principles.
//! It contains the core business logic and domain models, independent of infrastructure concerns.
//!
//! ## Architecture
//!
//! The domain layer follows the **Functional Core, Imperative Shell** pattern:
//!
//! - **Pure functions** - No I/O, no global state, deterministic
//! - **Domain types** - Semantic newtypes that prevent invalid states
//! - **Business rules** - Encapsulated in aggregate roots
//! - **Error handling** - Clear error taxonomy using `thiserror`
//!
//! ## Module Structure
//!
//! ### Core Types (`identifiers`)
//!
//! Semantic newtypes for domain identifiers:
//! - [`SessionName`] - Human-readable session names
//! - [`AgentId`] - Agent identifiers
//! - [`WorkspaceName`] - Workspace names
//! - [`TaskId`] / [`BeadId`] - Task identifiers with `bd-` prefix
//! - [`SessionId`] - Unique session identifiers
//! - [`AbsolutePath`] - Validated absolute filesystem paths
//!
//! Each identifier type:
//! - Validates input on construction (parse-once pattern)
//! - Cannot represent invalid states
//! - Implements `serde` serialization with validation
//! - Provides safe access to underlying values
//!
//! ### Aggregate Roots (`aggregates`)
//!
//! Aggregates are consistency boundaries that encapsulate business logic:
//! - [`Session`] - Development session with branch and parent hierarchy
//! - [`Workspace`] - Workspace lifecycle management
//! - [`Bead`] - Issue/task with state transitions
//!
//! ### Domain Events (`events`)
//!
//! Domain events represent important business events that have occurred:
//! - [`SessionCreatedEvent`] - A new session was created
//! - [`SessionCompletedEvent`] - A session was completed successfully
//! - [`SessionFailedEvent`] - A session failed
//! - [`WorkspaceCreatedEvent`] / [`WorkspaceRemovedEvent`] - Workspace lifecycle
//! - [`BeadCreatedEvent`] / [`BeadClosedEvent`] - Bead lifecycle
//!
//! Events are:
//! - **Immutable** - Cannot be modified after creation
//! - **Serializable** - Can be persisted and transmitted
//! - **Timestamped** - Include when they occurred
//!
//! ### Repository Traits (`repository`)
//!
//! Repository pattern for persistence abstraction:
//! - [`SessionRepository`] - Session CRUD operations
//! - [`WorkspaceRepository`] - Workspace management
//! - [`BeadRepository`] - Bead CRUD operations
//! - [`AgentRepository`] - Agent registration and heartbeat
//!
//! Traits enable:
//! - **Dependency injection** - Business logic depends on traits, not concrete implementations
//! - **Testing** - Mock implementations without real persistence
//! - **Swappable backends** - Switch between `SQLite`, `PostgreSQL`, in-memory, etc.
//!
//! ### Supporting Modules
//!
//! - **`agent`** - Agent domain model and state management
//! - **`session`** - Session domain types and state transitions
//! - **`workspace`** - Workspace domain types and lifecycle
//! - **`builders`** - Builder pattern implementations for aggregates
//! - **`macros`** - Domain-level procedural macros
//! - **`error_conversion`** - Error conversion traits for ergonomic error handling
//!
//! ## Design Principles
//!
//! ### Parse at Boundaries, Validate Once
//!
//! ```rust,ignore
//! use isolate_core::domain::{SessionName, SessionNameError};
//!
//! // Parse and validate at the boundary
//! let name = SessionName::parse("my-session")?;
//!
//! // Use throughout domain - no further validation needed
//! let session = Session::new(name.clone())?;
//! ```
//!
//! ### Make Illegal States Unrepresentable
//!
//! ```rust,ignore
//! // BAD: Using bools and Option allows invalid states
//! struct BadSession {
//!     is_active: bool,
//!     current_branch: Option<String>,  // Can be active with no branch
//! }
//!
//! // GOOD: Enum makes valid states explicit
//! enum BranchState {
//!     Detached,
//!     OnBranch { name: String },
//! }
//! ```
//!
//! ### Use Semantic Newtypes
//!
//! ```rust,ignore
//! // BAD: Raw primitives
//! fn create_session(name: &str, id: &str) -> Result<Session, Error>;
//!
//! // GOOD: Semantic types
//! fn create_session(name: &SessionName, id: &SessionId) -> Result<Session, Error>;
//! ```
//!
//! ## Error Handling
//!
//! The domain layer uses `thiserror` for clear error taxonomy:
//!
//! - **`IdentifierError`** - Identifier validation failures
//! - **`SessionError`** / **`WorkspaceError`** / **`BeadError`** - Aggregate-specific errors
//! - **`RepositoryError`** - Repository operation errors
//!
//! All domain errors are:
//! - **Expected** - Represent valid business scenarios (not exceptional)
//! - **Typed** - Can be matched and handled specifically
//! - **Descriptive** - Include context about what went wrong
//!
//! ## Related Modules
//!
//! - **`crate::coordination`** - Coordination layer for distributed operations
//! - **`crate::output`** - Output types for AI-first CLI
//! - **`crate::beads`** - Beads issue tracker implementation

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod agent;
pub mod aggregates;
pub mod builders;
pub mod error_conversion;
pub mod events;
pub mod identifiers;
pub mod macros;
pub mod repository;
pub mod session;
pub mod workspace;

pub use aggregates::{
    Bead, BeadError, BeadState, Session, SessionBuilder, SessionError, Workspace, WorkspaceBuilder,
    WorkspaceError,
};
// Re-export error conversion traits for ergonomic error handling
pub use error_conversion::{AggregateErrorExt, IdentifierErrorExt, IntoRepositoryError};
pub use events::{
    DomainEvent, EventMetadata, SessionCompletedEvent, SessionCreatedEvent, SessionFailedEvent,
    StoredEvent,
};
pub use identifiers::{
    AbsolutePath, AbsolutePathError, AgentId, AgentIdError, BeadId, BeadIdError, IdError,
    IdentifierError, SessionId, SessionIdError, SessionName, SessionNameError, TaskId, TaskIdError,
    WorkspaceName, WorkspaceNameError,
};
// Re-export repository traits for convenience
pub use repository::{
    AgentRepository, AgentState, BeadRepository, RepositoryError, RepositoryResult,
    SessionRepository, WorkspaceRepository,
};
pub use workspace::WorkspaceState;

// Include examples module for documentation and testing
#[cfg(test)]
pub mod macros_examples;
