//! Repository pattern trait interfaces for DDD persistence abstraction.
//!
//! # Repository Pattern
//!
//! The repository pattern abstracts data access behind interfaces, enabling:
//! - **Dependency injection**: Business logic depends on traits, not concrete implementations
//! - **Testing**: Mock implementations for unit tests without real persistence
//! - **Swappable backends**: Switch between `SQLite`, `PostgreSQL`, in-memory, etc.
//! - **Functional core**: Pure business logic independent of I/O
//!
//! # Architecture
//!
//! This module defines trait interfaces in the **domain layer** (core):
//! - Traits use domain types (`SessionId`, `SessionName`, etc.) not primitives
//! - Methods return `Result`s for proper error handling
//! - Clear documentation of error conditions
//! - No implementation details (`SQLite`, files, etc.) leak through
//!
//! Implementations live in the **infrastructure layer** (shell):
//! - `beads/db.rs` implements `BeadRepository` over `SQLite`
//! - Future: `PostgreSQL`, `Redis`, or in-memory implementations
//!
//! # Design Principles
//!
//! 1. **Domain types in signatures**: Use `SessionId` not `String`, `WorkspaceName` not `&str`
//! 2. **Result returns**: All methods return `Result<T, E>` for error handling
//! 3. **Collection semantics**: List methods return iterators for lazy evaluation
//! 4. **Clear errors**: Each trait documents its error conditions
//! 5. **Testability**: Traits can be mocked for unit testing business logic
//!
//! # Example
//!
//! ```rust,ignore
//! use isolate_core::domain::repository::{SessionRepository, RepositoryError};
//! use isolate_core::domain::SessionName;
//!
//! // Business logic depends on trait (dependency injection)
//! fn get_active_sessions(repo: &dyn SessionRepository) -> Result<Vec<Session>, RepositoryError> {
//!     let all = repo.list_all()?;
//!     Ok(all.into_iter()
//!         .filter(|s| s.is_active())
//!         .collect())
//! }
//!
//! // Test with mock
//! struct MockSessionRepo { sessions: Vec<Session> }
//! impl SessionRepository for MockSessionRepo { /* ... */ }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

mod agent;
mod bead;
mod error;
mod in_memory;
mod session;
mod workspace;

// Re-exports
pub use agent::{Agent, AgentRepository, AgentState};
pub use bead::{Bead, BeadRepository, BeadState};
pub use error::{RepositoryError, RepositoryResult};
pub use in_memory::InMemorySessionRepository;
pub use session::{Session, SessionRepository};
pub use workspace::{Workspace, WorkspaceRepository, WorkspaceState};
