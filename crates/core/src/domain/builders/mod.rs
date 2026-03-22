//! Type-safe builders for complex domain aggregates
//!
//! This module implements the builder pattern for domain types with:
//! - Compile-time enforcement of required fields
//! - Fluent API for optional fields
//! - Validation at build time
//! - Zero-unwrap, zero-panic
//!
//! # Design Principles
//!
//! 1. **Type-safe state machine**: Each builder state tracks which required fields have been set
//! 2. **Cannot build incomplete**: `build()` only available when all required fields are set
//! 3. **Zero-panic**: No `unwrap()`, `expect()`, or `panic!()`
//! 4. **Clear error messages**: Validation errors explain what's missing or invalid
//!
//! # Example
//!
//! ```rust,ignore
//! use isolate_core::{
//!     domain::builders::SessionOutputBuilder, types::SessionStatus, WorkspaceState,
//! };
//!
//! let session = SessionOutputBuilder::new()
//!     .name("my-session")?
//!     .status(SessionStatus::Active)
//!     .state(WorkspaceState::Working)
//!     .workspace_path("/path/to/workspace")?
//!     .build()?;
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod action;
pub mod agent_info;
pub mod conflict_detail;
pub mod errors;
pub mod issue;
pub mod plan;
pub mod session_output;
pub mod summary;
pub mod workspace_info;

// Re-export all public types for convenience
pub use action::ActionBuilder;
pub use agent_info::{AgentInfoBuilder, AgentState};
pub use conflict_detail::{ConflictDetailBuilder, ConflictType, ResolutionStrategy};
pub use errors::BuilderError;
pub use issue::{IssueBuilder, IssueKind};
pub use plan::PlanBuilder;
pub use session_output::SessionOutputBuilder;
pub use summary::SummaryBuilder;
pub use workspace_info::{WorkspaceInfoBuilder, WorkspaceInfoState};

#[cfg(test)]
mod tests;
