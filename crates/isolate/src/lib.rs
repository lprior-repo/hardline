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

pub mod beads;
pub mod checkpoint;
pub mod command_context;
pub mod commands;
pub mod dag;
pub mod domain;
pub mod error;
pub mod hints;
pub mod hooks;
pub mod json;
pub mod session;

pub use beads::{BeadMetadata, BeadRepository, BeadStatus};
pub use checkpoint::{find_pending_restores, AutoCheckpoint, CheckpointGuard};
pub use command_context::{next_write_command_id, resolve_base_command_id, with_command_context};
pub use dag::{BranchDag, BranchId, DagError};
pub use domain::{
    classify_command, BeadId, BeadWorkspaceMapping, CheckpointRecord, CheckpointState,
    CommittedGuard, EventContext, EventType, IsolateEvent, OperationRisk, WorkspaceGuard,
    WorkspaceId, WorkspaceState, WorkspaceStateMachine,
};
pub use error::{IsolateError, Result};
pub use hints::{
    generate_hints, hints_for_error, next_actions_for_command, suggest_next_actions, ActionRisk,
    CommandContext, Hint, HintType, NextAction, SystemState, WorkspaceInfo,
};
pub use hooks::{with_hooks, HookResult, HooksConfig};
pub use session::{
    validate_session_name, validate_status_transition, Session, SessionStatus, SessionUpdate,
};
