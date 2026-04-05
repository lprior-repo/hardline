//! Isolate domain — fully isolated git-clone workspaces for AI agents.
//!
//! This crate provides the bounded context for workspace isolation:
//! each agent operates in its own complete git clone (not a worktree),
//! enabling true parallelism without shared state.
//!
//! # Architecture
//!
//! ```text
//! WorkspaceState: Created → Working → Ready → Merged
//!                                 ├→ Conflict → Working
//!                                 └→ Abandoned
//! ```
//!
//! # Design Principles
//!
//! - **Zero panic**: No unwrap/expect in source code (tests exempt)
//! - **Railway-oriented**: All operations return `Result<T, E>`
//! - **DDD**: Domain types with explicit invariants
//! - **Data->Calc->Actions**: Layered architecture separating pure logic from side effects

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

pub mod checkpoint;
pub mod dag;
pub mod domain;
pub mod error;

pub use checkpoint::{find_pending_restores, AutoCheckpoint, CheckpointGuard};
pub use dag::{BranchDag, BranchId, DagError};
pub use domain::{
    BeadId, BeadWorkspaceMapping, classify_command, CheckpointRecord, CheckpointState,
    CommittedGuard, EventContext, EventType, IsolateEvent, OperationRisk,
    WorkspaceGuard, WorkspaceId, WorkspaceState, WorkspaceStateMachine,
};
pub use error::{IsolateError, Result};
